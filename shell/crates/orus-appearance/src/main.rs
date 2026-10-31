//! `orus-appearance`: read and change the Orus OS appearance setting.
//!
//! The setting is `appearance.json` in `~/.orus`, where the shell reads it too.
//! Changing the theme also rewrites the user's labwc config and asks the running
//! labwc to reload it.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use orus_core::appearance::{
    self, Accent, Appearance, Theme, Wallpaper, accent_color, selection_color,
};
use orus_core::{display, input};
use serde::Serialize;

const USAGE: &str = "usage:
  orus-appearance get                          print the setting as JSON
  orus-appearance set theme dark|light
  orus-appearance set accent teal|blue|purple|orange|rose
  orus-appearance set reduce-motion on|off
  orus-appearance set wallpaper auto|dunes|dunes-light
  orus-appearance apply-session                write the user's labwc config (run at session start)
  orus-appearance input get                    print the mouse and touchpad settings as JSON
  orus-appearance input set touchpad.tap|touchpad.natural-scroll|touchpad.disable-while-typing|
                             mouse.natural-scroll|mouse.left-handed|mouse.flat-acceleration on|off
  orus-appearance input set touchpad.speed|mouse.speed -1..1
  orus-appearance display get                  print the remembered screen settings as JSON
  orus-appearance display set OUTPUT [--mode|--custom-mode WIDTHxHEIGHT[@HERTZ]] [--scale N] [--pos X,Y]
                               [--transform normal|90|180|270] [--on|--off]   apply now and at every login";

/// Where the system labwc config lives unless `ORUS_SESSION_DIR` says otherwise.
const SYSTEM_SESSION_DIR: &str = "/usr/share/orus/session";

/// What `get` prints: the stored setting plus the colors that follow from it, so a
/// program that draws its own UI (System Settings) uses the same ones as the shell.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    #[serde(flatten)]
    appearance: Appearance,
    /// The wallpaper in use: the chosen one, else the theme's own.
    effective_wallpaper: Wallpaper,
    colors: Colors,
    /// Every accent's color for the current theme, for drawing the swatches.
    swatches: BTreeMap<&'static str, String>,
}

#[derive(Serialize)]
struct Colors {
    accent: String,
    selection: String,
}

impl From<Appearance> for Report {
    fn from(appearance: Appearance) -> Self {
        let hex = |rgb: u32| format!("#{rgb:06x}");
        Report {
            effective_wallpaper: appearance.effective_wallpaper(),
            swatches: Accent::ALL
                .iter()
                .map(|a| (a.as_str(), hex(accent_color(appearance.theme, *a))))
                .collect(),
            colors: Colors {
                accent: hex(accent_color(appearance.theme, appearance.accent)),
                selection: hex(selection_color(appearance.theme, appearance.accent)),
            },
            appearance,
        }
    }
}

fn data_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join(".orus")
}

fn session_dir() -> PathBuf {
    std::env::var_os("ORUS_SESSION_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(SYSTEM_SESSION_DIR))
}

fn apply_session(data_dir: &std::path::Path, theme: Theme) -> Result<(), String> {
    let system = session_dir();
    let input = input::load(data_dir).map_err(|e| e.to_string())?;
    let display = display::load(data_dir).map_err(|e| e.to_string())?;
    appearance::write_session_config(&system, &data_dir.join("labwc"), theme, &input, &display)
        .map_err(|e| {
            format!(
                "could not write the labwc config from {}: {e}",
                system.display()
            )
        })
}

/// Apply `--mode`, `--scale`, `--pos`, `--transform` and `--on`/`--off` to `setting`.
fn display_options(setting: &mut display::OutputSetting, options: &[&str]) -> Result<(), String> {
    let mut it = options.iter();
    while let Some(flag) = it.next() {
        let mut value = || {
            it.next()
                .map(|v| v.to_string())
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match *flag {
            "--mode" => {
                setting.mode = Some(value()?);
                setting.custom_mode = false;
            }
            "--custom-mode" => {
                setting.mode = Some(value()?);
                setting.custom_mode = true;
            }
            "--scale" => {
                let text = value()?;
                setting.scale = text
                    .parse()
                    .map_err(|_| format!("{text} is not a scale; use a number from 0.5 to 4"))?;
            }
            "--pos" => {
                let text = value()?;
                let parsed = text
                    .split_once(',')
                    .and_then(|(x, y)| Some((x.parse().ok()?, y.parse().ok()?)));
                setting.position =
                    Some(parsed.ok_or_else(|| format!("{text} is not a position; use X,Y"))?);
            }
            "--transform" => setting.transform = value()?,
            "--on" => setting.enabled = true,
            "--off" => setting.enabled = false,
            other => return Err(format!("unknown display option {other}")),
        }
    }
    Ok(())
}

/// Ask the running labwc to read its config again. The setting is saved either way, so
/// a labwc that cannot reload is reported, not fatal.
fn reconfigure_labwc() {
    match Command::new("labwc").arg("--reconfigure").status() {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!("orus-appearance: labwc --reconfigure exited with {status}"),
        Err(e) => eprintln!("orus-appearance: could not run labwc --reconfigure: {e}"),
    }
}

fn change(appearance: &mut Appearance, key: &str, value: &str) -> Result<(), String> {
    let bad = || format!("{value} is not a valid {key}");
    match key {
        "theme" => appearance.theme = Theme::parse(value).ok_or_else(bad)?,
        "accent" => appearance.accent = Accent::parse(value).ok_or_else(bad)?,
        "reduce-motion" => {
            appearance.reduce_motion = match value {
                "on" => true,
                "off" => false,
                _ => return Err(bad()),
            }
        }
        "wallpaper" => {
            appearance.wallpaper = match value {
                "auto" => None,
                name => Some(Wallpaper::parse(name).ok_or_else(bad)?),
            }
        }
        _ => return Err(format!("unknown setting {key}")),
    }
    Ok(())
}

fn run(args: &[&str]) -> Result<(), String> {
    let data_dir = data_dir();
    match args {
        ["get"] => {
            let current = appearance::load(&data_dir).map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string(&Report::from(current)).map_err(|e| e.to_string())?
            );
            Ok(())
        }
        ["set", key, value] => {
            let mut current = appearance::load(&data_dir).map_err(|e| e.to_string())?;
            let before = current.theme;
            change(&mut current, key, value)?;
            appearance::save(&data_dir, &current).map_err(|e| e.to_string())?;
            if current.theme != before {
                apply_session(&data_dir, current.theme)?;
                reconfigure_labwc();
            }
            Ok(())
        }
        ["input", "get"] => {
            let current = input::load(&data_dir).map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string(&current).map_err(|e| e.to_string())?
            );
            Ok(())
        }
        ["input", "set", key, value] => {
            let mut current = input::load(&data_dir).map_err(|e| e.to_string())?;
            current.set(key, value).map_err(|e| e.to_string())?;
            input::save(&data_dir, &current).map_err(|e| e.to_string())?;
            let theme = appearance::load(&data_dir)
                .map_err(|e| e.to_string())?
                .theme;
            apply_session(&data_dir, theme)?;
            reconfigure_labwc();
            Ok(())
        }
        ["display", "get"] => {
            let current = display::load(&data_dir).map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string(&current).map_err(|e| e.to_string())?
            );
            Ok(())
        }
        ["display", "set", output, options @ ..] => {
            let mut current = display::load(&data_dir).map_err(|e| e.to_string())?;
            let mut setting = current.outputs.get(*output).cloned().unwrap_or_default();
            display_options(&mut setting, options)?;
            current
                .set(output, setting.clone())
                .map_err(|e| e.to_string())?;
            // Apply first: a setting the screen refuses is not remembered.
            let status = Command::new("wlr-randr")
                .args(setting.randr_args(output))
                .status()
                .map_err(|e| format!("could not run wlr-randr: {e}"))?;
            if !status.success() {
                return Err(format!("wlr-randr exited with {status}"));
            }
            display::save(&data_dir, &current).map_err(|e| e.to_string())?;
            let theme = appearance::load(&data_dir)
                .map_err(|e| e.to_string())?
                .theme;
            apply_session(&data_dir, theme)
        }
        ["apply-session"] => {
            let current = appearance::load(&data_dir).map_err(|e| e.to_string())?;
            apply_session(&data_dir, current.theme)
        }
        _ => Err(USAGE.to_string()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) if message == USAGE => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
        Err(message) => {
            eprintln!("orus-appearance: {message}");
            ExitCode::FAILURE
        }
    }
}
