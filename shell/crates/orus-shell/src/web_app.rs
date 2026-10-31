//! Launch built waid web apps in their own `waid-shell` process.
//!
//! The embedded `SceneRenderer` only hosts IR packages, so a web bundle runs in
//! the standalone runtime. The runtime is shared: one `waid-shell` binary, many
//! bundles.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The environment variable and value that start `waid-shell` on `index_html`.
///
/// An app with a `waid-package.json` starts in the app shell: no browser
/// toolbar, titled and identified from that manifest. A bare bundle opens as a
/// page by URL.
pub fn launch_target(index_html: &Path) -> (&'static str, String) {
    let app_dir = index_html.parent();
    match app_dir.filter(|dir| dir.join("waid-package.json").is_file()) {
        Some(dir) => ("DETIR_WAID_APP_PATH", dir.display().to_string()),
        None => (
            "DETIR_INITIAL_URL",
            format!("file://{}", index_html.display()),
        ),
    }
}

/// The text of the page's `<title>`, trimmed, if it has a non-empty one.
/// Entities are not decoded.
pub fn page_title(index_html: &Path) -> Option<String> {
    let html = std::fs::read_to_string(index_html).ok()?;
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<title>")? + "<title>".len();
    let end = start + lower[start..].find("</title>")?;
    let title = html[start..end].trim();
    (!title.is_empty()).then(|| title.to_string())
}

/// Key under which the app at `index_html` is tracked as running.
pub fn running_key(index_html: &Path) -> String {
    index_html.display().to_string()
}

/// Start `waid-shell` on `index_html`, titled after the page and sized by
/// `external_app::app_window_size` for a screen of `shell_size` (logical pixels). Uses the `waid-shell` next to the shell
/// binary when present, otherwise the one on `PATH`.
pub fn launch(index_html: &Path, shell_size: (f64, f64)) -> std::io::Result<()> {
    let sibling = std::env::current_exe()?.with_file_name("waid-shell");
    let program = if sibling.exists() {
        sibling
    } else {
        PathBuf::from("waid-shell")
    };
    let title = page_title(index_html).unwrap_or_else(|| {
        index_html
            .parent()
            .and_then(|dir| dir.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Orus OS App".to_string())
    });
    let (launch_env, launch_value) = launch_target(index_html);
    let (width, height) = crate::external_app::app_window_size(shell_size);
    let mut command = Command::new(program);
    command
        .env(launch_env, launch_value)
        .env("DETIR_WINDOW_TITLE", title)
        .env("DETIR_WINDOW_WIDTH", width.to_string())
        .env("DETIR_WINDOW_HEIGHT", height.to_string())
        // The runtime draws the page at the design size, at the screen's own resolution.
        .env("DETIR_UI_SCALE", orus_core::design::SCALE.to_string());
    crate::external_app::spawn_tracked(&running_key(index_html), command)
}
