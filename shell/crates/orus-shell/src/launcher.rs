//! Orus OS launcher overlay: a Spotlight/Launchpad-style app grid.
//!
//! Triggered by Cmd+Space, the launcher displays available apps as a grid
//! over a dark scrim. Supports keyboard navigation (arrows + Enter) and
//! mouse clicks. Clicking outside the panel dismisses the overlay.
//!
//! ```text
//! +----------- scrim (full viewport) ----------+
//! |                                             |
//! |    +---- panel (centered, rounded) ----+    |
//! |    |         Launcher                  |    |
//! |    |                                   |    |
//! |    |    [  Terminal  ]   [  ...  ]      |    |
//! |    |                                   |    |
//! |    +-----------------------------------+    |
//! |                                             |
//! +---------------------------------------------+
//! ```

use std::path::PathBuf;
use std::time::Instant;

use jag_draw::{Brush, ColorLinPremul, FontStyle, Rect, RoundedRadii, RoundedRect};
use orus_core::desktop_apps::DesktopApp;
use orus_core::installed_apps::InstalledApp;

use crate::dock_bar::{DockIcon, settings_app_dir, store_app_dir};
use crate::palette::{self, color};

// ── Z-index assignments (must be above all possible window stacks) ──
// With z_gap=20000 per window and chrome_z=z_base+12000, N windows can
// reach z = (N-1)*20000 + 12000. 500000 safely covers ~25 windows.
const SCRIM_Z: i32 = 500_000;
const PANEL_BG_Z: i32 = 500_100;
const PANEL_BORDER_Z: i32 = 500_101;
const ICON_Z: i32 = 500_110;
const LABEL_Z: i32 = 500_111;

// ── Panel geometry ─────────────────────────────────────────────────
const PANEL_W: f32 = 720.0;
const PANEL_MIN_H: f32 = 200.0;
const PANEL_CORNER: f32 = 16.0;
const PANEL_PADDING_TOP: f32 = 24.0;
const PANEL_PADDING_H: f32 = 24.0;
const PANEL_PADDING_BOTTOM: f32 = 24.0;

// ── Search bar geometry ────────────────────────────────────────────
const SEARCH_BAR_H: f32 = 36.0;
const SEARCH_BAR_CORNER: f32 = 8.0;
const SEARCH_FONT_SIZE: f32 = 14.0;
const SEARCH_BAR_Z: i32 = 500_102;
const SEARCH_TEXT_Z: i32 = 500_103;

// ── App grid geometry ──────────────────────────────────────────────
const CELL_W: f32 = 112.0;
const CELL_H: f32 = 112.0;
/// The app tile, drawn from the same art as the dock's.
const ICON_DIAMETER: f32 = 56.0;
const ICON_RADIUS: f32 = ICON_DIAMETER / 2.0;
const LABEL_FONT_SIZE: f32 = 13.0;
const ICON_LABEL_GAP: f32 = 10.0;
/// The focused app's highlight, around its tile and label.
const FOCUS_CORNER: f32 = 14.0;
const FOCUS_PAD: f32 = 8.0;

// ── Colors ──────────────────────────────────────────────────────────
// The panel, search field, text and highlight come from the theme's palette.
const SCRIM_COLOR: ColorLinPremul = ColorLinPremul {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.55,
};

// ── Helpers ───────────────────────────────────────────────────────

// ── App identifier ─────────────────────────────────────────────────

/// Identifies a launchable application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppId {
    Terminal,
    TextEdit,
    FileManager,
    Viewer,
    /// The system Chromium, run as its own window outside the shell.
    Chromium,
    /// The Orus OS installer, present only on live media.
    Installer,
    /// LibreOffice, or its installer when it is missing; present only on installed systems.
    Office,
    /// An installed web app: a directory with an `index.html` and a
    /// `waid-package.json`, run by `detir-scene` in its own window.
    WebApp {
        dir: PathBuf,
    },
    /// A program installed with the package manager, started from its `.desktop` command.
    Desktop {
        argv: Vec<String>,
    },
}

/// An entry in the launcher grid.
#[derive(Debug, Clone)]
pub struct AppEntry {
    pub id: AppId,
    pub name: String,
    pub icon: DockIcon,
}

/// Result of user interaction with the launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherAction {
    /// User selected an app to launch.
    Launch(AppId),
    /// User dismissed the launcher (Escape or click outside).
    Dismiss,
}

/// Abstraction over keyboard keys the launcher cares about.
/// Keeps winit out of the shell crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherKey {
    Escape,
    Enter,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Backspace,
    Char(char),
}

// ── Launcher ───────────────────────────────────────────────────────

/// Cursor blink period in milliseconds (on for 530ms, off for 530ms).
const CURSOR_BLINK_MS: u128 = 530;

/// Launcher overlay state.
pub struct Launcher {
    pub visible: bool,
    /// Apps the shell itself provides.
    builtin: Vec<AppEntry>,
    /// Shown entries: the built-in apps, then the installed web apps.
    apps: Vec<AppEntry>,
    focus_index: usize,
    query: String,
    /// Reset on every keystroke so cursor stays solid while typing.
    cursor_epoch: Instant,
}

/// The installer wrapper from the `orus-installer` package. The installer
/// removes that package from the installed system, so the entry shows only on
/// live media.
const INSTALLER_PATH: &str = "/usr/bin/orus-install";

fn builtin_apps(installer_present: bool) -> Vec<AppEntry> {
    let entry = |id, name: &str, icon| AppEntry {
        id,
        name: name.to_string(),
        icon,
    };
    let mut apps = vec![
        entry(AppId::Terminal, "Terminal", DockIcon::Terminal),
        entry(AppId::TextEdit, "Text Edit", DockIcon::TextEdit),
        entry(AppId::FileManager, "Files", DockIcon::Files),
        entry(AppId::Viewer, "Viewer", DockIcon::Viewer),
        entry(AppId::Chromium, "Chromium", DockIcon::Browser),
    ];
    if installer_present {
        apps.push(entry(
            AppId::Installer,
            "Install Orus OS",
            DockIcon::Install,
        ));
    } else {
        // Installing about 500 MB into a live session's memory is no use.
        apps.push(entry(AppId::Office, "Office", DockIcon::Office));
    }
    apps
}

/// The package-manager programs to list: not Chromium, which has a tile of its own.
pub fn desktop_apps() -> Vec<DesktopApp> {
    orus_core::desktop_apps::list(&orus_core::desktop_apps::default_dirs(), |app| {
        app.argv[0].ends_with("chromium")
    })
}

impl Default for Launcher {
    fn default() -> Self {
        let builtin = builtin_apps(std::path::Path::new(INSTALLER_PATH).exists());
        Self {
            visible: false,
            apps: builtin.clone(),
            builtin,
            focus_index: 0,
            query: String::new(),
            cursor_epoch: Instant::now(),
        }
    }
}

impl Launcher {
    /// The names of the entries shown, in order.
    pub fn entry_names(&self) -> Vec<String> {
        self.apps.iter().map(|app| app.name.clone()).collect()
    }

    /// A launcher as it would be on a machine with or without the installer.
    pub fn with_installer(installer_present: bool) -> Self {
        let builtin = builtin_apps(installer_present);
        Self {
            visible: false,
            apps: builtin.clone(),
            builtin,
            focus_index: 0,
            query: String::new(),
            cursor_epoch: Instant::now(),
        }
    }

    /// Replace the installed web apps shown after the built-in ones.
    pub fn set_installed(&mut self, installed: &[InstalledApp]) {
        self.apps = self.builtin.clone();
        self.apps.extend(installed.iter().map(|app| AppEntry {
            id: AppId::WebApp {
                dir: app.dir.clone(),
            },
            name: app.name.clone(),
            icon: if app.dir == settings_app_dir() {
                DockIcon::Settings
            } else if app.dir == store_app_dir() {
                DockIcon::Store
            } else {
                DockIcon::App
            },
        }));
    }

    /// Add the programs installed with the package manager after the web apps.
    pub fn set_desktop(&mut self, apps: &[DesktopApp]) {
        self.apps.extend(apps.iter().map(|app| AppEntry {
            id: AppId::Desktop {
                argv: app.argv.clone(),
            },
            name: app.name.clone(),
            icon: DockIcon::App,
        }));
    }

    /// Toggle visibility. Resets focus index and search query when opening.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.focus_index = 0;
            self.query.clear();
            self.cursor_epoch = Instant::now();
        }
    }

    /// Whether the cursor should be visible right now (blink cycle).
    fn cursor_visible(&self) -> bool {
        self.cursor_epoch.elapsed().as_millis() % (CURSOR_BLINK_MS * 2) < CURSOR_BLINK_MS
    }

    /// Returns true if the launcher needs continuous redraws (for cursor blink).
    pub fn needs_redraw(&self) -> bool {
        self.visible
    }

    /// Return apps whose name matches the current search query (case-insensitive).
    fn filtered_apps(&self) -> Vec<&AppEntry> {
        if self.query.is_empty() {
            self.apps.iter().collect()
        } else {
            let q = self.query.to_lowercase();
            self.apps
                .iter()
                .filter(|a| a.name.to_lowercase().contains(&q))
                .collect()
        }
    }

    // ── Rendering ──────────────────────────────────────────────────

    /// Render the launcher overlay. Only draws when visible.
    ///
    /// `viewport` is the app viewport rect (below the shell bar).
    pub fn render(&self, canvas: &mut jag_surface::Canvas, viewport: Rect) {
        if !self.visible {
            return;
        }
        let pal = palette::get();

        // Scrim: semi-transparent dark overlay covering entire viewport
        canvas.fill_rect(
            viewport.x,
            viewport.y,
            viewport.w,
            viewport.h,
            Brush::Solid(SCRIM_COLOR),
            SCRIM_Z,
        );

        // Panel: centered rounded rectangle
        let panel = self.panel_rect(viewport);

        canvas.rounded_rect(
            RoundedRect {
                rect: panel,
                radii: RoundedRadii {
                    tl: PANEL_CORNER,
                    tr: PANEL_CORNER,
                    br: PANEL_CORNER,
                    bl: PANEL_CORNER,
                },
            },
            Brush::Solid(color(pal.sidebar)),
            PANEL_BG_Z,
        );

        // Panel border (1px inset look via slightly larger rounded rect)
        let border_rect = Rect {
            x: panel.x - 1.0,
            y: panel.y - 1.0,
            w: panel.w + 2.0,
            h: panel.h + 2.0,
        };
        canvas.rounded_rect(
            RoundedRect {
                rect: border_rect,
                radii: RoundedRadii {
                    tl: PANEL_CORNER + 1.0,
                    tr: PANEL_CORNER + 1.0,
                    br: PANEL_CORNER + 1.0,
                    bl: PANEL_CORNER + 1.0,
                },
            },
            Brush::Solid(color(pal.border)),
            PANEL_BORDER_Z - 1, // behind panel bg
        );

        // ── Search bar ─────────────────────────────────────────────
        let search_x = panel.x + PANEL_PADDING_H;
        let search_y = panel.y + PANEL_PADDING_TOP;
        let search_w = panel.w - PANEL_PADDING_H * 2.0;

        // Search bar border (behind bg)
        let sb_border = Rect {
            x: search_x - 1.0,
            y: search_y - 1.0,
            w: search_w + 2.0,
            h: SEARCH_BAR_H + 2.0,
        };
        canvas.rounded_rect(
            RoundedRect {
                rect: sb_border,
                radii: RoundedRadii {
                    tl: SEARCH_BAR_CORNER + 1.0,
                    tr: SEARCH_BAR_CORNER + 1.0,
                    br: SEARCH_BAR_CORNER + 1.0,
                    bl: SEARCH_BAR_CORNER + 1.0,
                },
            },
            Brush::Solid(color(pal.border)),
            SEARCH_BAR_Z - 1,
        );

        // Search bar background
        canvas.rounded_rect(
            RoundedRect {
                rect: Rect {
                    x: search_x,
                    y: search_y,
                    w: search_w,
                    h: SEARCH_BAR_H,
                },
                radii: RoundedRadii {
                    tl: SEARCH_BAR_CORNER,
                    tr: SEARCH_BAR_CORNER,
                    br: SEARCH_BAR_CORNER,
                    bl: SEARCH_BAR_CORNER,
                },
            },
            Brush::Solid(color(pal.field)),
            SEARCH_BAR_Z,
        );

        // Search text or placeholder
        let text_x = search_x + 12.0;
        let text_y = search_y + (SEARCH_BAR_H + SEARCH_FONT_SIZE) * 0.5 - 2.0;
        let query_w = if self.query.is_empty() {
            canvas.draw_text_run_styled(
                [text_x, text_y],
                "Search apps...".to_string(),
                SEARCH_FONT_SIZE,
                400.0,
                FontStyle::Normal,
                None,
                color(pal.text_dim),
                SEARCH_TEXT_Z,
            );
            0.0
        } else {
            canvas.draw_text_run_styled(
                [text_x, text_y],
                self.query.clone(),
                SEARCH_FONT_SIZE,
                400.0,
                FontStyle::Normal,
                None,
                color(pal.text),
                SEARCH_TEXT_Z,
            );
            canvas.measure_text_width(&self.query, SEARCH_FONT_SIZE)
        };

        // Blinking cursor
        if self.cursor_visible() {
            canvas.fill_rect(
                text_x + query_w + 1.0,
                search_y + 8.0,
                1.5,
                SEARCH_BAR_H - 16.0,
                Brush::Solid(color(pal.text)),
                SEARCH_TEXT_Z,
            );
        }

        // ── App grid ───────────────────────────────────────────────
        let filtered = self.filtered_apps();
        let grid_top = search_y + SEARCH_BAR_H + 24.0;

        if filtered.is_empty() {
            let msg = "No matching apps";
            let msg_w = canvas.measure_text_width(msg, SEARCH_FONT_SIZE);
            let msg_x = panel.x + (panel.w - msg_w) * 0.5;
            let msg_y = grid_top + 40.0;
            canvas.draw_text_run_styled(
                [msg_x, msg_y],
                msg.to_string(),
                SEARCH_FONT_SIZE,
                400.0,
                FontStyle::Normal,
                None,
                color(pal.text_dim),
                LABEL_Z,
            );
            return;
        }

        let cols = self.max_cols();
        for (i, app) in filtered.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            // Center the items in each row
            // Every row lines up with the first: the grid is centered once.
            let items_in_row = filtered.len().min(cols);
            let row_w = items_in_row as f32 * CELL_W;
            let row_x = panel.x + (panel.w - row_w) * 0.5;

            let cell_x = row_x + col as f32 * CELL_W;
            let icon_cx = cell_x + CELL_W * 0.5;
            let icon_cy = grid_top + row as f32 * CELL_H + ICON_RADIUS;

            // The focused app (the one Enter starts) is lit behind its tile and label.
            if i == self.focus_index {
                canvas.rounded_rect(
                    RoundedRect {
                        rect: Rect {
                            x: cell_x + FOCUS_PAD * 0.5,
                            y: icon_cy - ICON_RADIUS - FOCUS_PAD,
                            w: CELL_W - FOCUS_PAD,
                            h: ICON_DIAMETER
                                + ICON_LABEL_GAP
                                + LABEL_FONT_SIZE
                                + FOCUS_PAD * 2.0
                                + 2.0,
                        },
                        radii: RoundedRadii {
                            tl: FOCUS_CORNER,
                            tr: FOCUS_CORNER,
                            br: FOCUS_CORNER,
                            bl: FOCUS_CORNER,
                        },
                    },
                    Brush::Solid(color(pal.selection)),
                    ICON_Z - 1,
                );
            }

            canvas.draw_svg(
                app.icon.name(pal.theme),
                [icon_cx - ICON_RADIUS, icon_cy - ICON_RADIUS],
                [ICON_DIAMETER, ICON_DIAMETER],
                ICON_Z,
            );

            // Label: app name below icon
            let label_w = canvas.measure_text_width(&app.name, LABEL_FONT_SIZE);
            let label_x = icon_cx - label_w * 0.5;
            let label_y = icon_cy + ICON_RADIUS + ICON_LABEL_GAP + LABEL_FONT_SIZE;
            canvas.draw_text_run_styled(
                [label_x, label_y],
                app.name.to_string(),
                LABEL_FONT_SIZE,
                400.0,
                FontStyle::Normal,
                None,
                color(pal.text),
                LABEL_Z,
            );
        }
    }

    // ── Hit testing ────────────────────────────────────────────────

    /// Hit-test a click at `(x, y)` in viewport coordinates.
    ///
    /// Returns:
    /// - `Some(Launch(id))` if an app entry was clicked
    /// - `Some(Dismiss)` if the scrim (outside panel) was clicked
    /// - `None` if the panel background was clicked (absorb the click)
    pub fn hit_test(&self, x: f32, y: f32, viewport: Rect) -> Option<LauncherAction> {
        if !self.visible {
            return None;
        }

        let panel = self.panel_rect(viewport);

        // Check if inside panel
        let in_panel =
            x >= panel.x && x <= panel.x + panel.w && y >= panel.y && y <= panel.y + panel.h;

        if !in_panel {
            return Some(LauncherAction::Dismiss);
        }

        // Check app entry hit targets
        let filtered = self.filtered_apps();
        let cols = self.max_cols();
        let grid_top = panel.y + PANEL_PADDING_TOP + SEARCH_BAR_H + 24.0;

        for (i, app) in filtered.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            // Every row lines up with the first: the grid is centered once.
            let items_in_row = filtered.len().min(cols);
            let row_w = items_in_row as f32 * CELL_W;
            let row_x = panel.x + (panel.w - row_w) * 0.5;

            let cell_x = row_x + col as f32 * CELL_W;
            let icon_cy = grid_top + row as f32 * CELL_H + ICON_RADIUS;
            let hit_rect = Rect {
                x: cell_x,
                y: icon_cy - ICON_RADIUS - 6.0,
                w: CELL_W,
                h: ICON_DIAMETER + ICON_LABEL_GAP + LABEL_FONT_SIZE + 12.0,
            };
            if x >= hit_rect.x
                && x <= hit_rect.x + hit_rect.w
                && y >= hit_rect.y
                && y <= hit_rect.y + hit_rect.h
            {
                return Some(LauncherAction::Launch(app.id.clone()));
            }
        }

        // Clicked on panel background but not on an entry — absorb
        None
    }

    // ── Keyboard navigation ────────────────────────────────────────

    /// Handle a key press while the launcher is visible.
    ///
    /// Returns an action if the key was meaningful, or `None` to absorb it.
    pub fn handle_key(&mut self, key: LauncherKey) -> Option<LauncherAction> {
        // Reset blink so cursor stays solid while typing
        self.cursor_epoch = Instant::now();

        match key {
            LauncherKey::Escape => Some(LauncherAction::Dismiss),
            LauncherKey::Enter => {
                let filtered = self.filtered_apps();
                filtered
                    .get(self.focus_index)
                    .map(|app| LauncherAction::Launch(app.id.clone()))
            }
            LauncherKey::ArrowRight => {
                let count = self.filtered_apps().len();
                if self.focus_index + 1 < count {
                    self.focus_index += 1;
                }
                None
            }
            LauncherKey::ArrowLeft => {
                self.focus_index = self.focus_index.saturating_sub(1);
                None
            }
            LauncherKey::ArrowDown => {
                let count = self.filtered_apps().len();
                let cols = self.max_cols();
                let next = self.focus_index + cols;
                if next < count {
                    self.focus_index = next;
                }
                None
            }
            LauncherKey::ArrowUp => {
                let cols = self.max_cols();
                if self.focus_index >= cols {
                    self.focus_index -= cols;
                }
                None
            }
            LauncherKey::Backspace => {
                self.query.pop();
                self.clamp_focus();
                None
            }
            LauncherKey::Char(ch) => {
                self.query.push(ch);
                self.clamp_focus();
                None
            }
        }
    }

    /// Ensure focus_index stays within the filtered results.
    fn clamp_focus(&mut self) {
        let count = self.filtered_apps().len();
        if count == 0 {
            self.focus_index = 0;
        } else if self.focus_index >= count {
            self.focus_index = count - 1;
        }
    }

    // ── Helpers ────────────────────────────────────────────────────

    /// Maximum columns that fit inside the panel's horizontal padding.
    fn max_cols(&self) -> usize {
        let usable = PANEL_W - PANEL_PADDING_H * 2.0;
        (usable / CELL_W).floor().max(1.0) as usize
    }

    /// Number of rows needed for `n` items.
    fn row_count(&self, n: usize) -> usize {
        let cols = self.max_cols();
        n.div_ceil(cols)
    }

    /// Dynamic panel height based on the number of filtered apps.
    fn panel_height(&self, item_count: usize) -> f32 {
        let rows = self.row_count(item_count).max(1) as f32;
        let grid_h = rows * CELL_H;
        // top padding + search bar + gap + grid + bottom padding
        let h = PANEL_PADDING_TOP + SEARCH_BAR_H + 24.0 + grid_h + PANEL_PADDING_BOTTOM;
        h.max(PANEL_MIN_H)
    }

    /// Compute the centered panel rect within the viewport.
    fn panel_rect(&self, viewport: Rect) -> Rect {
        let h = self.panel_height(self.filtered_apps().len());
        let px = viewport.x + (viewport.w - PANEL_W) * 0.5;
        let py = viewport.y + (viewport.h - h) * 0.5;
        Rect {
            x: px,
            y: py,
            w: PANEL_W,
            h,
        }
    }
}
