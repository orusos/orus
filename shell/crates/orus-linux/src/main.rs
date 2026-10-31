//! Orus OS Universal — Linux launcher.
//!
//! With no arguments it is the desktop (wallpaper) plus its panel (`--panel`: bar, dock,
//! launcher). `--toggle-launcher` opens or closes the launcher (the keybinding runs it).
//! `--app <name> --size <w>x<h>` runs one app
//! (terminal, files, textedit, viewer) as a window of its own.

mod platform;

use anyhow::{Result, bail};
use orus_shell::app_kind::AppKind;
use orus_shell::runtime;
use platform::LinuxPlatformHooks;

/// Window size when `--size` is not given.
const DEFAULT_APP_SIZE: (f64, f64) = (1000.0, 640.0);

fn parse_size(text: &str) -> Option<(f64, f64)> {
    let (w, h) = text.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

/// The desktop: the wallpaper window, with the panel (bar, dock, launcher) running
/// beside it as its own process. The panel is stopped when the desktop exits.
fn run_desktop() -> Result<()> {
    let mut panel = std::process::Command::new(std::env::current_exe()?)
        .arg("--panel")
        .spawn()?;
    let result = runtime::run_desktop(LinuxPlatformHooks);
    let _ = panel.kill();
    let _ = panel.wait();
    result
}

fn main() -> Result<()> {
    env_logger::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut app = None;
    let mut panel = false;
    let mut size = DEFAULT_APP_SIZE;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--app" => {
                let name = rest.next().map(String::as_str).unwrap_or("");
                app = Some(match AppKind::parse(name) {
                    Some(kind) => kind,
                    None => {
                        bail!("unknown app \"{name}\" (terminal, files, textedit, viewer)")
                    }
                });
            }
            "--panel" => panel = true,
            "--toggle-launcher" => return orus_panel::toggle_launcher(),
            "--size" => {
                let text = rest.next().map(String::as_str).unwrap_or("");
                size = match parse_size(text) {
                    Some(size) => size,
                    None => bail!("--size needs WIDTHxHEIGHT, got \"{text}\""),
                };
            }
            other => bail!("unknown argument {other}"),
        }
    }
    if panel {
        return orus_panel::run();
    }
    match app {
        Some(kind) => runtime::run_app(LinuxPlatformHooks, kind, size),
        None => run_desktop(),
    }
}
