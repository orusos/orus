//! The sidebar's places: the home folder and its usual folders, and the devices.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceKind {
    Home,
    Documents,
    Downloads,
    Pictures,
    System,
    Removable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub label: String,
    pub path: PathBuf,
    pub kind: PlaceKind,
}

#[derive(Debug, Clone, Default)]
pub struct Places {
    pub favorites: Vec<Place>,
    pub devices: Vec<Place>,
}

/// Where removable media is mounted by the desktop's mount helpers.
const REMOVABLE_PREFIXES: [&str; 2] = ["/media/", "/run/media/"];

/// The mount points of removable media in `/proc/mounts` text. Spaces in a path
/// appear there as `\040`.
pub fn removable_mounts(mounts: &str) -> Vec<PathBuf> {
    mounts
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .map(|path| path.replace("\\040", " "))
        .filter(|path| REMOVABLE_PREFIXES.iter().any(|p| path.starts_with(p)))
        // `/run/media/<user>` itself is a directory, not a mount of media.
        .filter(|path| {
            path.trim_end_matches('/').matches('/').count() >= 3 || path.starts_with("/media/")
        })
        .map(PathBuf::from)
        .collect()
}

/// The places for `home`, with devices from `/proc/mounts` text `mounts`. The usual
/// folders appear only if they exist.
pub fn discover(home: &Path, mounts: &str) -> Places {
    let mut favorites = vec![Place {
        label: "Home".to_string(),
        path: home.to_path_buf(),
        kind: PlaceKind::Home,
    }];
    for (name, kind) in [
        ("Documents", PlaceKind::Documents),
        ("Downloads", PlaceKind::Downloads),
        ("Pictures", PlaceKind::Pictures),
    ] {
        let path = home.join(name);
        if path.is_dir() {
            favorites.push(Place {
                label: name.to_string(),
                path,
                kind,
            });
        }
    }

    let mut devices = vec![Place {
        label: "System".to_string(),
        path: PathBuf::from("/"),
        kind: PlaceKind::System,
    }];
    for path in removable_mounts(mounts) {
        let label = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "USB drive".to_string());
        devices.push(Place {
            label,
            path,
            kind: PlaceKind::Removable,
        });
    }
    Places { favorites, devices }
}
