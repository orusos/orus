//! Icons the login screen draws, embedded so it needs no files beside the binary.

use std::path::Path;

pub fn register() {
    jag_draw::set_builtin_svg_provider(bytes);
}

fn bytes(path: &Path) -> Option<&'static [u8]> {
    match path.file_name()?.to_str()? {
        "arrow-right.svg" => Some(include_bytes!("../assets/icons/arrow-right.svg")),
        "avatar.svg" => Some(include_bytes!("../assets/icons/avatar.svg")),
        "eye.svg" => Some(include_bytes!("../assets/icons/eye.svg")),
        "eye-off.svg" => Some(include_bytes!("../assets/icons/eye-off.svg")),
        "keyboard.svg" => Some(include_bytes!("../assets/icons/keyboard.svg")),
        "orus-k-login.svg" => Some(include_bytes!("../assets/icons/orus-k-login.svg")),
        "power.svg" => Some(include_bytes!("../assets/icons/power.svg")),
        // The network icons are the shell bar's.
        "status-ethernet.svg" => Some(include_bytes!(
            "../../orus-shell/assets/icons/status-ethernet.svg"
        )),
        "status-wifi.svg" => Some(include_bytes!(
            "../../orus-shell/assets/icons/status-wifi.svg"
        )),
        "status-wifi-off.svg" => Some(include_bytes!(
            "../../orus-shell/assets/icons/status-wifi-off.svg"
        )),
        _ => None,
    }
}
