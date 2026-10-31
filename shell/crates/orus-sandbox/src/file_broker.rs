//! Deny-by-default filesystem broker scoped to `~/.orus/`.
//!
//! Every file operation an app requests is routed through [`FileBroker`].
//! The broker canonicalises the target path, checks it against the allow
//! policy, and rejects anything outside the sandbox root.

use std::path::{Path, PathBuf};

use thiserror::Error;

/// Per-app storage quota in bytes (default 256 MiB).
pub const DEFAULT_QUOTA_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum FileBrokerError {
    #[error("path escapes sandbox root: {0}")]
    Escape(PathBuf),

    #[error("cross-app access denied: app `{app}` cannot access data of `{target}`")]
    CrossApp { app: String, target: String },

    #[error("write to read-only zone: {0}")]
    ReadOnly(PathBuf),

    #[error("quota exceeded for app `{0}`")]
    QuotaExceeded(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Filesystem broker enforcing deny-by-default with scoped access.
///
/// Directory structure under `root` (`~/.orus/`):
/// ```text
/// ~/.orus/
/// ├── apps/          # installed app bundles (read-only to apps)
/// ├── data/{app_id}/ # per-app mutable storage
/// ├── shared/        # host ↔ Orus OS exchange directory
/// ├── config/        # system configuration
/// └── downloads/     # CEF downloads
/// ```
#[derive(Debug)]
pub struct FileBroker {
    root: PathBuf,
    quotas: std::collections::HashMap<String, u64>,
}

impl FileBroker {
    /// Create a broker rooted at the given directory (typically `~/.orus/`).
    ///
    /// The root is canonicalized at construction time so that all subsequent
    /// path comparisons are consistent (important on macOS where `/var` →
    /// `/private/var`).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let raw: PathBuf = root.into();
        let canonical = std::fs::canonicalize(&raw).unwrap_or(raw);
        Self {
            root: canonical,
            quotas: std::collections::HashMap::new(),
        }
    }

    /// Set the storage quota for an app. Default is [`DEFAULT_QUOTA_BYTES`].
    pub fn set_quota(&mut self, app_id: &str, bytes: u64) {
        self.quotas.insert(app_id.to_owned(), bytes);
    }

    /// Check whether an app may **read** the given path.
    ///
    /// Allowed reads:
    /// - `~/.orus/apps/{any}/` — all app bundles are readable
    /// - `~/.orus/data/{app_id}/` — own data directory
    /// - `~/.orus/shared/` — host exchange directory
    /// - `~/.orus/config/` — system config (for sandbox-level reads)
    pub fn check_read(
        &self,
        app_id: &str,
        path: impl AsRef<Path>,
    ) -> Result<PathBuf, FileBrokerError> {
        let resolved = self.resolve(path.as_ref())?;
        self.assert_within_root(&resolved)?;

        // Per-app data isolation: only own subdirectory
        let data_dir = self.root.join("data");
        if resolved.starts_with(&data_dir) {
            let own_dir = data_dir.join(app_id);
            if !resolved.starts_with(&own_dir) {
                let target = resolved
                    .strip_prefix(&data_dir)
                    .ok()
                    .and_then(|p| p.components().next())
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .unwrap_or_default();
                return Err(FileBrokerError::CrossApp {
                    app: app_id.to_owned(),
                    target,
                });
            }
        }

        Ok(resolved)
    }

    /// Check whether an app may **write** to the given path.
    ///
    /// Allowed writes:
    /// - `~/.orus/data/{app_id}/` — own data directory only
    /// - `~/.orus/shared/` — host exchange directory
    ///
    /// Denied writes:
    /// - `~/.orus/apps/` — bundles are read-only
    /// - `~/.orus/config/` — system config is read-only to apps
    pub fn check_write(
        &self,
        app_id: &str,
        path: impl AsRef<Path>,
    ) -> Result<PathBuf, FileBrokerError> {
        let resolved = self.resolve(path.as_ref())?;
        self.assert_within_root(&resolved)?;

        // Apps dir is read-only
        let apps_dir = self.root.join("apps");
        if resolved.starts_with(&apps_dir) {
            return Err(FileBrokerError::ReadOnly(resolved));
        }

        // Config dir is read-only to apps
        let config_dir = self.root.join("config");
        if resolved.starts_with(&config_dir) {
            return Err(FileBrokerError::ReadOnly(resolved));
        }

        // Per-app data isolation
        let data_dir = self.root.join("data");
        if resolved.starts_with(&data_dir) {
            let own_dir = data_dir.join(app_id);
            if !resolved.starts_with(&own_dir) {
                let target = resolved
                    .strip_prefix(&data_dir)
                    .ok()
                    .and_then(|p| p.components().next())
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .unwrap_or_default();
                return Err(FileBrokerError::CrossApp {
                    app: app_id.to_owned(),
                    target,
                });
            }
        }

        // Shared dir is writable by all apps
        let shared_dir = self.root.join("shared");
        if resolved.starts_with(&shared_dir) {
            return Ok(resolved);
        }

        // Must be in own data dir or shared
        let own_data = self.root.join("data").join(app_id);
        if !resolved.starts_with(&own_data) && !resolved.starts_with(&shared_dir) {
            return Err(FileBrokerError::ReadOnly(resolved));
        }

        Ok(resolved)
    }

    /// Resolve a path to its canonical form without requiring the target to
    /// exist. Eliminates `.`, `..`, and redundant separators.
    fn resolve(&self, path: &Path) -> Result<PathBuf, FileBrokerError> {
        // If the path exists on disk, canonicalize resolves symlinks too.
        if path.exists() {
            return Ok(std::fs::canonicalize(path)?);
        }

        // For non-existent paths (e.g. write targets): find the longest
        // existing ancestor, canonicalize that, then append remaining
        // components with manual `..` resolution.
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };

        // Walk up until we find an existing ancestor to canonicalize
        let mut ancestor = abs.as_path();
        let mut tail_components = Vec::new();
        loop {
            if ancestor.exists() {
                break;
            }
            if let Some(name) = ancestor.file_name() {
                tail_components.push(name.to_os_string());
            }
            match ancestor.parent() {
                Some(p) => ancestor = p,
                None => break,
            }
        }

        let mut resolved = if ancestor.exists() {
            std::fs::canonicalize(ancestor)?
        } else {
            ancestor.to_path_buf()
        };

        // Append tail components in reverse (they were collected bottom-up)
        for comp in tail_components.into_iter().rev() {
            let s = comp.to_string_lossy();
            if s == ".." {
                resolved.pop();
            } else if s != "." {
                resolved.push(&comp);
            }
        }
        Ok(resolved)
    }

    /// Assert the resolved path is within the sandbox root.
    fn assert_within_root(&self, resolved: &Path) -> Result<(), FileBrokerError> {
        if !resolved.starts_with(&self.root) {
            return Err(FileBrokerError::Escape(resolved.to_path_buf()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, FileBroker) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".orus");
        // Create directory structure
        std::fs::create_dir_all(root.join("apps/hello")).unwrap();
        std::fs::create_dir_all(root.join("data/app-a")).unwrap();
        std::fs::create_dir_all(root.join("data/app-b")).unwrap();
        std::fs::create_dir_all(root.join("shared")).unwrap();
        std::fs::create_dir_all(root.join("config")).unwrap();
        std::fs::create_dir_all(root.join("downloads")).unwrap();
        // Write a file so canonicalize works
        std::fs::write(root.join("apps/hello/index.html"), "hello").unwrap();
        std::fs::write(root.join("data/app-a/notes.db"), "data").unwrap();
        std::fs::write(root.join("data/app-b/notes.db"), "data").unwrap();
        std::fs::write(root.join("shared/export.txt"), "shared").unwrap();
        std::fs::write(root.join("config/network.toml"), "").unwrap();
        let broker = FileBroker::new(&root);
        (tmp, broker)
    }

    #[test]
    fn read_own_app_bundle() {
        let (_tmp, broker) = setup();
        let root = &broker.root;
        assert!(
            broker
                .check_read("hello", root.join("apps/hello/index.html"))
                .is_ok()
        );
    }

    #[test]
    fn read_own_data() {
        let (_tmp, broker) = setup();
        let root = &broker.root;
        assert!(
            broker
                .check_read("app-a", root.join("data/app-a/notes.db"))
                .is_ok()
        );
    }

    #[test]
    fn deny_read_other_app_data() {
        let (_tmp, broker) = setup();
        let root = &broker.root;
        let err = broker
            .check_read("app-a", root.join("data/app-b/notes.db"))
            .unwrap_err();
        assert!(matches!(err, FileBrokerError::CrossApp { .. }));
    }

    #[test]
    fn deny_read_outside_root() {
        let (_tmp, broker) = setup();
        let err = broker.check_read("app-a", "/etc/passwd").unwrap_err();
        assert!(matches!(err, FileBrokerError::Escape(_)));
    }

    #[test]
    fn deny_path_traversal() {
        let (_tmp, broker) = setup();
        let root = &broker.root;
        let sneaky = root.join("apps/../../etc/passwd");
        let err = broker.check_read("app-a", &sneaky).unwrap_err();
        assert!(matches!(err, FileBrokerError::Escape(_)));
    }

    #[test]
    fn read_shared_dir() {
        let (_tmp, broker) = setup();
        let root = &broker.root;
        assert!(
            broker
                .check_read("app-a", root.join("shared/export.txt"))
                .is_ok()
        );
    }

    #[test]
    fn write_own_data() {
        let (_tmp, broker) = setup();
        let root = &broker.root;
        assert!(
            broker
                .check_write("app-a", root.join("data/app-a/notes.db"))
                .is_ok()
        );
    }

    #[test]
    fn deny_write_other_app_data() {
        let (_tmp, broker) = setup();
        let root = &broker.root;
        let err = broker
            .check_write("app-a", root.join("data/app-b/notes.db"))
            .unwrap_err();
        assert!(matches!(err, FileBrokerError::CrossApp { .. }));
    }

    #[test]
    fn deny_write_to_apps_dir() {
        let (_tmp, broker) = setup();
        let root = &broker.root;
        let err = broker
            .check_write("app-a", root.join("apps/hello/index.html"))
            .unwrap_err();
        assert!(matches!(err, FileBrokerError::ReadOnly(_)));
    }

    #[test]
    fn deny_write_to_config_dir() {
        let (_tmp, broker) = setup();
        let root = &broker.root;
        let err = broker
            .check_write("app-a", root.join("config/network.toml"))
            .unwrap_err();
        assert!(matches!(err, FileBrokerError::ReadOnly(_)));
    }

    #[test]
    fn write_shared_dir() {
        let (_tmp, broker) = setup();
        let root = &broker.root;
        assert!(
            broker
                .check_write("app-a", root.join("shared/export.txt"))
                .is_ok()
        );
    }

    #[test]
    fn deny_write_outside_root() {
        let (_tmp, broker) = setup();
        let err = broker.check_write("app-a", "/tmp/evil.txt").unwrap_err();
        assert!(matches!(err, FileBrokerError::Escape(_)));
    }

    #[test]
    fn write_nonexistent_path_in_own_data() {
        let (_tmp, broker) = setup();
        let root = &broker.root;
        // Path doesn't exist yet but is within allowed zone
        assert!(
            broker
                .check_write("app-a", root.join("data/app-a/new_file.db"))
                .is_ok()
        );
    }
}
