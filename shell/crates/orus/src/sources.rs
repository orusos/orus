//! Where apps come from: the sources the system ships and the user adds.
//!
//! System sources are `*.toml` files in `/etc/orus/sources.d/`; the user's are in
//! `~/.orus/config/sources.toml`. Both hold `[[source]]` tables. A user entry with the
//! name of a system source can only switch it on or off.

use std::net::IpAddr;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use url::{Host, Url};

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub name: String,
    pub url: String,
    /// Public key (a file of base64 text) the catalog's signature must match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<PathBuf>,
    /// A development source: plain `http` on a private address, no signature.
    #[serde(default, skip_serializing_if = "is_false")]
    pub development: bool,
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Provided by the system, so it cannot be removed.
    #[serde(skip)]
    pub system: bool,
}

fn is_false(v: &bool) -> bool {
    !*v
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SourceFile {
    #[serde(default, rename = "source")]
    sources: Vec<Source>,
}

/// Where the settings and caches live.
#[derive(Debug, Clone)]
pub struct Paths {
    pub system_dir: PathBuf,
    pub user_file: PathBuf,
    pub cache_dir: PathBuf,
    pub apps_dir: PathBuf,
}

impl Paths {
    /// The standard places under `home` (`ORUS_APPS_DIR` moves the apps).
    pub fn standard(home: &Path) -> Self {
        let orus = home.join(".orus");
        Self {
            system_dir: PathBuf::from("/etc/orus/sources.d"),
            user_file: orus.join("config").join("sources.toml"),
            cache_dir: orus.join("cache").join("catalogs"),
            apps_dir: std::env::var_os("ORUS_APPS_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| orus.join("apps")),
        }
    }
}

/// `url` as a source address. `https` always; `http` only for a development source on
/// this machine or a private network.
pub fn validate_url(url: &str, development: bool) -> Result<Url> {
    let parsed = Url::parse(url).with_context(|| format!("not a URL: {url}"))?;
    match parsed.scheme() {
        "https" => Ok(parsed),
        "http" if development && is_private_host(&parsed) => Ok(parsed),
        "http" if development => bail!(
            "a development source may use http only on this machine or a private network, not {}",
            parsed.host_str().unwrap_or("")
        ),
        "http" => bail!(
            "{url} is not https; add it as a development source if it is on a private network"
        ),
        other => bail!("{other}: addresses are not supported, use https"),
    }
}

fn is_private_host(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(name)) => name.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(ip)) => {
            IpAddr::V4(ip).is_loopback() || ip.is_private() || ip.is_link_local()
        }
        Some(Host::Ipv6(ip)) => ip.is_loopback() || (ip.segments()[0] & 0xfe00) == 0xfc00,
        None => false,
    }
}

fn read_file(path: &Path) -> Result<Vec<Source>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(toml::from_str::<SourceFile>(&text)
            .with_context(|| format!("{} is not valid", path.display()))?
            .sources),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e).with_context(|| format!("cannot read {}", path.display())),
    }
}

fn write_user(paths: &Paths, sources: &[Source]) -> Result<()> {
    if let Some(dir) = paths.user_file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = toml::to_string_pretty(&SourceFile {
        sources: sources.to_vec(),
    })?;
    std::fs::write(&paths.user_file, text)
        .with_context(|| format!("cannot write {}", paths.user_file.display()))
}

/// System sources, then the user's, with the user's on/off choices applied to the
/// system ones.
pub fn load(paths: &Paths) -> Result<Vec<Source>> {
    let mut all: Vec<Source> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&paths.system_dir) {
        let mut files: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        files.sort();
        for file in files {
            for mut source in read_file(&file)? {
                source.system = true;
                all.push(source);
            }
        }
    }
    for user in read_file(&paths.user_file)? {
        match all.iter_mut().find(|s| s.name == user.name) {
            Some(system) if system.system => system.enabled = user.enabled,
            Some(_) => bail!("two sources are named {}", user.name),
            None => all.push(user),
        }
    }
    Ok(all)
}

fn user_sources(paths: &Paths) -> Result<Vec<Source>> {
    read_file(&paths.user_file)
}

/// Add a source of the user's. A signed source needs `key`; only a development source
/// may go without one.
pub fn add(paths: &Paths, source: Source) -> Result<()> {
    validate_url(&source.url, source.development)?;
    if source.name.is_empty()
        || !source
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        bail!("a source name is letters, numbers, - and _");
    }
    if source.key.is_none() && !source.development {
        bail!("a source needs --key, or --development for an unsigned one on a private network");
    }
    if load(paths)?.iter().any(|s| s.name == source.name) {
        bail!("a source named {} already exists", source.name);
    }
    let mut mine = user_sources(paths)?;
    mine.push(Source {
        system: false,
        ..source
    });
    write_user(paths, &mine)
}

pub fn remove(paths: &Paths, name: &str) -> Result<()> {
    let all = load(paths)?;
    match all.iter().find(|s| s.name == name) {
        None => bail!("no source named {name}"),
        Some(s) if s.system => {
            bail!("{name} comes with the system; use `orus sources disable {name}`")
        }
        Some(_) => {}
    }
    let mine: Vec<Source> = user_sources(paths)?
        .into_iter()
        .filter(|s| s.name != name)
        .collect();
    write_user(paths, &mine)?;
    let _ = std::fs::remove_file(paths.cache_dir.join(format!("{name}.json")));
    Ok(())
}

pub fn set_enabled(paths: &Paths, name: &str, enabled: bool) -> Result<()> {
    let all = load(paths)?;
    let Some(source) = all.iter().find(|s| s.name == name) else {
        bail!("no source named {name}");
    };
    let mut mine = user_sources(paths)?;
    match mine.iter_mut().find(|s| s.name == name) {
        Some(entry) => entry.enabled = enabled,
        None => mine.push(Source {
            enabled,
            system: false,
            ..source.clone()
        }),
    }
    write_user(paths, &mine)
}
