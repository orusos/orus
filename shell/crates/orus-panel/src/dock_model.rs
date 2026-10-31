//! What the dock shows: its five fixed tiles, then one tile for every other app that
//! has a window open. An app's windows share its tile, whatever their number.

use orus_shell::dock_bar::{DockApp, DockIcon, DockTile};

use crate::toplevels::Toplevels;

/// The Settings web app's window title (its page `<title>`); web apps have no app id.
pub const SETTINGS_TITLE: &str = "Settings";

/// The Store web app's window title.
pub const STORE_TITLE: &str = "Store";

/// The dock tile an app belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// One of the five fixed tiles.
    Fixed(DockApp),
    /// Another app, named by `key`.
    Extra { key: String, icon: DockIcon },
}

/// The tile for a window with `app_id` and `title`.
pub fn classify(app_id: &str, title: &str) -> Kind {
    match app_id {
        "orus-files" => Kind::Fixed(DockApp::Files),
        "orus-terminal" => Kind::Fixed(DockApp::Terminal),
        "orus-textedit" => Kind::Extra {
            key: app_id.into(),
            icon: DockIcon::TextEdit,
        },
        "orus-viewer" => Kind::Extra {
            key: app_id.into(),
            icon: DockIcon::Viewer,
        },
        id if id.starts_with("chromium") => Kind::Fixed(DockApp::Browser),
        // Writer, Calc, Impress and the start center share one tile.
        id if id.starts_with("libreoffice") => Kind::Extra {
            key: "libreoffice".into(),
            icon: DockIcon::Office,
        },
        "" if title == SETTINGS_TITLE => Kind::Fixed(DockApp::Settings),
        "" if title == STORE_TITLE => Kind::Extra {
            key: format!("title:{title}"),
            icon: DockIcon::Store,
        },
        // A web app has no app id, so its title names it.
        "" => Kind::Extra {
            key: format!("title:{title}"),
            icon: DockIcon::App,
        },
        id => Kind::Extra {
            key: id.into(),
            icon: DockIcon::App,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockItem {
    pub kind: Option<Kind>,
    pub icon: DockIcon,
    /// Windows of this app, oldest first; empty for a fixed app that is not running.
    pub windows: Vec<u32>,
}

impl DockItem {
    pub fn running(&self) -> bool {
        !self.windows.is_empty()
    }

    pub fn fixed_app(&self) -> Option<DockApp> {
        match &self.kind {
            Some(Kind::Fixed(app)) => Some(*app),
            _ => None,
        }
    }
}

/// The dock's tiles for the windows in `toplevels`: the launcher and the four fixed
/// apps in their places, then the other apps in the order their first window opened.
pub fn items(toplevels: &Toplevels) -> Vec<DockItem> {
    let mut items: Vec<DockItem> = DockApp::ALL
        .iter()
        .map(|app| DockItem {
            kind: (*app != DockApp::Launcher).then_some(Kind::Fixed(*app)),
            icon: (*app).into(),
            windows: Vec::new(),
        })
        .collect();
    for (id, window) in toplevels.iter() {
        if window.is_desktop() {
            continue;
        }
        let kind = classify(&window.app_id, &window.title);
        if let Some(item) = items.iter_mut().find(|i| i.kind.as_ref() == Some(&kind)) {
            item.windows.push(id);
            continue;
        }
        let Kind::Extra { icon, .. } = &kind else {
            continue;
        };
        items.push(DockItem {
            icon: *icon,
            kind: Some(kind),
            windows: vec![id],
        });
    }
    items
}

pub fn tiles(items: &[DockItem]) -> Vec<DockTile> {
    items
        .iter()
        .map(|item| DockTile {
            icon: item.icon,
            running: item.kind.is_some().then(|| item.running()),
        })
        .collect()
}

/// The window to raise when `item` is clicked: the one after the focused window if the
/// focus is already in this app (so repeated clicks go round its windows), else its
/// first.
pub fn window_to_raise(item: &DockItem, active: Option<u32>) -> Option<u32> {
    let first = *item.windows.first()?;
    match active.and_then(|a| item.windows.iter().position(|w| *w == a)) {
        Some(i) => Some(item.windows[(i + 1) % item.windows.len()]),
        None => Some(first),
    }
}
