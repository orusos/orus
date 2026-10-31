//! A catalog: `index.json`, the apps a source offers, with a detached signature in
//! `index.json.sig`. `build` makes one from bundles.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{archive, sign};

pub const SCHEMA: &str = "orus.catalog.v1";
pub const INDEX_FILE: &str = "index.json";
pub const SIGNATURE_FILE: &str = "index.json.sig";
pub const ARCHIVE_EXTENSION: &str = "orusapp";

/// One app version a catalog offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The app's `appUniversalName`.
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    /// What the app asks for, for display; the install review reads the archive's own
    /// manifest.
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Where to download the archive: a full URL, or a name next to `index.json`.
    pub url: String,
    /// Lowercase hex SHA-256 of the archive.
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalog {
    pub schema: String,
    pub name: String,
    pub apps: Vec<Entry>,
}

impl Catalog {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let catalog: Catalog =
            serde_json::from_slice(bytes).context("the catalog is not valid JSON")?;
        if catalog.schema != SCHEMA {
            bail!(
                "the catalog schema is {}, expected {SCHEMA}",
                catalog.schema
            );
        }
        Ok(catalog)
    }

    /// `id` at `version`, or its newest version when none is given.
    pub fn find(&self, id: &str, version: Option<&str>) -> Option<&Entry> {
        self.apps
            .iter()
            .filter(|e| e.id == id && version.is_none_or(|v| e.version == v))
            .max_by(|a, b| compare_versions(&a.version, &b.version))
    }
}

/// Compare dotted versions part by part: numbers by value, anything else as text, and a
/// number is older than text.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let mut left = a.split('.');
    let mut right = b.split('.');
    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (x, y) => {
                let (x, y) = (x.unwrap_or("0"), y.unwrap_or("0"));
                let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(x), Ok(y)) => x.cmp(&y),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => x.cmp(y),
                };
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Pack each bundle in `bundles` into `out` and write `index.json` (and its signature
/// when `key` is given) there.
pub fn build(
    bundles: &[PathBuf],
    out: &Path,
    name: &str,
    key: Option<&SigningKey>,
) -> Result<Catalog> {
    std::fs::create_dir_all(out)?;
    let mut apps = Vec::new();
    for bundle in bundles {
        let info =
            orus_core::installed_apps::app_info(bundle).map_err(|e| anyhow::anyhow!("{e}"))?;
        let bytes = archive::pack(bundle)?;
        let file = format!("{}-{}.{ARCHIVE_EXTENSION}", info.id, info.version);
        std::fs::write(out.join(&file), &bytes)?;
        apps.push(Entry {
            id: info.id,
            name: info.name,
            version: info.version,
            description: info.description,
            capabilities: info.capabilities,
            url: file,
            sha256: sha256_hex(&bytes),
            size: bytes.len() as u64,
        });
    }
    let catalog = Catalog {
        schema: SCHEMA.into(),
        name: name.into(),
        apps,
    };
    let index = serde_json::to_vec_pretty(&catalog)?;
    std::fs::write(out.join(INDEX_FILE), &index)?;
    let signature_path = out.join(SIGNATURE_FILE);
    match key {
        Some(key) => std::fs::write(&signature_path, sign::sign(key, &index))?,
        None => {
            let _ = std::fs::remove_file(&signature_path);
        }
    }
    Ok(catalog)
}
