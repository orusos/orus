//! The embedded SVG icons the window draws, by name. The shell embeds the files
//! (`orus-shell/assets/icons/files-*.svg`); a test there checks every name here.

use orus_core::appearance::Theme;

use crate::layout::CellKind;
use crate::model::DirEntry;
use crate::places::PlaceKind;

pub const BACK: &str = "files-back.svg";
pub const FORWARD: &str = "files-forward.svg";
pub const SEARCH: &str = "files-search.svg";
pub const GRID: &str = "files-grid.svg";
pub const LIST: &str = "files-list.svg";
pub const PLUS: &str = "files-plus.svg";
pub const CLOSE: &str = "files-close.svg";
pub const HOME: &str = "files-home.svg";
pub const TAB_FOLDER: &str = "files-tab-folder.svg";

pub const ALL: &[&str] = &[
    BACK,
    FORWARD,
    SEARCH,
    GRID,
    LIST,
    PLUS,
    CLOSE,
    HOME,
    TAB_FOLDER,
    "files-documents.svg",
    "files-downloads.svg",
    "files-pictures.svg",
    "files-system.svg",
    "files-usb.svg",
    "files-folder-plain-dark.svg",
    "files-folder-plain-light.svg",
    "files-folder-doc-dark.svg",
    "files-folder-doc-light.svg",
    "files-folder-image-dark.svg",
    "files-folder-image-light.svg",
    "files-folder-download-dark.svg",
    "files-folder-download-light.svg",
    "files-file-dark.svg",
    "files-file-light.svg",
];

pub fn place_icon(kind: PlaceKind) -> &'static str {
    match kind {
        PlaceKind::Home => HOME,
        PlaceKind::Documents => "files-documents.svg",
        PlaceKind::Downloads => "files-downloads.svg",
        PlaceKind::Pictures => "files-pictures.svg",
        PlaceKind::System => "files-system.svg",
        PlaceKind::Removable => "files-usb.svg",
    }
}

/// The folder icon: a plain one, or one with an emblem when the name says what is in it.
pub fn folder_icon(name: &str, theme: Theme) -> &'static str {
    let dark = theme == Theme::Dark;
    match (name.to_lowercase().as_str(), dark) {
        ("documents" | "docs", true) => "files-folder-doc-dark.svg",
        ("documents" | "docs", false) => "files-folder-doc-light.svg",
        ("pictures" | "photos" | "wallpapers" | "images", true) => "files-folder-image-dark.svg",
        ("pictures" | "photos" | "wallpapers" | "images", false) => "files-folder-image-light.svg",
        ("downloads", true) => "files-folder-download-dark.svg",
        ("downloads", false) => "files-folder-download-light.svg",
        (_, true) => "files-folder-plain-dark.svg",
        (_, false) => "files-folder-plain-light.svg",
    }
}

pub fn file_icon(theme: Theme) -> &'static str {
    match theme {
        Theme::Dark => "files-file-dark.svg",
        Theme::Light => "files-file-light.svg",
    }
}

pub fn is_image(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, ext)| {
        ["png", "jpg", "jpeg", "gif", "webp"].contains(&ext.to_lowercase().as_str())
    })
}

pub fn cell_kind(entry: &DirEntry) -> CellKind {
    if entry.is_dir {
        CellKind::Folder
    } else if is_image(&entry.name) {
        CellKind::Thumbnail
    } else {
        CellKind::File
    }
}
