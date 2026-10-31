#![allow(dead_code)]

use std::path::{Path, PathBuf};

use orus::sources::Paths;
use orus::store::Store;

pub struct Env {
    pub root: tempfile::TempDir,
    pub store: Store,
}

impl Env {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let r = root.path();
        let paths = Paths {
            system_dir: r.join("etc-sources"),
            user_file: r.join("home/.orus/config/sources.toml"),
            cache_dir: r.join("home/.orus/cache/catalogs"),
            apps_dir: r.join("home/.orus/apps"),
        };
        std::fs::create_dir_all(&paths.system_dir).unwrap();
        Self {
            root,
            store: Store::new(paths),
        }
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.root.path().join(name)
    }
}

/// A built bundle in `dir/<id>-<version>` that passes the install checks.
pub fn bundle(dir: &Path, id: &str, version: &str, capabilities: &[&str]) -> PathBuf {
    let path = dir.join(format!("{id}-{version}"));
    std::fs::create_dir_all(path.join("assets")).unwrap();
    std::fs::write(
        path.join("index.html"),
        format!("<html><title>{id} {version}</title></html>"),
    )
    .unwrap();
    std::fs::write(path.join("assets/data.txt"), version).unwrap();
    let manifest = serde_json::json!({
        "appUniversalName": id,
        "nativeUrlScheme": format!("dev.test.{}", id.replace('-', "")),
        "productName": format!("App {id}"),
        "version": version,
        "description": format!("The {id} app"),
        "capabilities": capabilities,
    });
    std::fs::write(path.join("waid-package.json"), manifest.to_string()).unwrap();
    path
}

pub fn yes(_: &orus::store::Review) -> bool {
    true
}

/// A purge that does nothing, for tests of the files only.
pub fn no_purge(_: &Path) -> anyhow::Result<String> {
    Ok(String::new())
}
