//! The persistent dock: a floating strip of app icons at the bottom of the screen.
//!
//! Five fixed entries (launcher, Files, browser, terminal, Settings). Under every
//! icon but the launcher's sits a dot: teal when the app is running, gray when it is
//! not. Sizes are the desktop concept's, read 1:1 in logical pixels. The
//! window-switcher overlay in `dock.rs` is a different thing and is left as it is.

use std::collections::HashSet;
use std::time::Instant;

use jag_draw::{Brush, Rect, RoundedRadii, RoundedRect};

use orus_core::appearance::Theme;

use crate::app_kind::AppKind;
use crate::launcher::AppId;
use crate::palette::{self, color};
use crate::{external_app, paths, web_app};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockApp {
    Launcher,
    Files,
    Browser,
    Terminal,
    Settings,
}

/// The picture on a dock tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockIcon {
    Launcher,
    Files,
    Browser,
    Terminal,
    Settings,
    TextEdit,
    Viewer,
    /// The installer, shown in the launcher on live media.
    Install,
    /// The app store.
    Store,
    /// LibreOffice.
    Office,
    /// An app with no tile of its own.
    App,
}

impl DockIcon {
    /// Name of the embedded icon for `theme`.
    pub fn name(self, theme: Theme) -> &'static str {
        match (self, theme) {
            (DockIcon::Launcher, Theme::Dark) => "dock-launcher.svg",
            (DockIcon::Files, Theme::Dark) => "dock-files.svg",
            (DockIcon::Browser, Theme::Dark) => "dock-browser.svg",
            (DockIcon::Terminal, Theme::Dark) => "dock-terminal.svg",
            (DockIcon::Settings, Theme::Dark) => "dock-settings.svg",
            (DockIcon::TextEdit, Theme::Dark) => "dock-textedit.svg",
            (DockIcon::Viewer, Theme::Dark) => "dock-viewer.svg",
            (DockIcon::Install, Theme::Dark) => "dock-install.svg",
            (DockIcon::Store, Theme::Dark) => "dock-store.svg",
            (DockIcon::Office, Theme::Dark) => "dock-office.svg",
            (DockIcon::App, Theme::Dark) => "dock-app.svg",
            (DockIcon::Launcher, Theme::Light) => "dock-launcher-light.svg",
            (DockIcon::Files, Theme::Light) => "dock-files-light.svg",
            (DockIcon::Browser, Theme::Light) => "dock-browser-light.svg",
            (DockIcon::Terminal, Theme::Light) => "dock-terminal-light.svg",
            (DockIcon::Settings, Theme::Light) => "dock-settings-light.svg",
            (DockIcon::TextEdit, Theme::Light) => "dock-textedit-light.svg",
            (DockIcon::Viewer, Theme::Light) => "dock-viewer-light.svg",
            (DockIcon::Install, Theme::Light) => "dock-install-light.svg",
            (DockIcon::Store, Theme::Light) => "dock-store-light.svg",
            (DockIcon::Office, Theme::Light) => "dock-office-light.svg",
            (DockIcon::App, Theme::Light) => "dock-app-light.svg",
        }
    }
}

impl From<DockApp> for DockIcon {
    fn from(app: DockApp) -> Self {
        match app {
            DockApp::Launcher => DockIcon::Launcher,
            DockApp::Files => DockIcon::Files,
            DockApp::Browser => DockIcon::Browser,
            DockApp::Terminal => DockIcon::Terminal,
            DockApp::Settings => DockIcon::Settings,
        }
    }
}

/// One tile of the dock: its picture, and whether it has a dot under it (`None`: no
/// dot, as for the launcher; `Some(true)`: the app is running).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockTile {
    pub icon: DockIcon,
    pub running: Option<bool>,
}

impl DockApp {
    pub const ALL: [DockApp; 5] = [
        DockApp::Launcher,
        DockApp::Files,
        DockApp::Browser,
        DockApp::Terminal,
        DockApp::Settings,
    ];

    /// The app this icon starts; `None` for the launcher icon, which toggles the launcher.
    pub fn app_id(self) -> Option<AppId> {
        match self {
            DockApp::Launcher => None,
            DockApp::Files => Some(AppId::FileManager),
            DockApp::Browser => Some(AppId::Chromium),
            DockApp::Terminal => Some(AppId::Terminal),
            DockApp::Settings => Some(AppId::WebApp {
                dir: settings_app_dir(),
            }),
        }
    }

    /// The dock apps the programs this process started are still running.
    pub fn running_from_processes() -> HashSet<DockApp> {
        let mut running = HashSet::new();
        for (app, kind) in [
            (DockApp::Files, AppKind::Files),
            (DockApp::Terminal, AppKind::Terminal),
        ] {
            if external_app::is_running_prefix(&external_app::app_key_prefix(kind)) {
                running.insert(app);
            }
        }
        if external_app::is_running(external_app::CHROMIUM_KEY) {
            running.insert(DockApp::Browser);
        }
        let settings_index = settings_app_dir().join("index.html");
        if external_app::is_running(&web_app::running_key(&settings_index)) {
            running.insert(DockApp::Settings);
        }
        running
    }
}

/// The Store web app's folder.
pub fn store_app_dir() -> std::path::PathBuf {
    paths::apps_dir().join("orus-store")
}

/// The Settings web app's folder.
pub fn settings_app_dir() -> std::path::PathBuf {
    paths::apps_dir().join("orus-system-settings")
}

// Above in-shell windows, below the launcher (500_000) and window switcher (490_000).
const DOCK_Z: i32 = 480_000;

const ICON_SIZE: f32 = 40.0;
/// Distance between icon centers.
const ICON_PITCH: f32 = 65.0;
/// Space left of the first icon and right of the last.
const SIDE_PAD: f32 = 21.0;
const TOP_PAD: f32 = 14.0;
/// Gap between an icon and its dot.
const DOT_GAP: f32 = 6.0;
const DOT_SIZE: f32 = 7.0;
const BOTTOM_PAD: f32 = 5.0;
const CORNER: f32 = 22.0;
const BORDER_WIDTH: f32 = 1.0;
/// Space between the dock and the bottom edge of the screen.
const BOTTOM_MARGIN: f32 = 32.0;

/// Tiles in the dock before any other window adds one.
pub const FIXED_TILES: usize = DockApp::ALL.len();

/// Width of the dock with `tiles` tiles.
pub fn dock_width(tiles: usize) -> f32 {
    SIDE_PAD * 2.0 + ICON_PITCH * (tiles.max(1) as f32 - 1.0) + ICON_SIZE
}

/// Width of the dock with only its fixed tiles.
pub const DOCK_WIDTH: f32 = SIDE_PAD * 2.0 + ICON_PITCH * (FIXED_TILES as f32 - 1.0) + ICON_SIZE;
const DOCK_HEIGHT: f32 = TOP_PAD + ICON_SIZE + DOT_GAP + DOT_SIZE + BOTTOM_PAD;

/// The strip of the screen the dock takes, margin included.
pub const BAND_HEIGHT: f32 = DOCK_HEIGHT + BOTTOM_MARGIN;

/// Time the dock takes to slide away or come back.
const SLIDE_SECONDS: f32 = 0.18;

/// Height of the strip at the bottom edge where the pointer brings the dock back.
const EDGE_ZONE: f32 = 6.0;

/// The dock slides out of sight under a maximized window and comes back when the
/// pointer reaches the bottom edge.
pub struct DockBar {
    /// 1.0 fully shown, 0.0 fully hidden.
    shown: f32,
    last_tick: Instant,
    /// How many tiles the dock draws, which sets its width.
    tiles: usize,
}

impl Default for DockBar {
    fn default() -> Self {
        Self::new()
    }
}

fn rounded(rect: Rect, radius: f32) -> RoundedRect {
    RoundedRect {
        rect,
        radii: RoundedRadii {
            tl: radius,
            tr: radius,
            br: radius,
            bl: radius,
        },
    }
}

impl DockBar {
    pub fn new() -> Self {
        Self {
            shown: 1.0,
            last_tick: Instant::now(),
            tiles: FIXED_TILES,
        }
    }

    /// Whether the dock should be out: always, unless a window `covers` the screen,
    /// then only while the pointer is at the bottom edge or over the dock.
    pub fn wants_to_show(&self, viewport: Rect, pointer: (f32, f32), covers: bool) -> bool {
        if !covers {
            return true;
        }
        let dock = self.rect(viewport);
        let at_edge = pointer.1 >= viewport.y + viewport.h - EDGE_ZONE;
        let over_dock = self.shown > 0.0
            && pointer.0 >= dock.x
            && pointer.0 < dock.x + dock.w
            && pointer.1 >= dock.y;
        at_edge || over_dock
    }

    /// Move the slide toward `show` by the time since the last call, or all the
    /// way if `instant`. Returns true while the dock is still moving.
    pub fn tick(&mut self, show: bool, instant: bool) -> bool {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32().min(0.1);
        self.last_tick = now;
        let target = if show { 1.0 } else { 0.0 };
        if instant {
            self.shown = target;
            return false;
        }
        let step = dt / SLIDE_SECONDS;
        self.shown = if self.shown < target {
            (self.shown + step).min(target)
        } else {
            (self.shown - step).max(target)
        };
        self.shown != target
    }

    /// Whether the dock is out of sight, so nothing is drawn or clickable.
    pub fn is_hidden(&self) -> bool {
        self.shown <= 0.0
    }

    /// Draw the dock with `tiles` tiles from now on.
    pub fn set_tile_count(&mut self, tiles: usize) {
        self.tiles = tiles.max(1);
    }

    /// The dock's rect now: the resting rect pushed down by how far it has slid.
    fn current_rect(&self, viewport: Rect) -> Rect {
        let mut dock = self.rect(viewport);
        dock.y += (1.0 - self.shown) * BAND_HEIGHT;
        dock
    }

    /// The dock's rect for a window of `viewport` size, centered above the bottom margin.
    pub fn rect(&self, viewport: Rect) -> Rect {
        let width = dock_width(self.tiles);
        Rect {
            x: viewport.x + (viewport.w - width) * 0.5,
            y: viewport.y + viewport.h - BAND_HEIGHT,
            w: width,
            h: DOCK_HEIGHT,
        }
    }

    fn icon_rect(dock: Rect, index: usize) -> Rect {
        Rect {
            x: dock.x + SIDE_PAD + index as f32 * ICON_PITCH,
            y: dock.y + TOP_PAD,
            w: ICON_SIZE,
            h: ICON_SIZE,
        }
    }

    /// The tile whose cell contains `(x, y)`. A cell is as wide as the pitch, so the
    /// gaps between icons still hit the nearer one.
    pub fn hit_tile(&self, viewport: Rect, x: f32, y: f32) -> Option<usize> {
        if self.shown < 0.5 {
            return None;
        }
        let dock = self.current_rect(viewport);
        if x < dock.x || x >= dock.x + dock.w || y < dock.y || y >= dock.y + dock.h {
            return None;
        }
        let first_left = dock.x + SIDE_PAD - (ICON_PITCH - ICON_SIZE) * 0.5;
        let index = ((x - first_left) / ICON_PITCH).floor().max(0.0) as usize;
        (index < self.tiles).then_some(index)
    }

    /// The fixed app whose tile contains `(x, y)`, for a dock of only fixed tiles.
    pub fn hit_test(&self, viewport: Rect, x: f32, y: f32) -> Option<DockApp> {
        self.hit_tile(viewport, x, y)
            .and_then(|i| DockApp::ALL.get(i).copied())
    }

    /// Draw the dock of only fixed tiles; `running` are the apps to give a teal dot.
    pub fn render(
        &self,
        canvas: &mut jag_surface::Canvas,
        viewport: Rect,
        running: &HashSet<DockApp>,
    ) {
        let tiles: Vec<DockTile> = DockApp::ALL
            .iter()
            .map(|app| DockTile {
                icon: (*app).into(),
                running: (*app != DockApp::Launcher).then(|| running.contains(app)),
            })
            .collect();
        self.render_tiles(canvas, viewport, &tiles);
    }

    pub fn render_tiles(
        &self,
        canvas: &mut jag_surface::Canvas,
        viewport: Rect,
        tiles: &[DockTile],
    ) {
        if self.is_hidden() {
            return;
        }
        let dock = self.current_rect(viewport);
        let pal = palette::get();
        canvas.rounded_rect(
            rounded(
                Rect {
                    x: dock.x - BORDER_WIDTH,
                    y: dock.y - BORDER_WIDTH,
                    w: dock.w + BORDER_WIDTH * 2.0,
                    h: dock.h + BORDER_WIDTH * 2.0,
                },
                CORNER + BORDER_WIDTH,
            ),
            Brush::Solid(color(pal.dock_border)),
            DOCK_Z,
        );
        canvas.rounded_rect(
            rounded(dock, CORNER),
            Brush::Solid(color(pal.dock)),
            DOCK_Z + 1,
        );

        for (index, tile) in tiles.iter().enumerate() {
            let rect = Self::icon_rect(dock, index);
            canvas.draw_svg(
                tile.icon.name(pal.theme),
                [rect.x, rect.y],
                [rect.w, rect.h],
                DOCK_Z + 2,
            );
            let Some(running) = tile.running else {
                continue;
            };
            let dot = if running {
                pal.dot_running
            } else {
                pal.dot_idle
            };
            canvas.rounded_rect(
                rounded(
                    Rect {
                        x: rect.x + (rect.w - DOT_SIZE) * 0.5,
                        y: rect.y + rect.h + DOT_GAP,
                        w: DOT_SIZE,
                        h: DOT_SIZE,
                    },
                    DOT_SIZE * 0.5,
                ),
                Brush::Solid(color(dot)),
                DOCK_Z + 3,
            );
        }
    }
}
