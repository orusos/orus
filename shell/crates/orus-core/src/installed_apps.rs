//! Web apps installed under the Orus OS apps directory.
//!
//! An installed app is a built web bundle: a directory with an `index.html`
//! and a `waid-package.json`. The manifest rules are the ones `waid-shell`
//! enforces for an app-shell launch (`waid_launch/discovery.rs`), so an app
//! that installs here can be started.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// An app found under the apps directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledApp {
    /// Directory holding `index.html` and `waid-package.json`.
    pub dir: PathBuf,
    /// Display name: `productName` (or `name`) from the manifest, else the
    /// directory name.
    pub name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

struct Manifest {
    name: String,
    universal_name: String,
}

/// What a bundle's manifest says about the app, for catalogs and install review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    /// `appUniversalName`: the folder name the app installs under.
    pub id: String,
    pub name: String,
    /// `version`, or "0" when the manifest has none.
    pub version: String,
    pub description: String,
    /// The `capabilities` the app asks for, sorted.
    pub capabilities: Vec<String>,
}

/// Read and check the manifest of the bundle in `dir`: the rules for installing.
pub fn app_info(dir: &Path) -> Result<AppInfo, InstallError> {
    let manifest = read_manifest(dir)?;
    let text = fs::read_to_string(dir.join("waid-package.json"))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|e| InstallError::Invalid(format!("waid-package.json is not valid JSON: {e}")))?;
    let mut capabilities: Vec<String> = value
        .get("capabilities")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    capabilities.sort();
    capabilities.dedup();
    Ok(AppInfo {
        id: manifest.universal_name,
        name: manifest.name,
        version: string_field(&value, &["version"]).unwrap_or_else(|| "0".to_string()),
        description: string_field(&value, &["description"]).unwrap_or_default(),
        capabilities,
    })
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn is_valid_universal_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_lowercase() || first.is_ascii_digit())
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

fn is_valid_reverse_dns_scheme(scheme: &str) -> bool {
    let starts_with_letter = scheme
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic());
    scheme.contains('.')
        && starts_with_letter
        && scheme
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '-' | '.'))
        && scheme.split('.').all(|part| {
            let edge_ok = |ch: Option<char>| {
                ch.is_some_and(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
            };
            edge_ok(part.chars().next()) && edge_ok(part.chars().last())
        })
}

fn read_manifest(dir: &Path) -> Result<Manifest, InstallError> {
    let invalid = |message: String| InstallError::Invalid(format!("{}: {message}", dir.display()));

    if !dir.join("index.html").is_file() {
        return Err(invalid("no index.html".into()));
    }
    let manifest_path = dir.join("waid-package.json");
    let text =
        fs::read_to_string(&manifest_path).map_err(|_| invalid("no waid-package.json".into()))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| invalid(format!("waid-package.json is not valid JSON: {error}")))?;

    let universal_name = string_field(
        &value,
        &["appUniversalName", "app_universal_name", "universalName"],
    )
    .ok_or_else(|| invalid("waid-package.json needs appUniversalName".into()))?;
    if !is_valid_universal_name(&universal_name) {
        return Err(invalid(format!(
            "appUniversalName must be lowercase letters, numbers or hyphens, got {universal_name:?}"
        )));
    }
    let scheme = string_field(&value, &["nativeUrlScheme", "native_url_scheme"])
        .ok_or_else(|| invalid("waid-package.json needs nativeUrlScheme".into()))?;
    if !is_valid_reverse_dns_scheme(&scheme) {
        return Err(invalid(format!(
            "nativeUrlScheme must be a reverse-DNS scheme such as dev.orus.settings, got {scheme:?}"
        )));
    }

    let name =
        string_field(&value, &["productName", "name"]).unwrap_or_else(|| universal_name.clone());
    Ok(Manifest {
        name,
        universal_name,
    })
}

/// The apps installed under `apps_dir`, sorted by name. Directories that are
/// not valid app bundles are skipped.
pub fn list(apps_dir: &Path) -> Vec<InstalledApp> {
    let Ok(entries) = fs::read_dir(apps_dir) else {
        return Vec::new();
    };
    let mut apps: Vec<InstalledApp> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|dir| {
            read_manifest(&dir).ok().map(|manifest| InstalledApp {
                dir,
                name: manifest.name,
            })
        })
        .collect();
    apps.sort_by_key(|app| app.name.to_lowercase());
    apps
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), InstallError> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &target)?;
        } else {
            return Err(InstallError::Invalid(format!(
                "{}: symlinks are not installed",
                entry.path().display()
            )));
        }
    }
    Ok(())
}

/// Install the bundle in `source` as `<apps_dir>/<appUniversalName>`,
/// replacing an app already installed under that name.
pub fn install(source: &Path, apps_dir: &Path) -> Result<InstalledApp, InstallError> {
    let manifest = read_manifest(source)?;
    let target = apps_dir.join(&manifest.universal_name);
    let staging = apps_dir.join(format!(".installing-{}", manifest.universal_name));

    fs::create_dir_all(apps_dir)?;
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    copy_dir(source, &staging).inspect_err(|_| {
        let _ = fs::remove_dir_all(&staging);
    })?;
    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    fs::rename(&staging, &target)?;

    Ok(InstalledApp {
        dir: target,
        name: manifest.name,
    })
}

/// Remove the app installed as `universal_name`. Returns whether it existed.
pub fn remove(apps_dir: &Path, universal_name: &str) -> Result<bool, InstallError> {
    if !is_valid_universal_name(universal_name) {
        return Err(InstallError::Invalid(format!(
            "not an app name: {universal_name:?}"
        )));
    }
    let target = apps_dir.join(universal_name);
    if !target.is_dir() {
        return Ok(false);
    }
    fs::remove_dir_all(&target)?;
    Ok(true)
}
