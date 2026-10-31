use std::path::PathBuf;

/// Shared Orus OS data directory on the current host platform.
pub fn orus_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("Orus");
        }
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            return PathBuf::from(userprofile)
                .join("AppData")
                .join("Roaming")
                .join("Orus");
        }
    }

    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    home.join(".orus")
}

/// Legacy Orus OS data directory used before sandbox path unification.
pub fn legacy_orus_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("Orus");
        }
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            return PathBuf::from(userprofile)
                .join("AppData")
                .join("Roaming")
                .join("Orus");
        }
    }

    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    home.join(".orus")
}

/// Shared app bundle directory inside the Orus OS data dir.
pub fn apps_dir() -> PathBuf {
    let primary = orus_data_dir().join("apps");
    if primary.exists() {
        return primary;
    }

    let legacy = legacy_orus_data_dir().join("apps");
    if legacy.exists() {
        return legacy;
    }

    primary
}

/// Where shared images (wallpaper, app icons) are read from when `ORUS_ASSETS` is unset.
const DEFAULT_ASSETS_DIR: &str = "/usr/share/orus";

/// `relative` inside the shared assets directory, or `None` if there is no
/// such file (the caller then draws a plain fallback).
pub fn asset(relative: &str) -> Option<PathBuf> {
    let base = std::env::var_os("ORUS_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ASSETS_DIR));
    let path = base.join(relative);
    path.is_file().then_some(path)
}
