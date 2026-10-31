//! The power menu: Sleep, Restart, Shut down and Log out, opened from the right end
//! of the shell bar.

use std::io;
use std::process::Command;

use jag_draw::{Brush, FontStyle, Rect, RoundedRadii, RoundedRect, SvgStyle};

use crate::palette::{self, color};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerAction {
    Sleep,
    Restart,
    ShutDown,
    LogOut,
}

impl PowerAction {
    pub const ALL: [PowerAction; 4] = [
        PowerAction::Sleep,
        PowerAction::Restart,
        PowerAction::ShutDown,
        PowerAction::LogOut,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PowerAction::Sleep => "Sleep",
            PowerAction::Restart => "Restart",
            PowerAction::ShutDown => "Shut down",
            PowerAction::LogOut => "Log out",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            PowerAction::Sleep => "power-sleep.svg",
            PowerAction::Restart => "power-restart.svg",
            PowerAction::ShutDown => "power-off.svg",
            PowerAction::LogOut => "power-logout.svg",
        }
    }

    /// The program and arguments that do it. Log out asks labwc, which ends the
    /// session and returns to the login screen.
    pub fn command(self) -> (&'static str, &'static [&'static str]) {
        match self {
            PowerAction::Sleep => ("systemctl", &["suspend"]),
            PowerAction::Restart => ("systemctl", &["reboot"]),
            PowerAction::ShutDown => ("systemctl", &["poweroff"]),
            PowerAction::LogOut => ("labwc", &["--exit"]),
        }
    }

    pub fn run(self) -> io::Result<()> {
        let (program, args) = self.command();
        let status = Command::new(program).args(args).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!("{program} exited with {status}")))
        }
    }
}

// Above the launcher (500_000).
const MENU_Z: i32 = 505_000;

const MENU_W: f32 = 200.0;
const ROW_H: f32 = 40.0;
const PAD: f32 = 6.0;
/// Space between the bar and the menu, and between the menu and the screen's right edge.
const BAR_GAP: f32 = 6.0;
const RIGHT_GAP: f32 = 14.0;
const CORNER: f32 = 12.0;
const ROW_CORNER: f32 = 8.0;
const ICON_SIZE: f32 = 18.0;
const ICON_STROKE: f32 = 1.6;
const TEXT_SIZE: f32 = 14.0;
const TEXT_X: f32 = 44.0;
const ICON_X: f32 = 14.0;
const BASELINE_NUDGE: f32 = -2.0;

#[derive(Default)]
pub struct PowerMenu {
    pub visible: bool,
}

fn round(rect: Rect, radius: f32) -> RoundedRect {
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

impl PowerMenu {
    /// The menu's rect, hanging below a bar drawn in `bar`, at the right edge.
    pub fn rect(bar: Rect) -> Rect {
        Rect {
            x: bar.x + bar.w - RIGHT_GAP - MENU_W,
            y: bar.y + bar.h + BAR_GAP,
            w: MENU_W,
            h: PAD * 2.0 + ROW_H * PowerAction::ALL.len() as f32,
        }
    }

    fn row_rect(menu: Rect, index: usize) -> Rect {
        Rect {
            x: menu.x + PAD,
            y: menu.y + PAD + index as f32 * ROW_H,
            w: menu.w - PAD * 2.0,
            h: ROW_H,
        }
    }

    /// The action whose row contains `(x, y)`.
    pub fn action_at(bar: Rect, x: f32, y: f32) -> Option<PowerAction> {
        let menu = Self::rect(bar);
        PowerAction::ALL.iter().enumerate().find_map(|(i, action)| {
            let r = Self::row_rect(menu, i);
            (x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h).then_some(*action)
        })
    }

    pub fn render(&self, canvas: &mut jag_surface::Canvas, bar: Rect, pointer: (f32, f32)) {
        if !self.visible {
            return;
        }
        let pal = palette::get();
        let menu = Self::rect(bar);
        canvas.rounded_rect(
            round(
                Rect {
                    x: menu.x - 1.0,
                    y: menu.y - 1.0,
                    w: menu.w + 2.0,
                    h: menu.h + 2.0,
                },
                CORNER + 1.0,
            ),
            Brush::Solid(color(pal.dock_border)),
            MENU_Z,
        );
        canvas.rounded_rect(
            round(menu, CORNER),
            Brush::Solid(color(pal.dock)),
            MENU_Z + 1,
        );
        let hovered = Self::action_at(bar, pointer.0, pointer.1);
        for (i, action) in PowerAction::ALL.iter().enumerate() {
            let row = Self::row_rect(menu, i);
            if hovered == Some(*action) {
                canvas.rounded_rect(
                    round(row, ROW_CORNER),
                    Brush::Solid(color(pal.selection)),
                    MENU_Z + 2,
                );
            }
            canvas.draw_svg_styled(
                action.icon(),
                [row.x + ICON_X - PAD, row.y + (ROW_H - ICON_SIZE) * 0.5],
                [ICON_SIZE, ICON_SIZE],
                SvgStyle::new()
                    .with_stroke(color(pal.text))
                    .with_stroke_width(ICON_STROKE),
                MENU_Z + 3,
            );
            canvas.draw_text_run_styled(
                [
                    row.x + TEXT_X - PAD,
                    row.y + ROW_H * 0.5 + TEXT_SIZE * 0.5 + BASELINE_NUDGE,
                ],
                action.label().to_string(),
                TEXT_SIZE,
                500.0,
                FontStyle::Normal,
                None,
                color(pal.text),
                MENU_Z + 3,
            );
        }
    }
}
