//! Screen mode and scale per output: `display.json` in the Orus OS data directory.
//! The choices are applied with `wlr-randr` and replayed from labwc's `autostart` at
//! the next login.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "display.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputSetting {
    /// `WIDTHxHEIGHT` or `WIDTHxHEIGHT@HERTZ`, as `wlr-randr --mode` takes it; `None`
    /// leaves the screen's own choice.
    pub mode: Option<String>,
    /// `mode` is not one the screen lists; `wlr-randr --custom-mode` asks the compositor
    /// to try it. Virtual screens (a VM's) take any size; a real panel may refuse.
    pub custom_mode: bool,
    pub scale: f32,
    pub enabled: bool,
    /// Top-left corner in the shared layout, in logical pixels; `None` leaves it as it is.
    pub position: Option<(i32, i32)>,
    /// `normal`, `90`, `180` or `270`, as `wlr-randr --transform` takes it.
    pub transform: String,
}

impl Default for OutputSetting {
    fn default() -> Self {
        Self {
            mode: None,
            custom_mode: false,
            scale: 1.0,
            enabled: true,
            position: None,
            transform: "normal".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Display {
    pub outputs: BTreeMap<String, OutputSetting>,
}

#[derive(Debug, thiserror::Error)]
pub enum DisplayError {
    #[error("display.json is not valid: {0}")]
    Invalid(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("{0}")]
    Setting(String),
}

/// Output names go into a shell script, so only plain characters pass.
fn valid_output(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn valid_mode(mode: &str) -> bool {
    let (size, hertz) = match mode.split_once('@') {
        Some((size, hertz)) => (size, Some(hertz)),
        None => (mode, None),
    };
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let in_range = |s: &str| {
        s.parse::<u32>()
            .is_ok_and(|v| (MIN_SIDE..=MAX_SIDE).contains(&v))
    };
    let size_ok = size
        .split_once('x')
        .is_some_and(|(w, h)| digits(w) && digits(h) && in_range(w) && in_range(h));
    let hertz_ok = hertz.is_none_or(|h| h.parse::<f32>().is_ok_and(|v| v > 0.0 && v < 1000.0));
    size_ok && hertz_ok
}

/// The shortest and longest side of a screen mode accepted.
const MIN_SIDE: u32 = 320;
const MAX_SIDE: u32 = 16_384;

/// The largest coordinate accepted; real layouts are far smaller.
const MAX_COORDINATE: i32 = 100_000;

impl OutputSetting {
    /// Refuses values `wlr-randr` would not take, so they are safe in a shell line.
    pub fn check(&self) -> Result<(), DisplayError> {
        if let Some(mode) = &self.mode
            && !valid_mode(mode)
        {
            return Err(DisplayError::Setting(format!(
                "{mode} is not a mode; use WIDTHxHEIGHT or WIDTHxHEIGHT@HERTZ"
            )));
        }
        if !(0.5..=4.0).contains(&self.scale) {
            return Err(DisplayError::Setting(format!(
                "{} is not a scale; use a number from 0.5 to 4",
                self.scale
            )));
        }
        if !matches!(self.transform.as_str(), "normal" | "90" | "180" | "270") {
            return Err(DisplayError::Setting(format!(
                "{} is not a rotation; use normal, 90, 180 or 270",
                self.transform
            )));
        }
        if let Some((x, y)) = self.position
            && (x.abs() > MAX_COORDINATE || y.abs() > MAX_COORDINATE)
        {
            return Err(DisplayError::Setting(format!(
                "position {x},{y} is out of range"
            )));
        }
        Ok(())
    }

    /// The `wlr-randr` arguments that apply this setting to `output`.
    pub fn randr_args(&self, output: &str) -> Vec<String> {
        let mut args = vec!["--output".to_string(), output.to_string()];
        args.push(if self.enabled { "--on" } else { "--off" }.into());
        if self.enabled {
            if let Some(mode) = &self.mode {
                let flag = if self.custom_mode {
                    "--custom-mode"
                } else {
                    "--mode"
                };
                args.extend([flag.into(), mode.clone()]);
            }
            args.extend(["--scale".into(), format!("{}", self.scale)]);
            args.extend(["--transform".into(), self.transform.clone()]);
            if let Some((x, y)) = self.position {
                args.extend(["--pos".into(), format!("{x},{y}")]);
            }
        }
        args
    }
}

impl Display {
    /// Remember `setting` for `output`.
    pub fn set(&mut self, output: &str, setting: OutputSetting) -> Result<(), DisplayError> {
        if !valid_output(output) {
            return Err(DisplayError::Setting(format!(
                "{output} is not an output name"
            )));
        }
        setting.check()?;
        self.outputs.insert(output.to_string(), setting);
        Ok(())
    }

    /// labwc's `autostart` script: one `wlr-randr` line per remembered output. Values
    /// were checked by `set`, so they are safe in a shell line.
    pub fn autostart(&self) -> String {
        let mut script = String::new();
        for (output, setting) in &self.outputs {
            script.push_str(&format!(
                "wlr-randr {} &\n",
                setting.randr_args(output).join(" ")
            ));
        }
        script
    }
}

/// Read the setting from `data_dir`. A missing file is no remembered outputs.
pub fn load(data_dir: &Path) -> Result<Display, DisplayError> {
    match fs::read_to_string(data_dir.join(FILE_NAME)) {
        Ok(text) => {
            let display: Display = serde_json::from_str(&text)?;
            // The file is hand-editable and ends up in a shell script: check it again.
            let mut checked = Display::default();
            for (output, setting) in display.outputs {
                checked.set(&output, setting)?;
            }
            Ok(checked)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Display::default()),
        Err(e) => Err(e.into()),
    }
}

pub fn save(data_dir: &Path, display: &Display) -> Result<(), DisplayError> {
    fs::create_dir_all(data_dir)?;
    let temp = data_dir.join(format!("{FILE_NAME}.tmp"));
    fs::write(&temp, serde_json::to_string_pretty(display)?)?;
    fs::rename(temp, data_dir.join(FILE_NAME))?;
    Ok(())
}
