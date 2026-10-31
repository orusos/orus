//! Mouse and touchpad settings: `input.json` in the Orus OS data directory, written
//! into the user's labwc config as `<libinput>` rules.

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "input.json";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Touchpad {
    pub tap_to_click: bool,
    pub natural_scroll: bool,
    /// -1.0 (slowest) to 1.0 (fastest), as libinput takes it.
    pub pointer_speed: f32,
    pub disable_while_typing: bool,
}

impl Default for Touchpad {
    fn default() -> Self {
        Self {
            tap_to_click: true,
            natural_scroll: true,
            pointer_speed: 0.0,
            disable_while_typing: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Mouse {
    pub natural_scroll: bool,
    /// -1.0 (slowest) to 1.0 (fastest), as libinput takes it.
    pub pointer_speed: f32,
    pub left_handed: bool,
    /// A constant pointer speed instead of one that grows with how fast the mouse moves.
    pub flat_acceleration: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Input {
    pub touchpad: Touchpad,
    pub mouse: Mouse,
}

#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("input.json is not valid: {0}")]
    Invalid(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("{0}")]
    Setting(String),
}

fn on_off(value: &str, key: &str) -> Result<bool, InputError> {
    match value {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(InputError::Setting(format!(
            "{value} is not valid for {key}; use on or off"
        ))),
    }
}

fn speed(value: &str, key: &str) -> Result<f32, InputError> {
    match value.parse::<f32>() {
        Ok(v) if (-1.0..=1.0).contains(&v) => Ok(v),
        _ => Err(InputError::Setting(format!(
            "{value} is not valid for {key}; use a number from -1 to 1"
        ))),
    }
}

impl Input {
    /// Change the setting named `key` (`touchpad.tap`, `mouse.speed`, ...) to `value`.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), InputError> {
        match key {
            "touchpad.tap" => self.touchpad.tap_to_click = on_off(value, key)?,
            "touchpad.natural-scroll" => self.touchpad.natural_scroll = on_off(value, key)?,
            "touchpad.speed" => self.touchpad.pointer_speed = speed(value, key)?,
            "touchpad.disable-while-typing" => {
                self.touchpad.disable_while_typing = on_off(value, key)?
            }
            "mouse.natural-scroll" => self.mouse.natural_scroll = on_off(value, key)?,
            "mouse.speed" => self.mouse.pointer_speed = speed(value, key)?,
            "mouse.left-handed" => self.mouse.left_handed = on_off(value, key)?,
            "mouse.flat-acceleration" => self.mouse.flat_acceleration = on_off(value, key)?,
            _ => return Err(InputError::Setting(format!("unknown setting {key}"))),
        }
        Ok(())
    }

    /// The labwc `<libinput>` element for these settings. labwc's `touchpad` category
    /// is touchpads; `non-touch` is mice and other pointers.
    pub fn labwc_xml(&self) -> String {
        let yes = |on: bool| if on { "yes" } else { "no" };
        let t = &self.touchpad;
        let m = &self.mouse;
        format!(
            "  <libinput>\n\
             \x20   <device category=\"touchpad\">\n\
             \x20     <tap>{}</tap>\n\
             \x20     <naturalScroll>{}</naturalScroll>\n\
             \x20     <pointerSpeed>{:.2}</pointerSpeed>\n\
             \x20     <disableWhileTyping>{}</disableWhileTyping>\n\
             \x20   </device>\n\
             \x20   <device category=\"non-touch\">\n\
             \x20     <naturalScroll>{}</naturalScroll>\n\
             \x20     <pointerSpeed>{:.2}</pointerSpeed>\n\
             \x20     <leftHanded>{}</leftHanded>\n\
             \x20     <accelProfile>{}</accelProfile>\n\
             \x20   </device>\n\
             \x20 </libinput>\n",
            yes(t.tap_to_click),
            yes(t.natural_scroll),
            t.pointer_speed,
            yes(t.disable_while_typing),
            yes(m.natural_scroll),
            m.pointer_speed,
            yes(m.left_handed),
            if m.flat_acceleration {
                "flat"
            } else {
                "adaptive"
            },
        )
    }
}

/// `rc` with `input`'s `<libinput>` element added before `</labwc_config>`; `None` if
/// `rc` has no such closing tag.
pub fn with_input(rc: &str, input: &Input) -> Option<String> {
    let end = rc.rfind("</labwc_config>")?;
    Some(format!("{}{}{}", &rc[..end], input.labwc_xml(), &rc[end..]))
}

/// Read the setting from `data_dir`. A missing file is the default.
pub fn load(data_dir: &Path) -> Result<Input, InputError> {
    match fs::read_to_string(data_dir.join(FILE_NAME)) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Input::default()),
        Err(e) => Err(e.into()),
    }
}

pub fn save(data_dir: &Path, input: &Input) -> Result<(), InputError> {
    fs::create_dir_all(data_dir)?;
    let temp = data_dir.join(format!("{FILE_NAME}.tmp"));
    fs::write(&temp, serde_json::to_string_pretty(input)?)?;
    fs::rename(temp, data_dir.join(FILE_NAME))?;
    Ok(())
}
