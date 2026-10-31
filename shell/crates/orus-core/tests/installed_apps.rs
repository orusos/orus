use std::fs;
use std::path::Path;

use orus_core::installed_apps::{InstallError, install, list, remove};

fn bundle(dir: &Path, manifest: &str) {
    fs::create_dir_all(dir.join("assets")).unwrap();
    fs::write(dir.join("index.html"), "<html></html>").unwrap();
    fs::write(dir.join("assets").join("app.js"), "1").unwrap();
    fs::write(dir.join("waid-package.json"), manifest).unwrap();
}

const GOOD: &str =
    r#"{"productName":"Notes","appUniversalName":"notes","nativeUrlScheme":"dev.orus.notes"}"#;

#[test]
fn install_copies_the_bundle_under_its_universal_name_and_lists_it() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let apps = tmp.path().join("apps");
    bundle(&src, GOOD);

    let app = install(&src, &apps).unwrap();
    assert_eq!(app.name, "Notes");
    assert_eq!(app.dir, apps.join("notes"));
    assert!(apps.join("notes/assets/app.js").is_file());

    let listed = list(&apps);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "Notes");
}

#[test]
fn reinstall_replaces_the_previous_files() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let apps = tmp.path().join("apps");
    bundle(&src, GOOD);
    install(&src, &apps).unwrap();
    fs::write(apps.join("notes/stale.txt"), "x").unwrap();

    install(&src, &apps).unwrap();
    assert!(!apps.join("notes/stale.txt").exists());
    assert!(apps.join("notes/index.html").is_file());
}

#[test]
fn bundles_the_launcher_would_refuse_are_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let apps = tmp.path().join("apps");
    let cases = [
        (r#"{"appUniversalName":"notes"}"#, "nativeUrlScheme"),
        (
            r#"{"appUniversalName":"Notes","nativeUrlScheme":"dev.orus.notes"}"#,
            "appUniversalName",
        ),
        (
            r#"{"appUniversalName":"notes","nativeUrlScheme":"notes"}"#,
            "nativeUrlScheme",
        ),
        ("not json", "not valid JSON"),
    ];
    for (i, (manifest, expect)) in cases.iter().enumerate() {
        let src = tmp.path().join(format!("src{i}"));
        bundle(&src, manifest);
        match install(&src, &apps) {
            Err(InstallError::Invalid(message)) => {
                assert!(
                    message.contains(expect),
                    "{message} should mention {expect}"
                )
            }
            other => panic!("expected an invalid bundle, got {other:?}"),
        }
    }
    assert!(list(&apps).is_empty());
}

#[test]
fn a_directory_without_index_html_is_not_an_app() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("waid-package.json"), GOOD).unwrap();
    assert!(matches!(
        install(&src, &tmp.path().join("apps")),
        Err(InstallError::Invalid(_))
    ));
}

#[test]
fn remove_deletes_the_app_and_reports_missing_ones() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let apps = tmp.path().join("apps");
    bundle(&src, GOOD);
    install(&src, &apps).unwrap();

    assert!(remove(&apps, "notes").unwrap());
    assert!(!apps.join("notes").exists());
    assert!(!remove(&apps, "notes").unwrap());
    assert!(matches!(
        remove(&apps, "../etc"),
        Err(InstallError::Invalid(_))
    ));
}
