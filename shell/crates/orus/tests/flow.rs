mod common;

use common::{Env, bundle, no_purge, yes};
use orus::catalog::{self, compare_versions};
use orus::serve;
use orus::sign;
use orus::sources::{self, Source};
use orus::store::{Review, Trust};

fn dev_source(name: &str, addr: std::net::SocketAddr) -> Source {
    Source {
        name: name.into(),
        url: format!("http://127.0.0.1:{}", addr.port()),
        key: None,
        development: true,
        enabled: true,
        system: false,
    }
}

/// A served catalog of `bundles` and an environment with it added as a development source.
fn served(env: &Env, bundles: &[std::path::PathBuf]) -> serve::Running {
    let out = env.path("catalog");
    catalog::build(bundles, &out, "Dev", None).unwrap();
    let running = serve::start(&out, "127.0.0.1:0").unwrap();
    sources::add(&env.store.paths, dev_source("dev", running.addr)).unwrap();
    running
}

#[test]
fn an_app_goes_from_a_dev_catalog_to_installed_and_back_to_removed() {
    let env = Env::new();
    let b = bundle(
        &env.path("src"),
        "hello",
        "1.0.0",
        &["media.camera.capture"],
    );
    let running = served(&env, &[b]);

    let report = env.store.update().unwrap();
    assert_eq!(report.len(), 1);
    assert_eq!(*report[0].1.as_ref().unwrap(), 1);

    let hits = env.store.search("hel").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].trust, Trust::Development);

    let mut seen = None;
    let info = env
        .store
        .install_hit(&env.store.resolve("hello").unwrap(), &mut |r: &Review| {
            seen = Some(r.clone());
            true
        })
        .unwrap();
    assert_eq!(
        (info.id.as_str(), info.version.as_str()),
        ("hello", "1.0.0")
    );
    let review = seen.unwrap();
    assert_eq!(review.trust, Trust::Development);
    assert_eq!(review.info.capabilities, ["media.camera.capture"]);
    assert!(review.installed.is_none());

    let installed = env.store.installed();
    assert_eq!(installed.len(), 1);
    assert!(
        env.store
            .paths
            .apps_dir
            .join("hello/assets/data.txt")
            .is_file()
    );

    assert!(env.store.remove("hello", &mut no_purge).unwrap().is_some());
    assert!(env.store.installed().is_empty());
    assert!(env.store.remove("hello", &mut no_purge).unwrap().is_none());
    running.stop();
}

#[test]
fn declining_the_review_installs_nothing() {
    let env = Env::new();
    let b = bundle(&env.path("src"), "hello", "1.0.0", &[]);
    let running = served(&env, &[b]);
    env.store.update().unwrap();
    let error = env
        .store
        .install_hit(&env.store.resolve("hello").unwrap(), &mut |_| false)
        .unwrap_err();
    assert!(
        error.to_string().contains("nothing was installed"),
        "{error}"
    );
    assert!(env.store.installed().is_empty());
    assert!(!env.store.paths.apps_dir.join("hello").exists());
    running.stop();
}

#[test]
fn a_changed_download_is_refused_before_anything_is_unpacked() {
    let env = Env::new();
    let b = bundle(&env.path("src"), "hello", "1.0.0", &[]);
    let running = served(&env, &[b]);
    env.store.update().unwrap();
    // Replace the archive after the catalog recorded its hash.
    let archive = env.path("catalog/hello-1.0.0.orusapp");
    let other = bundle(&env.path("other"), "hello", "1.0.0", &["host.shell"]);
    std::fs::write(&archive, orus::archive::pack(&other).unwrap()).unwrap();

    let error = env
        .store
        .install_hit(&env.store.resolve("hello").unwrap(), &mut yes)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("does not match its catalog entry"),
        "{error}"
    );
    assert!(env.store.installed().is_empty());
    running.stop();
}

#[test]
fn an_archive_that_is_not_the_cataloged_app_is_refused() {
    let env = Env::new();
    let b = bundle(&env.path("src"), "hello", "1.0.0", &[]);
    let out = env.path("catalog");
    let mut built = catalog::build(&[b], &out, "Dev", None).unwrap();
    // The catalog claims a version the archive does not hold, with a matching hash.
    built.apps[0].version = "9.9.9".into();
    std::fs::write(out.join("index.json"), serde_json::to_vec(&built).unwrap()).unwrap();
    let running = serve::start(&out, "127.0.0.1:0").unwrap();
    sources::add(&env.store.paths, dev_source("dev", running.addr)).unwrap();
    env.store.update().unwrap();

    let error = env
        .store
        .install_hit(&env.store.resolve("hello").unwrap(), &mut yes)
        .unwrap_err();
    assert!(error.to_string().contains("catalog says"), "{error}");
    assert!(env.store.installed().is_empty());
    running.stop();
}

#[test]
fn upgrade_installs_newer_versions_and_asks_only_about_new_capabilities() {
    let env = Env::new();
    let src = env.path("src");
    let v1 = bundle(&src, "hello", "1.0.0", &["media.audio.playback"]);
    let running = served(&env, &[v1]);
    env.store.update().unwrap();
    env.store
        .install_hit(&env.store.resolve("hello").unwrap(), &mut yes)
        .unwrap();

    // 1.1.0 asks for nothing new: installs without asking.
    let v11 = bundle(&src, "hello", "1.1.0", &["media.audio.playback"]);
    catalog::build(&[v11], &env.path("catalog"), "Dev", None).unwrap();
    env.store.update().unwrap();
    let mut asked = false;
    let done = env
        .store
        .upgrade(&mut |_| {
            asked = true;
            true
        })
        .unwrap();
    assert_eq!(
        done,
        [(
            "hello".to_string(),
            "1.0.0".to_string(),
            "1.1.0".to_string()
        )]
    );
    assert!(!asked);

    // 1.2.0 asks for the camera: declined, so 1.1.0 stays.
    let v12 = bundle(
        &src,
        "hello",
        "1.2.0",
        &["media.audio.playback", "media.camera.capture"],
    );
    catalog::build(&[v12], &env.path("catalog"), "Dev", None).unwrap();
    env.store.update().unwrap();
    let mut new_caps = Vec::new();
    let error = env
        .store
        .upgrade(&mut |r| {
            new_caps = r.new_capabilities().into_iter().cloned().collect();
            false
        })
        .unwrap_err();
    assert!(error.to_string().contains("cancelled"), "{error}");
    assert_eq!(new_caps, ["media.camera.capture"]);
    assert_eq!(env.store.installed_info("hello").unwrap().version, "1.1.0");

    // Approved, it installs; then there is nothing newer.
    let done = env.store.upgrade(&mut yes).unwrap();
    assert_eq!(
        done,
        [(
            "hello".to_string(),
            "1.1.0".to_string(),
            "1.2.0".to_string()
        )]
    );
    assert!(env.store.upgrade(&mut yes).unwrap().is_empty());
    running.stop();
}

#[test]
fn a_different_version_keeps_the_old_one_for_rollback_and_the_same_version_does_not() {
    let env = Env::new();
    let src = env.path("src");
    let v1 = bundle(&src, "hello", "1.0.0", &[]);
    let v2 = bundle(&src, "hello", "2.0.0", &[]);

    env.store.install_dir(&v1, &mut yes).unwrap();
    assert!(!env.store.paths.apps_dir.join(".previous/hello").exists());
    env.store.install_dir(&v1, &mut yes).unwrap();
    assert!(
        !env.store.paths.apps_dir.join(".previous/hello").exists(),
        "same version: no backup"
    );

    env.store.install_dir(&v2, &mut yes).unwrap();
    assert_eq!(env.store.installed_info("hello").unwrap().version, "2.0.0");
    assert!(env.store.paths.apps_dir.join(".previous/hello").is_dir());
    assert_eq!(
        env.store.installed().len(),
        1,
        "the earlier version is not listed"
    );

    assert_eq!(env.store.rollback("hello").unwrap(), "1.0.0");
    assert_eq!(
        env.store.rollback("hello").unwrap(),
        "2.0.0",
        "rolling back again swaps forward"
    );

    env.store.remove("hello", &mut no_purge).unwrap();
    assert!(!env.store.paths.apps_dir.join(".previous/hello").exists());
    assert!(env.store.rollback("hello").is_err());
}

#[test]
fn a_signed_catalog_checks_against_its_key_and_a_tampered_or_foreign_one_fails() {
    let env = Env::new();
    let b = bundle(&env.path("src"), "hello", "1.0.0", &[]);
    let (private, public) = sign::generate();
    let (_, other_public) = sign::generate();
    let (key_file, pub_file, other_pub_file) =
        (env.path("k.key"), env.path("k.pub"), env.path("o.pub"));
    std::fs::write(&key_file, private).unwrap();
    std::fs::write(&pub_file, public).unwrap();
    std::fs::write(&other_pub_file, other_public).unwrap();
    let out = env.path("catalog");
    catalog::build(
        &[b],
        &out,
        "Signed",
        Some(&sign::signing_key_from_file(&key_file).unwrap()),
    )
    .unwrap();
    let running = serve::start(&out, "127.0.0.1:0").unwrap();
    let source = |name: &str, key: &std::path::Path| Source {
        key: Some(key.to_path_buf()),
        ..dev_source(name, running.addr)
    };
    sources::add(&env.store.paths, source("good", &pub_file)).unwrap();
    sources::add(&env.store.paths, source("foreign", &other_pub_file)).unwrap();

    let report = env.store.update().unwrap();
    let result = |name: &str| {
        report
            .iter()
            .find(|(n, _)| n == name)
            .unwrap()
            .1
            .as_ref()
            .map(|c| *c)
    };
    assert_eq!(result("good").unwrap(), 1);
    assert!(
        result("foreign")
            .unwrap_err()
            .to_string()
            .contains("signature"),
        "wrong key"
    );
    assert_eq!(env.store.search("").unwrap()[0].trust, Trust::Verified);

    // Change one byte of the index: the signature no longer matches.
    let index = out.join("index.json");
    let mut text = std::fs::read_to_string(&index).unwrap();
    text = text.replace("Signed", "Signeq");
    std::fs::write(&index, text).unwrap();
    let report = env.store.update().unwrap();
    assert!(report.iter().find(|(n, _)| n == "good").unwrap().1.is_err());
    running.stop();
}

#[test]
fn an_archive_with_a_path_that_leaves_its_folder_is_refused() {
    let env = Env::new();
    let evil = env.path("evil.orusapp");
    {
        use std::io::Write;
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&evil).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("../escaped.txt", options).unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();
    }
    let error = env.store.install_file(&evil, &mut yes).unwrap_err();
    assert!(error.to_string().contains("unsafe path"), "{error}");
    assert!(!env.path("home/.orus/escaped.txt").exists());
    assert!(!env.path("escaped.txt").exists());
    assert!(env.store.installed().is_empty());
}

#[test]
fn a_bundle_that_fails_the_manifest_rules_is_not_installed() {
    let env = Env::new();
    let dir = env.path("bad");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "x").unwrap();
    std::fs::write(
        dir.join("waid-package.json"),
        r#"{"appUniversalName":"Bad Name"}"#,
    )
    .unwrap();
    assert!(env.store.install_dir(&dir, &mut yes).is_err());
    assert!(env.store.installed().is_empty());
}

#[test]
fn install_url_refuses_plain_http() {
    let env = Env::new();
    let error = env
        .store
        .install_url("http://example.com/a.orusapp", None, &mut yes)
        .unwrap_err();
    assert!(error.to_string().contains("not https"), "{error}");
}

#[test]
fn source_addresses_follow_the_https_and_development_rules() {
    use orus::sources::validate_url;
    assert!(validate_url("https://packages.kilne.dev", false).is_ok());
    assert!(validate_url("http://192.168.64.1:6678", true).is_ok());
    assert!(validate_url("http://localhost:6678", true).is_ok());
    assert!(validate_url("http://10.0.0.5", true).is_ok());
    assert!(
        validate_url("http://192.168.64.1:6678", false).is_err(),
        "http needs --development"
    );
    assert!(
        validate_url("http://example.com", true).is_err(),
        "not a private host"
    );
    assert!(validate_url("http://8.8.8.8", true).is_err());
    assert!(validate_url("ftp://example.com", false).is_err());
    assert!(validate_url("file:///etc", true).is_err());
}

#[test]
fn sources_are_added_removed_and_switched_and_system_ones_stay() {
    let env = Env::new();
    let paths = &env.store.paths;
    std::fs::write(
        paths.system_dir.join("orus.toml"),
        "[[source]]\nname = \"orus\"\nurl = \"https://packages.kilne.dev\"\nkey = \"/usr/share/orus/keyrings/orus.pub\"\n",
    )
    .unwrap();
    let list = sources::load(paths).unwrap();
    assert_eq!(list.len(), 1);
    assert!(list[0].system && list[0].enabled);

    let mk = |name: &str, url: &str, key: Option<&str>, dev: bool| Source {
        name: name.into(),
        url: url.into(),
        key: key.map(Into::into),
        development: dev,
        enabled: true,
        system: false,
    };
    assert!(
        sources::add(paths, mk("x", "https://x.example", None, false)).is_err(),
        "unsigned needs --development"
    );
    assert!(
        sources::add(paths, mk("x", "http://x.example", Some("k"), false)).is_err(),
        "http needs --development"
    );
    assert!(
        sources::add(paths, mk("orus", "https://y.example", Some("k"), false)).is_err(),
        "name is taken"
    );
    assert!(
        sources::add(
            paths,
            mk("bad name!", "https://y.example", Some("k"), false)
        )
        .is_err()
    );
    sources::add(
        paths,
        mk("mine", "https://mine.example", Some("/k.pub"), false),
    )
    .unwrap();
    sources::add(paths, mk("vm", "http://192.168.64.1:6678", None, true)).unwrap();
    assert_eq!(sources::load(paths).unwrap().len(), 3);

    assert!(
        sources::remove(paths, "orus").is_err(),
        "a system source cannot be removed"
    );
    sources::set_enabled(paths, "orus", false).unwrap();
    let list = sources::load(paths).unwrap();
    assert!(!list.iter().find(|s| s.name == "orus").unwrap().enabled);
    assert!(
        list.iter().find(|s| s.name == "orus").unwrap().system,
        "still the system's entry"
    );
    sources::set_enabled(paths, "orus", true).unwrap();
    sources::set_enabled(paths, "mine", false).unwrap();
    sources::remove(paths, "vm").unwrap();
    let names: Vec<_> = sources::load(paths)
        .unwrap()
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, ["orus", "mine"]);
    assert!(sources::remove(paths, "nope").is_err());
}

#[test]
fn versions_compare_by_number_not_by_text() {
    use std::cmp::Ordering::*;
    assert_eq!(compare_versions("1.10.0", "1.9.0"), Greater);
    assert_eq!(compare_versions("1.0", "1.0.0"), Equal);
    assert_eq!(compare_versions("2", "10"), Less);
    assert_eq!(compare_versions("1.0.0", "1.0.1"), Less);
    assert_eq!(
        compare_versions("1.0.0-beta", "1.0.0"),
        Greater,
        "text sorts after a number here"
    );
}

#[test]
fn removing_purges_the_runtime_state_first_and_a_failed_purge_keeps_the_app() {
    let env = Env::new();
    let b = bundle(&env.path("src"), "hello", "1.0.0", &[]);
    env.store.install_dir(&b, &mut yes).unwrap();

    let mut seen = Vec::new();
    let mut failing = |dir: &std::path::Path| -> anyhow::Result<String> {
        seen.push((dir.to_path_buf(), dir.join("waid-package.json").is_file()));
        anyhow::bail!("keychain locked")
    };
    let error = env.store.remove("hello", &mut failing).unwrap_err();
    assert!(error.to_string().contains("still installed"), "{error:#}");
    assert!(format!("{error:#}").contains("keychain locked"));
    assert_eq!(seen.len(), 1);
    assert!(seen[0].1, "the manifest is still there when the purge runs");
    assert_eq!(
        env.store.installed().len(),
        1,
        "a failed purge does not half-remove the app"
    );

    let mut works =
        |_: &std::path::Path| -> anyhow::Result<String> { Ok("removed 3 things\n".into()) };
    let report = env.store.remove("hello", &mut works).unwrap();
    assert_eq!(report.as_deref(), Some("removed 3 things\n"));
    assert!(env.store.installed().is_empty());
}

#[test]
fn upgrade_only_takes_versions_from_the_source_the_app_came_from() {
    let env = Env::new();
    let v1 = bundle(&env.path("src"), "hello", "1.0.0", &[]);
    let first = served(&env, &[v1]);
    env.store.update().unwrap();
    env.store
        .install_hit(&env.store.resolve("hello").unwrap(), &mut yes)
        .unwrap();

    // A second source offers a higher version of the same id.
    let v9 = bundle(&env.path("other-src"), "hello", "9.0.0", &[]);
    let out = env.path("other-catalog");
    catalog::build(&[v9], &out, "Other", None).unwrap();
    let second = serve::start(&out, "127.0.0.1:0").unwrap();
    sources::add(&env.store.paths, dev_source("other", second.addr)).unwrap();
    env.store.update().unwrap();

    assert!(env.store.upgrade(&mut yes).unwrap().is_empty());
    assert_eq!(env.store.installed_info("hello").unwrap().version, "1.0.0");

    // An app installed from a file has no source and is not upgraded either.
    let local = bundle(&env.path("local-src"), "hello", "1.0.0", &[]);
    env.store.install_dir(&local, &mut yes).unwrap();
    assert!(env.store.upgrade(&mut yes).unwrap().is_empty());
    first.stop();
    second.stop();
}

#[test]
fn the_store_and_settings_cannot_be_removed() {
    let env = Env::new();
    for id in ["orus-store", "orus-system-settings"] {
        let b = bundle(&env.path("src"), id, "1.0.0", &[]);
        env.store.install_dir(&b, &mut yes).unwrap();
        let mut purged = false;
        let mut purge = |_: &std::path::Path| -> anyhow::Result<String> {
            purged = true;
            Ok(String::new())
        };
        let error = env.store.remove(id, &mut purge).unwrap_err();
        assert!(error.to_string().contains("cannot be removed"), "{error}");
        assert!(!purged, "nothing of a system app is purged");
        assert!(env.store.installed_info(id).is_some());
    }
}
