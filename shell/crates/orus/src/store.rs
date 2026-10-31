//! Installing, updating, rolling back and removing apps, from folders, archives, URLs
//! and the catalogs of the sources.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use orus_core::installed_apps::{self, AppInfo};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::catalog::{self, Catalog, Entry, compare_versions};
use crate::sources::{self, Paths, Source, validate_url};
use crate::{archive, fetch, sign};

/// How far an install is trusted, shown at the permissions screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Trust {
    /// From a source whose catalog signature checked out.
    Verified,
    /// From a development source: unsigned, on a private network.
    Development,
    /// A file or folder the user chose.
    Local,
    /// A URL with no catalog; the download's SHA-256 is what to compare.
    Url { sha256: String },
}

/// What an install is about to do, for the user to approve.
#[derive(Debug, Clone)]
pub struct Review {
    pub info: AppInfo,
    pub trust: Trust,
    /// Where it comes from: a source name, a path or a URL.
    pub from: String,
    /// The version already installed, with what it asked for.
    pub installed: Option<AppInfo>,
}

impl Review {
    /// The capabilities this install asks for that the installed version did not.
    pub fn new_capabilities(&self) -> Vec<&String> {
        self.info
            .capabilities
            .iter()
            .filter(|c| {
                self.installed
                    .as_ref()
                    .is_none_or(|old| !old.capabilities.contains(c))
            })
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Cached {
    trust: Trust,
    source_url: String,
    development: bool,
    catalog: Catalog,
}

/// An app a catalog offers.
#[derive(Debug, Clone)]
pub struct Hit {
    pub source: String,
    pub trust: Trust,
    pub entry: Entry,
    /// Where `entry.url` is relative to.
    base: Url,
    development: bool,
}

/// Apps that ship with Orus OS: the session installs them at every login, and the Store
/// and Settings are how a user manages everything else, so they cannot be removed.
pub const SYSTEM_APPS: &[&str] = &["orus-store", "orus-system-settings"];

pub fn is_system_app(id: &str) -> bool {
    SYSTEM_APPS.contains(&id)
}

pub struct Store {
    pub paths: Paths,
}

fn with_trailing_slash(url: &Url) -> Url {
    let mut base = url.clone();
    if !base.path().ends_with('/') {
        let path = format!("{}/", base.path());
        base.set_path(&path);
    }
    base
}

impl Store {
    pub fn new(paths: Paths) -> Self {
        Self { paths }
    }

    // ── Catalogs ───────────────────────────────────────────────────

    fn cache_file(&self, source: &str) -> PathBuf {
        self.paths.cache_dir.join(format!("{source}.json"))
    }

    /// Refresh every enabled source. Each result is the source's name and how many
    /// apps it offers, or why it could not be used.
    pub fn update(&self) -> Result<Vec<(String, Result<usize>)>> {
        let mut report = Vec::new();
        for source in sources::load(&self.paths)?
            .into_iter()
            .filter(|s| s.enabled)
        {
            let result = self.update_source(&source);
            report.push((source.name, result));
        }
        Ok(report)
    }

    fn update_source(&self, source: &Source) -> Result<usize> {
        let base = with_trailing_slash(&validate_url(&source.url, source.development)?);
        let index = fetch::get(&base.join(catalog::INDEX_FILE)?, fetch::MAX_CATALOG_BYTES)?;
        let trust = match &source.key {
            Some(key) => {
                let signature = fetch::get(&base.join(catalog::SIGNATURE_FILE)?, 4096)?;
                sign::verify(
                    &sign::verifying_key_from_file(key)?,
                    &index,
                    std::str::from_utf8(&signature).context("the signature is not text")?,
                )?;
                Trust::Verified
            }
            None if source.development => Trust::Development,
            None => bail!(
                "{} has no key, so its catalog cannot be trusted",
                source.name
            ),
        };
        let catalog = Catalog::parse(&index)?;
        let count = catalog.apps.len();
        std::fs::create_dir_all(&self.paths.cache_dir)?;
        let cached = Cached {
            trust,
            source_url: base.to_string(),
            development: source.development,
            catalog,
        };
        std::fs::write(
            self.cache_file(&source.name),
            serde_json::to_vec_pretty(&cached)?,
        )?;
        Ok(count)
    }

    /// Every app version the cached catalogs of enabled sources offer.
    pub fn offers(&self) -> Result<Vec<Hit>> {
        let mut hits = Vec::new();
        for source in sources::load(&self.paths)?
            .into_iter()
            .filter(|s| s.enabled)
        {
            let Ok(bytes) = std::fs::read(self.cache_file(&source.name)) else {
                continue;
            };
            let cached: Cached = serde_json::from_slice(&bytes).with_context(|| {
                format!(
                    "the cache for {} is damaged; run `orus update`",
                    source.name
                )
            })?;
            let base = Url::parse(&cached.source_url)?;
            for entry in cached.catalog.apps {
                hits.push(Hit {
                    source: source.name.clone(),
                    trust: cached.trust.clone(),
                    entry,
                    base: base.clone(),
                    development: cached.development,
                });
            }
        }
        Ok(hits)
    }

    /// Apps whose id, name or description contains `text`, newest version of each per
    /// source; everything when `text` is empty.
    pub fn search(&self, text: &str) -> Result<Vec<Hit>> {
        let needle = text.to_lowercase();
        let mut hits: Vec<Hit> = self
            .offers()?
            .into_iter()
            .filter(|h| {
                needle.is_empty()
                    || [&h.entry.id, &h.entry.name, &h.entry.description]
                        .iter()
                        .any(|f| f.to_lowercase().contains(&needle))
            })
            .collect();
        hits.sort_by(|a, b| {
            a.entry
                .id
                .cmp(&b.entry.id)
                .then_with(|| compare_versions(&b.entry.version, &a.entry.version))
        });
        hits.dedup_by(|b, a| a.entry.id == b.entry.id && a.source == b.source);
        Ok(hits)
    }

    /// `name` or `name@version` among the cached catalogs.
    pub fn resolve(&self, spec: &str) -> Result<Hit> {
        let (id, version) = match spec.split_once('@') {
            Some((id, version)) => (id, Some(version)),
            None => (spec, None),
        };
        self.offers()?
            .into_iter()
            .filter(|h| h.entry.id == id && version.is_none_or(|v| h.entry.version == v))
            .max_by(|a, b| compare_versions(&a.entry.version, &b.entry.version))
            .ok_or_else(|| anyhow::anyhow!("no app {spec} in the sources; try `orus update`"))
    }

    // ── Installing ─────────────────────────────────────────────────

    /// Install `hit`: download, check the SHA-256, unpack, and install if `approve`
    /// agrees to the review.
    pub fn install_hit(
        &self,
        hit: &Hit,
        approve: &mut dyn FnMut(&Review) -> bool,
    ) -> Result<AppInfo> {
        let url = hit.base.join(&hit.entry.url)?;
        validate_url(url.as_str(), hit.development)?;
        let bytes = fetch::get(&url, fetch::MAX_ARCHIVE_BYTES)?;
        let actual = catalog::sha256_hex(&bytes);
        if !actual.eq_ignore_ascii_case(&hit.entry.sha256) {
            bail!(
                "{} does not match its catalog entry (SHA-256 {actual}, expected {}); nothing was installed",
                hit.entry.id,
                hit.entry.sha256
            );
        }
        let info = self.finish(
            &bytes,
            Some(&hit.entry),
            hit.trust.clone(),
            &hit.source,
            approve,
        )?;
        self.set_origin(&info.id, Some(&hit.source))?;
        Ok(info)
    }

    /// The source an app was installed from; `upgrade` only takes new versions from it,
    /// so another source cannot replace an app by offering a higher version.
    fn origin_file(&self, id: &str) -> PathBuf {
        self.paths.apps_dir.join(".origins").join(id)
    }

    fn origin(&self, id: &str) -> Option<String> {
        std::fs::read_to_string(self.origin_file(id)).ok()
    }

    fn set_origin(&self, id: &str, source: Option<&str>) -> Result<()> {
        let file = self.origin_file(id);
        match source {
            Some(name) => {
                std::fs::create_dir_all(file.parent().expect("origin file has a parent"))?;
                std::fs::write(&file, name)?;
            }
            None => {
                let _ = std::fs::remove_file(&file);
            }
        }
        Ok(())
    }

    /// Install a `.orusapp` file the user chose.
    pub fn install_file(
        &self,
        path: &Path,
        approve: &mut dyn FnMut(&Review) -> bool,
    ) -> Result<AppInfo> {
        let bytes =
            std::fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
        self.finish(
            &bytes,
            None,
            Trust::Local,
            &path.display().to_string(),
            approve,
        )
    }

    /// Install the built bundle in `dir`.
    pub fn install_dir(
        &self,
        dir: &Path,
        approve: &mut dyn FnMut(&Review) -> bool,
    ) -> Result<AppInfo> {
        let bytes = archive::pack(dir)?;
        self.finish(
            &bytes,
            None,
            Trust::Local,
            &dir.display().to_string(),
            approve,
        )
    }

    /// Install an archive from `url` (https), optionally pinned to `sha256`.
    pub fn install_url(
        &self,
        url: &str,
        sha256: Option<&str>,
        approve: &mut dyn FnMut(&Review) -> bool,
    ) -> Result<AppInfo> {
        let parsed = validate_url(url, false)?;
        let bytes = fetch::get(&parsed, fetch::MAX_ARCHIVE_BYTES)?;
        let actual = catalog::sha256_hex(&bytes);
        if let Some(expected) = sha256
            && !actual.eq_ignore_ascii_case(expected)
        {
            bail!("the download's SHA-256 is {actual}, not {expected}; nothing was installed");
        }
        self.finish(&bytes, None, Trust::Url { sha256: actual }, url, approve)
    }

    fn staging_dir(&self) -> PathBuf {
        self.paths
            .apps_dir
            .join(format!(".staging-{}", std::process::id()))
    }

    fn previous_dir(&self) -> PathBuf {
        self.paths.apps_dir.join(".previous")
    }

    /// Unpack, check against the catalog entry, review, and move into place.
    fn finish(
        &self,
        archive_bytes: &[u8],
        expected: Option<&Entry>,
        trust: Trust,
        from: &str,
        approve: &mut dyn FnMut(&Review) -> bool,
    ) -> Result<AppInfo> {
        let staging = self.staging_dir();
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging)?;
        let result = self.finish_in(&staging, archive_bytes, expected, trust, from, approve);
        let _ = std::fs::remove_dir_all(&staging);
        if let Ok(info) = &result {
            self.set_origin(&info.id, None)?;
        }
        result
    }

    fn finish_in(
        &self,
        staging: &Path,
        archive_bytes: &[u8],
        expected: Option<&Entry>,
        trust: Trust,
        from: &str,
        approve: &mut dyn FnMut(&Review) -> bool,
    ) -> Result<AppInfo> {
        archive::unpack(archive_bytes, staging)?;
        let info = installed_apps::app_info(staging).map_err(|e| anyhow::anyhow!("{e}"))?;
        if let Some(entry) = expected
            && (info.id != entry.id || info.version != entry.version)
        {
            bail!(
                "the archive holds {} {} but the catalog says {} {}; nothing was installed",
                info.id,
                info.version,
                entry.id,
                entry.version
            );
        }
        let installed = self.installed_info(&info.id);
        let review = Review {
            info: info.clone(),
            trust,
            from: from.to_string(),
            installed,
        };
        if !approve(&review) {
            bail!("cancelled; nothing was installed");
        }
        self.put_in_place(staging, &info.id, review.installed.as_ref())?;
        Ok(info)
    }

    /// Move the unpacked app to its folder, keeping a different installed version as
    /// the one to roll back to. Installing the same version again replaces it.
    fn put_in_place(&self, staging: &Path, id: &str, installed: Option<&AppInfo>) -> Result<()> {
        let target = self.paths.apps_dir.join(id);
        let new_version = installed_apps::app_info(staging).map(|i| i.version).ok();
        if target.exists() {
            let different = installed.is_some_and(|old| Some(&old.version) != new_version.as_ref());
            if different {
                let previous = self.previous_dir();
                std::fs::create_dir_all(&previous)?;
                let slot = previous.join(id);
                let _ = std::fs::remove_dir_all(&slot);
                std::fs::rename(&target, &slot)?;
            } else {
                std::fs::remove_dir_all(&target)?;
            }
        }
        std::fs::rename(staging, &target)
            .with_context(|| format!("cannot move the app to {}", target.display()))?;
        Ok(())
    }

    // ── What is installed ──────────────────────────────────────────

    /// Whether `rollback` has an earlier version of `id` to go back to.
    pub fn has_previous(&self, id: &str) -> bool {
        self.previous_dir().join(id).is_dir()
    }

    pub fn installed_info(&self, id: &str) -> Option<AppInfo> {
        installed_apps::app_info(&self.paths.apps_dir.join(id)).ok()
    }

    pub fn installed(&self) -> Vec<AppInfo> {
        installed_apps::list(&self.paths.apps_dir)
            .into_iter()
            .filter_map(|app| installed_apps::app_info(&app.dir).ok())
            .collect()
    }

    /// Install the newest version of every installed app that the source it came from
    /// offers a newer one of (apps installed from a file or URL are not upgraded). `approve` is asked only when an upgrade asks for something new. Returns
    /// what was upgraded, as (id, old version, new version).
    pub fn upgrade(
        &self,
        approve: &mut dyn FnMut(&Review) -> bool,
    ) -> Result<Vec<(String, String, String)>> {
        let offers = self.offers()?;
        let mut done = Vec::new();
        for app in self.installed() {
            let origin = self.origin(&app.id);
            let newest = offers
                .iter()
                .filter(|h| h.entry.id == app.id && Some(&h.source) == origin.as_ref())
                .max_by(|a, b| compare_versions(&a.entry.version, &b.entry.version));
            let Some(hit) = newest
                .filter(|h| compare_versions(&h.entry.version, &app.version) == Ordering::Greater)
            else {
                continue;
            };
            let mut gate =
                |review: &Review| review.new_capabilities().is_empty() || approve(review);
            self.install_hit(hit, &mut gate)?;
            done.push((
                app.id.clone(),
                app.version.clone(),
                hit.entry.version.clone(),
            ));
        }
        Ok(done)
    }

    // ── Removing ───────────────────────────────────────────────────

    /// Swap the installed version with the one kept before the last upgrade. Returns the
    /// version now installed.
    pub fn rollback(&self, id: &str) -> Result<String> {
        let target = self.paths.apps_dir.join(id);
        let slot = self.previous_dir().join(id);
        if !slot.is_dir() {
            bail!("{id} has no earlier version to go back to");
        }
        let swap = self.previous_dir().join(format!(".swap-{id}"));
        let _ = std::fs::remove_dir_all(&swap);
        if target.exists() {
            std::fs::rename(&target, &swap)?;
        }
        std::fs::rename(&slot, &target)?;
        if swap.exists() {
            std::fs::rename(&swap, &slot)?;
        }
        Ok(self
            .installed_info(id)
            .map(|i| i.version)
            .unwrap_or_default())
    }

    /// Remove the app and the earlier version kept for rollback. `purge` runs first, on the
    /// installed folder, to remove what the runtime keeps for the app; if it fails the app
    /// stays installed. Returns `None` when the app was not installed, else what `purge`
    /// reported.
    pub fn remove(
        &self,
        id: &str,
        purge: &mut dyn FnMut(&Path) -> Result<String>,
    ) -> Result<Option<String>> {
        if is_system_app(id) {
            anyhow::bail!("{id} is part of Orus OS and cannot be removed");
        }
        let dir = self.paths.apps_dir.join(id);
        let report = if dir.is_dir() {
            Some(purge(&dir).with_context(|| {
                format!("could not remove the stored data of {id}; the app is still installed")
            })?)
        } else {
            None
        };
        installed_apps::remove(&self.paths.apps_dir, id).map_err(|e| anyhow::anyhow!("{e}"))?;
        let _ = std::fs::remove_dir_all(self.previous_dir().join(id));
        self.set_origin(id, None)?;
        Ok(report)
    }
}
