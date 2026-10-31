//! Embedded SVG icons for the file manager and viewer.
//!
//! Registered with jag-draw's pluggable provider at startup, so the icons
//! render without an `images/` directory next to the binary.

use std::path::Path;

/// Register the embedded icons with jag-draw. Call once at startup.
pub fn register() {
    jag_draw::set_builtin_svg_provider(builtin_svg_bytes);
}

/// The bytes of the embedded icon named by the file name of `path`.
pub fn builtin_svg_bytes(path: &Path) -> Option<&'static [u8]> {
    let name = path.file_name()?.to_str()?;
    match name {
        "bash.svg" => Some(include_bytes!("../assets/icons/bash.svg")),
        "c.svg" => Some(include_bytes!("../assets/icons/c.svg")),
        "chevron-right.svg" => Some(include_bytes!("../assets/icons/chevron-right.svg")),
        "cplusplus.svg" => Some(include_bytes!("../assets/icons/cplusplus.svg")),
        "csharp.svg" => Some(include_bytes!("../assets/icons/csharp.svg")),
        "css.svg" => Some(include_bytes!("../assets/icons/css.svg")),
        "dock-textedit.svg" => Some(include_bytes!("../assets/icons/dock-textedit.svg")),
        "dock-textedit-light.svg" => {
            Some(include_bytes!("../assets/icons/dock-textedit-light.svg"))
        }
        "dock-viewer.svg" => Some(include_bytes!("../assets/icons/dock-viewer.svg")),
        "dock-viewer-light.svg" => Some(include_bytes!("../assets/icons/dock-viewer-light.svg")),
        "dock-app.svg" => Some(include_bytes!("../assets/icons/dock-app.svg")),
        "dock-app-light.svg" => Some(include_bytes!("../assets/icons/dock-app-light.svg")),
        "dock-office.svg" => Some(include_bytes!("../assets/icons/dock-office.svg")),
        "dock-office-light.svg" => Some(include_bytes!("../assets/icons/dock-office-light.svg")),
        "dock-store.svg" => Some(include_bytes!("../assets/icons/dock-store.svg")),
        "dock-store-light.svg" => Some(include_bytes!("../assets/icons/dock-store-light.svg")),
        "dock-install.svg" => Some(include_bytes!("../assets/icons/dock-install.svg")),
        "dock-install-light.svg" => Some(include_bytes!("../assets/icons/dock-install-light.svg")),
        "dock-browser.svg" => Some(include_bytes!("../assets/icons/dock-browser.svg")),
        "dock-files.svg" => Some(include_bytes!("../assets/icons/dock-files.svg")),
        "dock-launcher.svg" => Some(include_bytes!("../assets/icons/dock-launcher.svg")),
        "dock-settings.svg" => Some(include_bytes!("../assets/icons/dock-settings.svg")),
        "dock-terminal.svg" => Some(include_bytes!("../assets/icons/dock-terminal.svg")),
        "dock-browser-light.svg" => Some(include_bytes!("../assets/icons/dock-browser-light.svg")),
        "dock-files-light.svg" => Some(include_bytes!("../assets/icons/dock-files-light.svg")),
        "dock-launcher-light.svg" => {
            Some(include_bytes!("../assets/icons/dock-launcher-light.svg"))
        }
        "dock-settings-light.svg" => {
            Some(include_bytes!("../assets/icons/dock-settings-light.svg"))
        }
        "dock-terminal-light.svg" => {
            Some(include_bytes!("../assets/icons/dock-terminal-light.svg"))
        }
        "orus-k-light.svg" => Some(include_bytes!("../assets/icons/orus-k-light.svg")),
        "docker.svg" => Some(include_bytes!("../assets/icons/docker.svg")),
        "elixir.svg" => Some(include_bytes!("../assets/icons/elixir.svg")),
        "file.svg" => Some(include_bytes!("../assets/icons/file.svg")),
        "file-generic.svg" => Some(include_bytes!("../assets/icons/file-generic.svg")),
        "folder-open.svg" => Some(include_bytes!("../assets/icons/folder-open.svg")),
        "files-back.svg" => Some(include_bytes!("../assets/icons/files-back.svg")),
        "files-close.svg" => Some(include_bytes!("../assets/icons/files-close.svg")),
        "files-documents.svg" => Some(include_bytes!("../assets/icons/files-documents.svg")),
        "files-downloads.svg" => Some(include_bytes!("../assets/icons/files-downloads.svg")),
        "files-file-dark.svg" => Some(include_bytes!("../assets/icons/files-file-dark.svg")),
        "files-file-light.svg" => Some(include_bytes!("../assets/icons/files-file-light.svg")),
        "files-folder-doc-dark.svg" => {
            Some(include_bytes!("../assets/icons/files-folder-doc-dark.svg"))
        }
        "files-folder-doc-light.svg" => {
            Some(include_bytes!("../assets/icons/files-folder-doc-light.svg"))
        }
        "files-folder-download-dark.svg" => Some(include_bytes!(
            "../assets/icons/files-folder-download-dark.svg"
        )),
        "files-folder-download-light.svg" => Some(include_bytes!(
            "../assets/icons/files-folder-download-light.svg"
        )),
        "files-folder-image-dark.svg" => Some(include_bytes!(
            "../assets/icons/files-folder-image-dark.svg"
        )),
        "files-folder-image-light.svg" => Some(include_bytes!(
            "../assets/icons/files-folder-image-light.svg"
        )),
        "files-folder-plain-dark.svg" => Some(include_bytes!(
            "../assets/icons/files-folder-plain-dark.svg"
        )),
        "files-folder-plain-light.svg" => Some(include_bytes!(
            "../assets/icons/files-folder-plain-light.svg"
        )),
        "files-forward.svg" => Some(include_bytes!("../assets/icons/files-forward.svg")),
        "files-grid.svg" => Some(include_bytes!("../assets/icons/files-grid.svg")),
        "files-home.svg" => Some(include_bytes!("../assets/icons/files-home.svg")),
        "files-list.svg" => Some(include_bytes!("../assets/icons/files-list.svg")),
        "files-pictures.svg" => Some(include_bytes!("../assets/icons/files-pictures.svg")),
        "files-plus.svg" => Some(include_bytes!("../assets/icons/files-plus.svg")),
        "files-search.svg" => Some(include_bytes!("../assets/icons/files-search.svg")),
        "files-system.svg" => Some(include_bytes!("../assets/icons/files-system.svg")),
        "files-tab-folder.svg" => Some(include_bytes!("../assets/icons/files-tab-folder.svg")),
        "files-usb.svg" => Some(include_bytes!("../assets/icons/files-usb.svg")),
        "git.svg" => Some(include_bytes!("../assets/icons/git.svg")),
        "go.svg" => Some(include_bytes!("../assets/icons/go.svg")),
        "haskell.svg" => Some(include_bytes!("../assets/icons/haskell.svg")),
        "home.svg" => Some(include_bytes!("../assets/icons/home.svg")),
        "html.svg" => Some(include_bytes!("../assets/icons/html.svg")),
        "image.svg" => Some(include_bytes!("../assets/icons/image.svg")),
        "java.svg" => Some(include_bytes!("../assets/icons/java.svg")),
        "javascript.svg" => Some(include_bytes!("../assets/icons/javascript.svg")),
        "json.svg" => Some(include_bytes!("../assets/icons/json.svg")),
        "orus-k.svg" => Some(include_bytes!("../assets/icons/orus-k.svg")),
        "kotlin.svg" => Some(include_bytes!("../assets/icons/kotlin.svg")),
        "lock.svg" => Some(include_bytes!("../assets/icons/lock.svg")),
        "lua.svg" => Some(include_bytes!("../assets/icons/lua.svg")),
        "markdown.svg" => Some(include_bytes!("../assets/icons/markdown.svg")),
        "new-file.svg" => Some(include_bytes!("../assets/icons/new-file.svg")),
        "new-folder.svg" => Some(include_bytes!("../assets/icons/new-folder.svg")),
        "php.svg" => Some(include_bytes!("../assets/icons/php.svg")),
        "power-logout.svg" => Some(include_bytes!("../assets/icons/power-logout.svg")),
        "power-off.svg" => Some(include_bytes!("../assets/icons/power-off.svg")),
        "power-restart.svg" => Some(include_bytes!("../assets/icons/power-restart.svg")),
        "power-sleep.svg" => Some(include_bytes!("../assets/icons/power-sleep.svg")),
        "python.svg" => Some(include_bytes!("../assets/icons/python.svg")),
        "r.svg" => Some(include_bytes!("../assets/icons/r.svg")),
        "react.svg" => Some(include_bytes!("../assets/icons/react.svg")),
        "ruby.svg" => Some(include_bytes!("../assets/icons/ruby.svg")),
        "rust.svg" => Some(include_bytes!("../assets/icons/rust.svg")),
        "scala.svg" => Some(include_bytes!("../assets/icons/scala.svg")),
        "status-battery.svg" => Some(include_bytes!("../assets/icons/status-battery.svg")),
        "status-battery-charging.svg" => Some(include_bytes!(
            "../assets/icons/status-battery-charging.svg"
        )),
        "status-ethernet.svg" => Some(include_bytes!("../assets/icons/status-ethernet.svg")),
        "status-wifi.svg" => Some(include_bytes!("../assets/icons/status-wifi.svg")),
        "status-wifi-off.svg" => Some(include_bytes!("../assets/icons/status-wifi-off.svg")),
        "swift.svg" => Some(include_bytes!("../assets/icons/swift.svg")),
        "toml.svg" => Some(include_bytes!("../assets/icons/toml.svg")),
        "typescript.svg" => Some(include_bytes!("../assets/icons/typescript.svg")),
        "yaml.svg" => Some(include_bytes!("../assets/icons/yaml.svg")),
        "window-close.svg" => Some(include_bytes!("../assets/icons/window-close.svg")),
        "window-maximize.svg" => Some(include_bytes!("../assets/icons/window-maximize.svg")),
        "window-minimize.svg" => Some(include_bytes!("../assets/icons/window-minimize.svg")),
        "zig.svg" => Some(include_bytes!("../assets/icons/zig.svg")),
        _ => None,
    }
}
