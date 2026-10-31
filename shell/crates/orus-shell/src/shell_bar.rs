//! Orus OS shell bar: the slim strip at the top of the screen.
//!
//! Left: the K mark (opens the launcher) and the focused app's name.
//! Center: the date. Right: agent status, network, battery and the clock.

use std::time::{Duration, Instant};

use chrono::Local;
use jag_draw::{Brush, ColorLinPremul, FontStyle, Rect, RoundedRadii, RoundedRect, SvgStyle};

use orus_core::appearance::Theme;

use crate::palette::{self, color};
use crate::system_status::{Network, SystemStatus};

/// Z-index for shell bar background and content.
/// Sits above all application content (viewport z=0) and above the
/// Detir toolbar (z=9000) if one were present.
const SHELL_BAR_Z: i32 = 9500;

/// Z-index for shell bar text and icons (slightly above background).
const SHELL_BAR_TEXT_Z: i32 = 9510;

/// Font size for shell bar text in logical pixels.
const FONT_SIZE: f32 = 15.0;

/// Horizontal padding inside the shell bar.
const H_PADDING: f32 = 30.0;

/// Width of the clickable area at the left end of the bar, in logical pixels.
const APPS_BUTTON_WIDTH: f32 = 56.0;

/// Where the app name starts, measured from the left edge of the bar.
const NAME_X: f32 = 60.0;

/// Corner radius of the battery's charge fill.
const BATTERY_FILL_RADIUS: f32 = 1.0;

/// Width of the clickable area at the right end of the bar (network, battery and
/// clock), which opens the power menu.
const STATUS_AREA_WIDTH: f32 = 200.0;

/// The K mark's size in logical pixels.
const MARK_W: f32 = 18.0;
const MARK_H: f32 = 21.0;

/// Side of a status icon, in logical pixels.
const STATUS_ICON: f32 = 18.0;

/// Gap between items on the right of the bar.
const ITEM_GAP: f32 = 22.0;

/// Stroke width of the status icons.
const STATUS_STROKE: f32 = 1.5;

/// The battery icon's body in its 24-unit art: the inside of the outline, where
/// the charge level is filled.
const BATTERY_BODY_X: f32 = 4.0;
const BATTERY_BODY_Y: f32 = 8.0;
const BATTERY_BODY_W: f32 = 12.0;
const BATTERY_BODY_H: f32 = 8.0;
const ICON_UNITS: f32 = 24.0;

/// A battery at or below this percent is drawn in the warning color.
const BATTERY_LOW_PERCENT: u8 = 20;

/// How often the network and battery state is read again.
const STATUS_REFRESH: Duration = Duration::from_secs(5);

/// Warning color for a low battery.
const WARNING: u32 = 0xe5534b;

/// Agent connection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentStatus {
    /// No active agent session.
    #[default]
    Idle,
    /// Agent is connected and processing.
    Active,
    /// Agent connection lost.
    Disconnected,
}

impl AgentStatus {
    /// Short label for the status indicator.
    fn label(self) -> &'static str {
        match self {
            Self::Idle => "",
            Self::Active => "Agent",
            Self::Disconnected => "Offline",
        }
    }

    /// Indicator color (shown as a text tint).
    fn color(self) -> ColorLinPremul {
        match self {
            Self::Idle => color(palette::get().text),
            Self::Active => color(palette::get().accent),
            Self::Disconnected => color(WARNING),
        }
    }
}

/// Orus OS shell bar state and rendering.
pub struct ShellBar {
    /// Name of the currently focused application.
    pub app_name: String,
    /// Current agent connection status.
    pub agent_status: AgentStatus,
    status: SystemStatus,
    status_read_at: Instant,
}

impl Default for ShellBar {
    fn default() -> Self {
        Self::new("Orus OS")
    }
}

impl ShellBar {
    /// Create a new shell bar with the given app name.
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            agent_status: AgentStatus::default(),
            status: SystemStatus::read(),
            status_read_at: Instant::now(),
        }
    }

    /// Read the network and battery state again if the last read is old.
    pub fn refresh_status(&mut self) {
        if self.status_read_at.elapsed() >= STATUS_REFRESH {
            self.status = SystemStatus::read();
            self.status_read_at = Instant::now();
        }
    }

    /// Whether `(x, y)` is on the K mark area of a bar drawn in `rect`.
    pub fn apps_button_contains(rect: Rect, x: f32, y: f32) -> bool {
        x >= rect.x && x < rect.x + APPS_BUTTON_WIDTH && y >= rect.y && y < rect.y + rect.h
    }

    /// Whether `(x, y)` is on the right end of a bar drawn in `rect`.
    pub fn status_area_contains(rect: Rect, x: f32, y: f32) -> bool {
        x >= rect.x + rect.w - STATUS_AREA_WIDTH
            && x < rect.x + rect.w
            && y >= rect.y
            && y < rect.y + rect.h
    }

    fn icon(canvas: &mut jag_surface::Canvas, name: &str, x: f32, y: f32, rgba: ColorLinPremul) {
        canvas.draw_svg_styled(
            name,
            [x, y],
            [STATUS_ICON, STATUS_ICON],
            SvgStyle::new()
                .with_stroke(rgba)
                .with_stroke_width(STATUS_STROKE),
            SHELL_BAR_TEXT_Z,
        );
    }

    /// Render the shell bar into the canvas within the given rect.
    pub fn render(&self, canvas: &mut jag_surface::Canvas, rect: Rect) {
        let pal = palette::get();
        let text = color(pal.text);
        canvas.fill_rect(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Brush::Solid(color(pal.bar)),
            SHELL_BAR_Z,
        );
        canvas.fill_rect(
            rect.x,
            rect.y + rect.h - 1.0,
            rect.w,
            1.0,
            Brush::Solid(color(pal.bar_border)),
            SHELL_BAR_Z + 1,
        );

        // Baseline: the same offset the launcher's search box uses to center text.
        let text_y = rect.y + (rect.h + FONT_SIZE) * 0.5 - 2.0;
        let mid_y = rect.y + rect.h * 0.5;

        // --- Left: K mark, then the app name ---
        let mark_x = rect.x + H_PADDING;
        canvas.draw_svg(
            if pal.theme == Theme::Light {
                "orus-k-light.svg"
            } else {
                "orus-k.svg"
            },
            [mark_x, mid_y - MARK_H * 0.5],
            [MARK_W, MARK_H],
            SHELL_BAR_TEXT_Z,
        );
        canvas.draw_text_run_styled(
            [rect.x + NAME_X, text_y],
            self.app_name.clone(),
            FONT_SIZE,
            500.0,
            FontStyle::Normal,
            None,
            text,
            SHELL_BAR_TEXT_Z,
        );

        // --- Center: the date ---
        let now = Local::now();
        let date = now.format("%a, %-d %b").to_string();
        let date_w = canvas.measure_text_width(&date, FONT_SIZE);
        canvas.draw_text_run_styled(
            [rect.x + (rect.w - date_w) * 0.5, text_y],
            date,
            FONT_SIZE,
            500.0,
            FontStyle::Normal,
            None,
            text,
            SHELL_BAR_TEXT_Z,
        );

        // --- Right, from the edge inward: clock, battery, network, agent ---
        let mut x = rect.x + rect.w - H_PADDING;
        let clock = now.format("%H:%M").to_string();
        let clock_w = canvas.measure_text_width(&clock, FONT_SIZE);
        x -= clock_w;
        canvas.draw_text_run_styled(
            [x, text_y],
            clock,
            FONT_SIZE,
            500.0,
            FontStyle::Normal,
            None,
            text,
            SHELL_BAR_TEXT_Z,
        );

        let icon_y = mid_y - STATUS_ICON * 0.5;
        if let Some(battery) = self.status.battery {
            x -= ITEM_GAP + STATUS_ICON;
            if battery.charging {
                Self::icon(canvas, "status-battery-charging.svg", x, icon_y, text);
            } else {
                let low = battery.percent <= BATTERY_LOW_PERCENT;
                let tint = if low { color(WARNING) } else { text };
                Self::icon(canvas, "status-battery.svg", x, icon_y, tint);
                let scale = STATUS_ICON / ICON_UNITS;
                let level = f32::from(battery.percent) / 100.0;
                canvas.rounded_rect(
                    RoundedRect {
                        rect: Rect {
                            x: x + BATTERY_BODY_X * scale,
                            y: icon_y + BATTERY_BODY_Y * scale,
                            w: BATTERY_BODY_W * scale * level,
                            h: BATTERY_BODY_H * scale,
                        },
                        radii: RoundedRadii {
                            tl: BATTERY_FILL_RADIUS,
                            tr: BATTERY_FILL_RADIUS,
                            br: BATTERY_FILL_RADIUS,
                            bl: BATTERY_FILL_RADIUS,
                        },
                    },
                    Brush::Solid(tint),
                    SHELL_BAR_TEXT_Z,
                );
            }
        }

        x -= ITEM_GAP + STATUS_ICON;
        let network_icon = match self.status.network {
            Network::Wifi => "status-wifi.svg",
            Network::Wired => "status-ethernet.svg",
            Network::Offline => "status-wifi-off.svg",
        };
        Self::icon(canvas, network_icon, x, icon_y, text);

        // Agent status label, if there is one.
        let status_label = self.agent_status.label();
        if !status_label.is_empty() {
            let label_w = canvas.measure_text_width(status_label, FONT_SIZE);
            canvas.draw_text_run_styled(
                [x - ITEM_GAP - label_w, text_y],
                status_label.to_string(),
                FONT_SIZE,
                500.0,
                FontStyle::Normal,
                None,
                self.agent_status.color(),
                SHELL_BAR_TEXT_Z,
            );
        }
    }
}
