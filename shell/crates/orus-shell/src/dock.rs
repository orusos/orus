//! Orus OS Dock: an OS-level window switcher overlay.
//!
//! The dock shows all currently open/running windows from the WindowManager.
//! It's an **app switcher**, not an app launcher (that's the Launcher's job).
//! Clicking an entry focuses that window; the dock can also close windows.
//!
//! ```text
//! ┌──────────── scrim (full viewport) ────────────┐
//! │                                                │
//! │   ┌──────── dock panel (centered) ─────────┐   │
//! │   │                                        │   │
//! │   │   [ Terminal ]   [ Detir Browser ]      │   │
//! │   │       ●              ◉ (focused)       │   │
//! │   │                                        │   │
//! │   └────────────────────────────────────────┘   │
//! │                                                │
//! └────────────────────────────────────────────────┘
//! ```

use jag_draw::{Brush, ColorLinPremul, Rect, RoundedRadii, RoundedRect};

use crate::window_manager::WindowId;

// ── Z-index assignments (must be above all possible window stacks) ──
const DOCK_SCRIM_Z: i32 = 490_000;
const DOCK_PANEL_Z: i32 = 490_100;
const DOCK_BORDER_Z: i32 = 490_101;
const DOCK_ITEM_Z: i32 = 490_110;
const DOCK_LABEL_Z: i32 = 490_111;
// ── Panel geometry ─────────────────────────────────────────────────
const PANEL_PADDING_H: f32 = 24.0;
const PANEL_PADDING_V: f32 = 20.0;
const PANEL_CORNER: f32 = 16.0;

// ── Item geometry ──────────────────────────────────────────────────
const ITEM_W: f32 = 120.0;
const ITEM_H: f32 = 80.0;
const ITEM_GAP: f32 = 16.0;
const ICON_SIZE: f32 = 36.0;
const LABEL_FONT_SIZE: f32 = 12.0;

// ── Colors ─────────────────────────────────────────────────────────
const SCRIM_COLOR: ColorLinPremul = ColorLinPremul {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.50,
};

const PANEL_BG: ColorLinPremul = ColorLinPremul {
    // #252528
    r: 0.0179,
    g: 0.0179,
    b: 0.0192,
    a: 0.95,
};

const PANEL_BORDER: ColorLinPremul = ColorLinPremul {
    // #3a3a3e
    r: 0.0423,
    g: 0.0423,
    b: 0.0455,
    a: 1.0,
};

const ITEM_BG: ColorLinPremul = ColorLinPremul {
    // #2a2a2e
    r: 0.0192,
    g: 0.0192,
    b: 0.0210,
    a: 1.0,
};

const ITEM_FOCUSED_BG: ColorLinPremul = ColorLinPremul {
    // #3b82f6 at 30% opacity
    r: 0.0127,
    g: 0.0692,
    b: 0.2654,
    a: 0.30,
};

const ITEM_FOCUSED_BORDER: ColorLinPremul = ColorLinPremul {
    // #3b82f6
    r: 0.0423,
    g: 0.2307,
    b: 0.8845,
    a: 1.0,
};

const LABEL_COLOR: ColorLinPremul = ColorLinPremul {
    // #e0e0e4
    r: 0.7354,
    g: 0.7354,
    b: 0.7649,
    a: 1.0,
};

/// Terminal icon green
const TERMINAL_ICON_COLOR: ColorLinPremul = ColorLinPremul {
    // #34c759
    r: 0.0303,
    g: 0.5639,
    b: 0.0963,
    a: 1.0,
};

/// Browser icon blue
const BROWSER_ICON_COLOR: ColorLinPremul = ColorLinPremul {
    // #3b82f6
    r: 0.0423,
    g: 0.2307,
    b: 0.8845,
    a: 1.0,
};

/// An entry in the dock overlay, built from WindowManager state.
#[derive(Debug, Clone)]
pub struct DockEntry {
    pub window_id: WindowId,
    pub title: String,
    pub is_terminal: bool,
    pub focused: bool,
}

/// Action returned when the user interacts with the dock.
#[derive(Debug)]
pub enum DockAction {
    /// Switch focus to this window.
    FocusWindow(WindowId),
    /// Close this window.
    CloseWindow(WindowId),
    /// Dismiss the dock overlay.
    Dismiss,
}

/// The Orus OS dock overlay — an OS-level window switcher.
#[derive(Default)]
pub struct OrusDock {
    pub visible: bool,
}

impl OrusDock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Render the dock overlay.
    ///
    /// `entries` are built by the caller from `WindowManager::windows()`.
    /// `viewport` is the full app viewport rect (below shell bar).
    pub fn render(
        &self,
        canvas: &mut jag_surface::Canvas,
        entries: &[DockEntry],
        viewport: Rect,
        _provider: &dyn jag_draw::TextProvider,
    ) {
        if !self.visible || entries.is_empty() {
            return;
        }

        // Scrim (full viewport)
        canvas.fill_rect(
            viewport.x,
            viewport.y,
            viewport.w,
            viewport.h,
            Brush::Solid(SCRIM_COLOR),
            DOCK_SCRIM_Z,
        );
        // Hit region for scrim (click to dismiss)
        canvas.hit_region_rect(SCRIM_HIT_REGION, viewport, DOCK_SCRIM_Z);

        // Panel size based on number of entries
        let n = entries.len() as f32;
        let content_w = n * ITEM_W + (n - 1.0).max(0.0) * ITEM_GAP;
        let panel_w = content_w + PANEL_PADDING_H * 2.0;
        let panel_h = ITEM_H + PANEL_PADDING_V * 2.0 + 28.0; // 28 for title row
        let panel_x = viewport.x + (viewport.w - panel_w) / 2.0;
        let panel_y = viewport.y + (viewport.h - panel_h) / 2.0;

        let panel_rect = Rect {
            x: panel_x,
            y: panel_y,
            w: panel_w,
            h: panel_h,
        };

        // Panel border (slightly larger rounded rect behind the fill)
        let radii = RoundedRadii {
            tl: PANEL_CORNER,
            tr: PANEL_CORNER,
            br: PANEL_CORNER,
            bl: PANEL_CORNER,
        };
        let border_rect = Rect {
            x: panel_rect.x - 1.0,
            y: panel_rect.y - 1.0,
            w: panel_rect.w + 2.0,
            h: panel_rect.h + 2.0,
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
            Brush::Solid(PANEL_BORDER),
            DOCK_PANEL_Z,
        );
        // Panel background
        canvas.rounded_rect(
            RoundedRect {
                rect: panel_rect,
                radii,
            },
            Brush::Solid(PANEL_BG),
            DOCK_BORDER_Z,
        );
        // Panel hit region (absorb clicks on the panel itself)
        canvas.hit_region_rect(PANEL_HIT_REGION, panel_rect, DOCK_PANEL_Z);

        // Title (centered)
        let title_text = "Open Windows";
        let title_size = 14.0;
        let title_w = canvas.measure_text_width(title_text, title_size);
        let title_x = panel_x + (panel_w - title_w) * 0.5;
        let title_y = panel_y + PANEL_PADDING_V + 14.0; // baseline
        canvas.draw_text_run_weighted(
            [title_x, title_y],
            title_text.to_string(),
            title_size,
            600.0,
            LABEL_COLOR,
            DOCK_LABEL_Z,
        );

        // Items
        let items_y = panel_y + PANEL_PADDING_V + 28.0;
        let items_start_x = panel_x + PANEL_PADDING_H;

        let white = ColorLinPremul {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };

        for (i, entry) in entries.iter().enumerate() {
            let item_x = items_start_x + i as f32 * (ITEM_W + ITEM_GAP);
            let item_rect = Rect {
                x: item_x,
                y: items_y,
                w: ITEM_W,
                h: ITEM_H,
            };

            // Item background (highlight if focused)
            let item_radii = RoundedRadii {
                tl: 8.0,
                tr: 8.0,
                br: 8.0,
                bl: 8.0,
            };
            let (bg_color, border_color) = if entry.focused {
                (ITEM_FOCUSED_BG, Some(ITEM_FOCUSED_BORDER))
            } else {
                (ITEM_BG, None)
            };
            canvas.rounded_rect(
                RoundedRect {
                    rect: item_rect,
                    radii: item_radii,
                },
                Brush::Solid(bg_color),
                DOCK_ITEM_Z,
            );
            if let Some(bc) = border_color {
                let border_item = Rect {
                    x: item_rect.x - 1.5,
                    y: item_rect.y - 1.5,
                    w: item_rect.w + 3.0,
                    h: item_rect.h + 3.0,
                };
                canvas.rounded_rect(
                    RoundedRect {
                        rect: border_item,
                        radii: RoundedRadii {
                            tl: 9.5,
                            tr: 9.5,
                            br: 9.5,
                            bl: 9.5,
                        },
                    },
                    Brush::Solid(bc),
                    DOCK_ITEM_Z - 1,
                );
            }

            // Icon (centered circle with symbol)
            let icon_cx = item_x + ITEM_W / 2.0;
            let icon_cy = items_y + 24.0;
            let icon_color = if entry.is_terminal {
                TERMINAL_ICON_COLOR
            } else {
                BROWSER_ICON_COLOR
            };
            canvas.circle(
                [icon_cx, icon_cy],
                ICON_SIZE / 2.0,
                Brush::Solid(icon_color),
                DOCK_ITEM_Z + 2,
            );

            // Icon symbol text (measure for centering)
            let symbol = if entry.is_terminal { "> _" } else { "</>" };
            let sym_size = 10.0;
            let sym_w = canvas.measure_text_width(symbol, sym_size);
            let sym_x = icon_cx - sym_w * 0.5;
            let sym_y = icon_cy + sym_size * 0.35; // baseline offset
            canvas.draw_text_run_weighted(
                [sym_x, sym_y],
                symbol.to_string(),
                sym_size,
                700.0,
                white,
                DOCK_ITEM_Z + 3,
            );

            // Label (measured and centered)
            let label_w = canvas.measure_text_width(&entry.title, LABEL_FONT_SIZE);
            let label_x = item_x + (ITEM_W - label_w) * 0.5;
            let label_y = items_y + ITEM_H - 8.0; // baseline near bottom
            canvas.draw_text_run(
                [label_x, label_y],
                entry.title.clone(),
                LABEL_FONT_SIZE,
                LABEL_COLOR,
                DOCK_LABEL_Z,
            );

            // Hit region for clicking this entry (focus window)
            let entry_region = ENTRY_HIT_REGION_BASE + i as u32;
            canvas.hit_region_rect(entry_region, item_rect, DOCK_ITEM_Z + 5);
        }
    }

    /// Hit-test a click at (x, y) against dock hit regions.
    /// Returns a `DockAction` if the click was on a dock element.
    pub fn hit_test(&self, region_id: u32, entries: &[DockEntry]) -> Option<DockAction> {
        if !self.visible {
            return None;
        }

        if region_id == SCRIM_HIT_REGION {
            return Some(DockAction::Dismiss);
        }
        if region_id == PANEL_HIT_REGION {
            // Absorb click on panel itself
            return None;
        }

        // Check entry regions (focus window)
        if (ENTRY_HIT_REGION_BASE..ENTRY_HIT_REGION_BASE + 100).contains(&region_id) {
            let idx = (region_id - ENTRY_HIT_REGION_BASE) as usize;
            if let Some(entry) = entries.get(idx) {
                return Some(DockAction::FocusWindow(entry.window_id));
            }
        }

        None
    }
}

// ── Hit region IDs ─────────────────────────────────────────────────
// These must not collide with other region IDs in the compositor.
// Using high values (50000+) to avoid collision with app-level regions.
pub const SCRIM_HIT_REGION: u32 = 50000;
pub const PANEL_HIT_REGION: u32 = 50001;
pub const ENTRY_HIT_REGION_BASE: u32 = 50100;
