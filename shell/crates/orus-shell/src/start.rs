//! Starting the apps the launcher and the dock list.

use crate::app_kind::AppKind;
use crate::launcher::AppId;

/// Start `app`. Terminal, Files, Text Edit and Viewer, and web apps, each open
/// as a window of their own, sized from `screen` (logical pixels). Failures are logged.
pub fn start(app: AppId, screen: (f64, f64)) {
    let start_app = |kind: AppKind| {
        if let Err(e) = crate::external_app::launch_app(kind, screen) {
            log::error!("Failed to start {}: {e}", kind.title());
        }
    };
    match app {
        AppId::Terminal => start_app(AppKind::Terminal),
        AppId::TextEdit => start_app(AppKind::TextEdit),
        AppId::FileManager => start_app(AppKind::Files),
        AppId::Viewer => start_app(AppKind::Viewer),
        AppId::Chromium => {
            if let Err(e) = crate::external_app::launch_chromium() {
                log::error!("Failed to launch Chromium: {e}");
            }
        }
        AppId::Installer => {
            if let Err(e) = crate::external_app::launch_installer() {
                log::error!("Failed to launch the installer: {e}");
            }
        }
        AppId::Office => {
            if let Err(e) = crate::external_app::launch_office(screen) {
                log::error!("Failed to start Office: {e}");
            }
        }
        AppId::Desktop { argv } => {
            if let Err(e) = crate::external_app::launch_desktop(&argv) {
                log::error!("Failed to start {}: {e}", argv[0]);
            }
        }
        AppId::WebApp { dir } => {
            let index = dir.join("index.html");
            if let Err(e) = crate::web_app::launch(&index, screen) {
                log::error!("Failed to launch {}: {e}", index.display());
            }
        }
    }
}
