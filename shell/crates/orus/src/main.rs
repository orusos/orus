//! The `orus` command.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Result, bail};
use orus::catalog;
use orus::serve;
use orus::sign;
use orus::sources::{self, Paths, Source};
use orus::store::{Review, Store, Trust};

const USAGE: &str = "usage:
  orus install [--yes | --allow CAP,CAP] [--sha256 HASH] <folder | file.orusapp | https://URL | name[@version]>
  orus remove <name>              remove an app and its earlier version, keep its data
  orus purge <name>               remove an app and the data it stored
  orus open <name>                start an installed app in its own window
  orus rollback <name>            go back to the version before the last upgrade
  orus list [--json]              list installed apps
  orus search [--json] [text]     search the sources' catalogs
  orus update                     refresh the sources' catalogs
  orus upgrade [--yes]            install newer versions of installed apps
  orus sources list [--json]
  orus sources add [--development] [--key FILE] [--name NAME] <url>
  orus sources remove|enable|disable <name>
  orus catalog keygen <name>      write <name>.key (private) and <name>.pub
  orus catalog build [--name NAME] [--key FILE] [--out DIR] <bundle folder>...
  orus catalog serve [--port N] [--bind ADDR] <folder>";

/// Take `--flag` out of `args`, returning whether it was there.
fn flag(args: &mut Vec<String>, name: &str) -> bool {
    let before = args.len();
    args.retain(|a| a != name);
    args.len() != before
}

/// Take `--option VALUE` out of `args`.
fn option(args: &mut Vec<String>, name: &str) -> Result<Option<String>> {
    let Some(i) = args.iter().position(|a| a == name) else {
        return Ok(None);
    };
    if i + 1 >= args.len() {
        bail!("{name} needs a value");
    }
    let value = args.remove(i + 1);
    args.remove(i);
    Ok(Some(value))
}

fn describe(trust: &Trust) -> String {
    match trust {
        Trust::Verified => "signed catalog".into(),
        Trust::Development => "development source: not signed".into(),
        Trust::Local => "a file on this computer".into(),
        Trust::Url { sha256 } => format!("a URL with no catalog: unverified, SHA-256 {sha256}"),
    }
}

/// Show what an install does and ask, unless `yes`.
fn approver(yes: bool, allow: Option<Vec<String>>) -> impl FnMut(&Review) -> bool {
    move |review| {
        let info = &review.info;
        println!("{} {} ({})", info.name, info.version, info.id);
        println!("  from: {} ({})", review.from, describe(&review.trust));
        match &review.installed {
            Some(old) => println!("  replaces version {}", old.version),
            None => println!("  new app"),
        }
        if info.capabilities.is_empty() {
            println!("  asks for no capabilities");
        } else {
            println!("  asks for:");
            for capability in &info.capabilities {
                let new = review.new_capabilities().contains(&capability);
                println!(
                    "    {capability}{}",
                    if new && review.installed.is_some() {
                        "  (new)"
                    } else {
                        ""
                    }
                );
            }
        }
        if yes {
            return true;
        }
        if let Some(allowed) = &allow {
            // The store app showed the user these capabilities; refuse an archive that asks
            // for more than was shown.
            return info.capabilities.iter().all(|c| allowed.contains(c));
        }
        print!("Install? [y/N] ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).is_ok() && matches!(line.trim(), "y" | "Y" | "yes")
    }
}

/// Ask the runtime to remove what it stores for the app installed in `dir`.
fn purge_with_runtime(dir: &Path) -> Result<String> {
    let beside = std::env::current_exe()
        .ok()
        .map(|exe| exe.with_file_name("waid-shell"));
    let program = match beside {
        Some(path) if path.exists() => path,
        _ => PathBuf::from("waid-shell"),
    };
    let output = std::process::Command::new(&program)
        .arg("purge-app")
        .arg(dir)
        .output()
        .map_err(|e| anyhow::anyhow!("cannot run {} to remove the app's stored data ({e}); use `orus remove` to remove only the app", program.display()))?;
    if !output.status.success() {
        bail!(
            "{} purge-app failed: {}",
            program.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Start the installed app `id` in its own `waid-shell` process and return at once.
fn open_app(store: &Store, id: &str) -> Result<()> {
    let Some(info) = store.installed_info(id) else {
        bail!("{id} is not installed");
    };
    let beside = std::env::current_exe()
        .ok()
        .map(|exe| exe.with_file_name("waid-shell"));
    let program = match beside {
        Some(path) if path.exists() => path,
        _ => PathBuf::from("waid-shell"),
    };
    std::process::Command::new(&program)
        .env("DETIR_WAID_APP_PATH", store.paths.apps_dir.join(id))
        .env("DETIR_WINDOW_TITLE", &info.name)
        .env("DETIR_UI_SCALE", orus_core::design::SCALE.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| anyhow::anyhow!("cannot run {}: {e}", program.display()))?;
    Ok(())
}

fn is_url(text: &str) -> bool {
    text.starts_with("https://") || text.starts_with("http://")
}

fn install(store: &Store, mut args: Vec<String>) -> Result<()> {
    let yes = flag(&mut args, "--yes");
    let allow = option(&mut args, "--allow")?.map(|list| {
        list.split(',')
            .filter(|c| !c.is_empty())
            .map(String::from)
            .collect()
    });
    let sha256 = option(&mut args, "--sha256")?;
    let [target] = args.as_slice() else {
        bail!("install takes one folder, file, URL or name");
    };
    let mut approve = approver(yes, allow);
    let path = Path::new(target);
    let info = if path.is_dir() {
        store.install_dir(path, &mut approve)?
    } else if path.is_file() {
        store.install_file(path, &mut approve)?
    } else if is_url(target) {
        store.install_url(target, sha256.as_deref(), &mut approve)?
    } else {
        let hit = store.resolve(target)?;
        store.install_hit(&hit, &mut approve)?
    };
    println!("installed {} {}", info.name, info.version);
    Ok(())
}

fn sources_command(paths: &Paths, mut args: Vec<String>) -> Result<()> {
    let action = if args.is_empty() {
        String::new()
    } else {
        args.remove(0)
    };
    match action.as_str() {
        "list" | "" if flag(&mut args, "--json") => {
            let all: Vec<_> = sources::load(paths)?
                .into_iter()
                .map(|s| {
                    serde_json::json!({
                        "name": s.name, "url": s.url, "development": s.development,
                        "system": s.system, "enabled": s.enabled, "signed": s.key.is_some(),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string(&all)?);
        }
        "list" | "" => {
            for s in sources::load(paths)? {
                let kind = if s.development {
                    "development"
                } else if s.system {
                    "system"
                } else {
                    "user"
                };
                println!(
                    "{}\t{}\t{}\t{}",
                    s.name,
                    s.url,
                    kind,
                    if s.enabled { "on" } else { "off" }
                );
            }
        }
        "add" => {
            let development = flag(&mut args, "--development");
            let key = option(&mut args, "--key")?.map(PathBuf::from);
            let name = option(&mut args, "--name")?;
            let [url] = args.as_slice() else {
                bail!("sources add takes one URL");
            };
            let name = match name {
                Some(name) => name,
                None => url::Url::parse(url)?
                    .host_str()
                    .unwrap_or("source")
                    .replace('.', "-"),
            };
            sources::add(
                paths,
                Source {
                    name: name.clone(),
                    url: url.clone(),
                    key,
                    development,
                    enabled: true,
                    system: false,
                },
            )?;
            println!("added {name}; run `orus update` to read its catalog");
        }
        "remove" | "enable" | "disable" => {
            let [name] = args.as_slice() else {
                bail!("sources {action} takes a source name");
            };
            match action.as_str() {
                "remove" => sources::remove(paths, name)?,
                other => sources::set_enabled(paths, name, other == "enable")?,
            }
        }
        other => bail!("unknown sources command {other}"),
    }
    Ok(())
}

fn catalog_command(mut args: Vec<String>) -> Result<()> {
    let action = if args.is_empty() {
        String::new()
    } else {
        args.remove(0)
    };
    match action.as_str() {
        "keygen" => {
            let [name] = args.as_slice() else {
                bail!("catalog keygen takes a name for the key files");
            };
            let (private, public) = sign::generate();
            // Owner-only, and refuse to overwrite an existing key.
            let mut opts = std::fs::OpenOptions::new();
            opts.write(true).create_new(true);
            #[cfg(unix)]
            std::os::unix::fs::OpenOptionsExt::mode(&mut opts, 0o600);
            opts.open(format!("{name}.key"))?.write_all(private.as_ref())?;
            std::fs::write(format!("{name}.pub"), public)?;
            println!("wrote {name}.key (keep it secret) and {name}.pub");
        }
        "build" => {
            let name = option(&mut args, "--name")?.unwrap_or_else(|| "Development".into());
            let key = option(&mut args, "--key")?
                .map(|p| sign::signing_key_from_file(Path::new(&p)))
                .transpose()?;
            let out =
                PathBuf::from(option(&mut args, "--out")?.unwrap_or_else(|| "catalog".into()));
            let bundles: Vec<PathBuf> = args.iter().map(PathBuf::from).collect();
            if bundles.is_empty() {
                bail!("catalog build takes one or more bundle folders");
            }
            let built = catalog::build(&bundles, &out, &name, key.as_ref())?;
            println!(
                "wrote {} app(s) to {}{}",
                built.apps.len(),
                out.display(),
                if key.is_some() {
                    ", signed"
                } else {
                    ", unsigned"
                }
            );
        }
        "serve" => {
            let port: u16 = option(&mut args, "--port")?
                .map(|p| p.parse())
                .transpose()?
                .unwrap_or(serve::DEFAULT_PORT);
            let bind = option(&mut args, "--bind")?.unwrap_or_else(|| "0.0.0.0".into());
            let [dir] = args.as_slice() else {
                bail!("catalog serve takes one folder");
            };
            let running = serve::start(Path::new(dir), &format!("{bind}:{port}"))?;
            println!("serving {dir} on http://{}  (Ctrl+C to stop)", running.addr);
            running.wait();
        }
        other => bail!("unknown catalog command {other}"),
    }
    Ok(())
}

fn run(mut args: Vec<String>) -> Result<ExitCode> {
    if args.is_empty() {
        eprintln!("{USAGE}");
        return Ok(ExitCode::from(2));
    }
    let command = args.remove(0);
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let paths = Paths::standard(&home);
    let store = Store::new(paths.clone());
    match command.as_str() {
        "install" => install(&store, args)?,
        "remove" | "purge" => {
            let purge_data = command == "purge";
            let [name] = args.as_slice() else {
                bail!("{command} takes an app name");
            };
            let mut purge = |dir: &Path| -> Result<String> {
                if purge_data {
                    purge_with_runtime(dir)
                } else {
                    Ok(String::new())
                }
            };
            match store.remove(name, &mut purge)? {
                None => bail!("{name} is not installed"),
                Some(report) => {
                    print!("{report}");
                    println!(
                        "removed {name}{}",
                        if purge_data { " and its data" } else { "" }
                    );
                }
            }
        }
        "open" => {
            let [name] = args.as_slice() else {
                bail!("open takes an app name");
            };
            open_app(&store, name)?;
        }
        "rollback" => {
            let [name] = args.as_slice() else {
                bail!("rollback takes an app name");
            };
            println!("{name} is back at version {}", store.rollback(name)?);
        }
        "list" => {
            let apps = store.installed();
            if flag(&mut args, "--json") {
                let apps: Vec<_> = apps
                    .iter()
                    .map(|a| {
                        serde_json::json!({
                            "id": a.id, "name": a.name, "version": a.version,
                            "description": a.description, "capabilities": a.capabilities,
                            "canRollBack": store.has_previous(&a.id),
                            "system": orus::store::is_system_app(&a.id),
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string(&apps)?);
            } else {
                for app in apps {
                    println!("{}\t{}\t{}", app.id, app.version, app.name);
                }
            }
        }
        "search" if flag(&mut args, "--json") => {
            let hits: Vec<_> = store
                .search(&args.join(" "))?
                .into_iter()
                .map(|hit| {
                    serde_json::json!({
                        "id": hit.entry.id,
                        "name": hit.entry.name,
                        "version": hit.entry.version,
                        "description": hit.entry.description,
                        "capabilities": hit.entry.capabilities,
                        "size": hit.entry.size,
                        "source": hit.source,
                        "trust": describe(&hit.trust),
                        "unsigned": hit.trust != Trust::Verified,
                        "installed": store.installed_info(&hit.entry.id).map(|i| i.version),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string(&hits)?);
        }
        "search" => {
            for hit in store.search(&args.join(" "))? {
                let installed = store
                    .installed_info(&hit.entry.id)
                    .map(|i| format!(" (installed {})", i.version))
                    .unwrap_or_default();
                println!(
                    "{}\t{}\t{}{installed}",
                    hit.entry.id, hit.entry.version, hit.entry.name
                );
            }
        }
        "update" => {
            for (name, result) in store.update()? {
                match result {
                    Ok(count) => println!("{name}: {count} app(s)"),
                    Err(error) => eprintln!("{name}: {error:#}"),
                }
            }
        }
        "upgrade" => {
            let yes = flag(&mut args, "--yes");
            let done = store.upgrade(&mut approver(yes, None))?;
            for (id, old, new) in &done {
                println!("{id}: {old} -> {new}");
            }
            if done.is_empty() {
                println!("everything is up to date");
            }
        }
        "sources" => sources_command(&paths, args)?,
        "catalog" => catalog_command(args)?,
        _ => {
            eprintln!("{USAGE}");
            return Ok(ExitCode::from(2));
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("orus: {error:#}");
            ExitCode::FAILURE
        }
    }
}
