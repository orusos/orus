//! The appearance setting shared by the shell, the window theme and apps:
//! one JSON file, `appearance.json`, in the Orus OS data directory.

use crate::display::Display;
use crate::input::{Input, with_input};
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "appearance.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "dark" => Some(Theme::Dark),
            "light" => Some(Theme::Light),
            _ => None,
        }
    }

    /// Name of the labwc window theme for this theme.
    pub fn labwc_theme(self) -> &'static str {
        match self {
            Theme::Dark => "orus",
            Theme::Light => "orus-light",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Accent {
    #[default]
    Teal,
    Blue,
    Purple,
    Orange,
    Rose,
}

impl Accent {
    pub const ALL: [Accent; 5] = [
        Accent::Teal,
        Accent::Blue,
        Accent::Purple,
        Accent::Orange,
        Accent::Rose,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Accent::Teal => "teal",
            Accent::Blue => "blue",
            Accent::Purple => "purple",
            Accent::Orange => "orange",
            Accent::Rose => "rose",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "teal" => Some(Accent::Teal),
            "blue" => Some(Accent::Blue),
            "purple" => Some(Accent::Purple),
            "orange" => Some(Accent::Orange),
            "rose" => Some(Accent::Rose),
            _ => None,
        }
    }
}

/// Window content color for `theme`; selections are drawn on it.
pub fn surface_color(theme: Theme) -> u32 {
    match theme {
        Theme::Dark => 0x1a2125,
        Theme::Light => 0xfdfdfe,
    }
}

/// The accent as sRGB hex, from the swatches in the dark and light desktop concepts.
pub fn accent_color(theme: Theme, accent: Accent) -> u32 {
    const DARK: [u32; 5] = [0x63b1ac, 0x5182ba, 0x7f6aaf, 0xd29872, 0xc66b7e];
    const LIGHT: [u32; 5] = [0x1f8285, 0x538dd9, 0x8c65cd, 0xe29c6d, 0xe06e8f];
    let i = match accent {
        Accent::Teal => 0,
        Accent::Blue => 1,
        Accent::Purple => 2,
        Accent::Orange => 3,
        Accent::Rose => 4,
    };
    match theme {
        Theme::Dark => DARK[i],
        Theme::Light => LIGHT[i],
    }
}

/// The selected row or tile: the accent mixed into the surface, at the share
/// measured in the concepts (about 27% on dark, 16% on light).
pub fn selection_color(theme: Theme, accent: Accent) -> u32 {
    let (share, base) = match theme {
        Theme::Dark => (0.27_f32, surface_color(theme)),
        Theme::Light => (0.16_f32, surface_color(theme)),
    };
    let over = accent_color(theme, accent);
    let channel = |shift: u32| {
        let o = ((over >> shift) & 0xff) as f32;
        let b = ((base >> shift) & 0xff) as f32;
        (o * share + b * (1.0 - share)).round() as u32
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

/// A wallpaper that ships with Orus OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Wallpaper {
    #[serde(rename = "dunes")]
    Dunes,
    #[serde(rename = "dunes-light")]
    DunesLight,
}

impl Wallpaper {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "dunes" => Some(Wallpaper::Dunes),
            "dunes-light" => Some(Wallpaper::DunesLight),
            _ => None,
        }
    }

    /// File name inside the wallpapers directory.
    pub fn file(self) -> &'static str {
        match self {
            Wallpaper::Dunes => "orus-dunes.png",
            Wallpaper::DunesLight => "orus-dunes-light.png",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Appearance {
    pub theme: Theme,
    pub accent: Accent,
    pub reduce_motion: bool,
    /// `None` follows the theme: dunes for dark, light dunes for light.
    pub wallpaper: Option<Wallpaper>,
}

impl Appearance {
    /// The wallpaper in use: the chosen one, else the theme's own.
    pub fn effective_wallpaper(&self) -> Wallpaper {
        self.wallpaper.unwrap_or(match self.theme {
            Theme::Dark => Wallpaper::Dunes,
            Theme::Light => Wallpaper::DunesLight,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppearanceError {
    #[error("appearance.json is not valid: {0}")]
    Invalid(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// Read the setting from `data_dir`. A missing file is the default appearance.
pub fn load(data_dir: &Path) -> Result<Appearance, AppearanceError> {
    match fs::read_to_string(data_dir.join(FILE_NAME)) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Appearance::default()),
        Err(e) => Err(e.into()),
    }
}

/// Write the setting to `data_dir`, replacing the file in one step so a reader
/// never sees a half-written one.
pub fn save(data_dir: &Path, appearance: &Appearance) -> Result<(), AppearanceError> {
    fs::create_dir_all(data_dir)?;
    let temp = data_dir.join(format!("{FILE_NAME}.tmp"));
    fs::write(&temp, serde_json::to_string_pretty(appearance)?)?;
    fs::rename(temp, data_dir.join(FILE_NAME))?;
    Ok(())
}

/// `rc` with the labwc theme name set to `name`; `None` if `rc` has no `<theme><name>`.
pub fn with_theme_name(rc: &str, name: &str) -> Option<String> {
    let theme = rc.find("<theme>")?;
    let open = theme + rc[theme..].find("<name>")? + "<name>".len();
    let close = open + rc[open..].find("</name>")?;
    Some(format!("{}{}{}", &rc[..open], name, &rc[close..]))
}

/// Write the user's labwc config: the system `rc.xml` with the theme of `theme` and
/// the mouse and touchpad settings, and an `autostart` that restores the screen
/// settings. labwc is started on this directory so these can change without touching
/// the system files.
pub fn write_session_config(
    system_dir: &Path,
    user_dir: &Path,
    theme: Theme,
    input: &Input,
    display: &Display,
) -> Result<(), AppearanceError> {
    let rc = fs::read_to_string(system_dir.join("rc.xml"))?;
    let themed = with_theme_name(&rc, theme.labwc_theme())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "rc.xml has no <theme><name>"))?;
    let configured = with_input(&themed, input).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "rc.xml has no </labwc_config>")
    })?;
    fs::create_dir_all(user_dir)?;
    fs::write(user_dir.join("rc.xml"), configured)?;
    fs::write(user_dir.join("autostart"), display.autostart())?;
    Ok(())
}
