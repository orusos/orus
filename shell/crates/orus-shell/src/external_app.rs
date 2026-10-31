//! Run programs outside the shell as their own windows.

use std::collections::BTreeSet;
use std::io;
use std::process::Command;
use std::sync::Mutex;

/// Keys of the programs this shell started that are still running.
static RUNNING: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// Whether the program started under `key` by [`spawn_tracked`] is still running.
pub fn is_running(key: &str) -> bool {
    RUNNING.lock().is_ok_and(|running| running.contains(key))
}

/// Start `command`, count it as running under `key` until it exits, and reap
/// it then so it does not stay a zombie.
pub fn spawn_tracked(key: &str, mut command: Command) -> io::Result<()> {
    let mut child = command.spawn()?;
    if let Ok(mut running) = RUNNING.lock() {
        running.insert(key.to_string());
    }
    let key = key.to_string();
    std::thread::spawn(move || {
        let _ = child.wait();
        if let Ok(mut running) = RUNNING.lock() {
            running.remove(&key);
        }
    });
    Ok(())
}

/// Whether any program started under a key beginning with `prefix` is still running.
pub fn is_running_prefix(prefix: &str) -> bool {
    RUNNING
        .lock()
        .is_ok_and(|running| running.iter().any(|key| key.starts_with(prefix)))
}

static NEXT_APP_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Prefix of the keys of the processes [`launch_app`] starts for `kind`.
pub fn app_key_prefix(kind: crate::app_kind::AppKind) -> String {
    format!("app-{}-", kind.name())
}

/// Share of the screen's width an app window opens at.
pub const APP_WINDOW_FRACTION: f64 = 0.8;

/// The client size, in the compositor's pixels, for a new app window on a screen of `screen`
/// size: wide as `APP_WINDOW_FRACTION`, and short enough that the compositor, which
/// centers a new window, leaves the dock visible below it (the title bar counts
/// toward the height, and the dock band is kept free at the top as well).
///
/// `screen` is in the units the shell draws in, which are the mock-up's pixels; they are drawn
/// at `design::SCALE`, so they are multiplied by it to get pixels. The title bar is drawn by
/// the compositor and is already in pixels.
pub fn app_window_size(screen: (f64, f64)) -> (f64, f64) {
    let scale = f64::from(orus_core::design::SCALE);
    let band = f64::from(crate::dock_bar::BAND_HEIGHT) * scale;
    let title = f64::from(crate::app_window::TITLE_BAR_HEIGHT);
    (
        (screen.0 * scale * APP_WINDOW_FRACTION).round(),
        (screen.1 * scale - 2.0 * band - title)
            .max(MIN_APP_HEIGHT * scale)
            .round(),
    )
}

/// A window never opens shorter than this.
const MIN_APP_HEIGHT: f64 = 300.0;

/// Start `kind` as a window of its own: this same binary run with `--app`, sized
/// by [`app_window_size`].
pub fn launch_app(kind: crate::app_kind::AppKind, screen: (f64, f64)) -> io::Result<()> {
    launch_app_with(kind, screen, &[])
}

/// [`launch_app`] with extra environment variables for the app.
fn launch_app_with(
    kind: crate::app_kind::AppKind,
    screen: (f64, f64),
    env: &[(&str, &str)],
) -> io::Result<()> {
    let (w, h) = app_window_size(screen);
    let mut command = Command::new(std::env::current_exe()?);
    command.args(["--app", kind.name(), "--size", &format!("{w}x{h}")]);
    command.envs(env.iter().copied());
    let n = NEXT_APP_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    spawn_tracked(&format!("{}{n}", app_key_prefix(kind)), command)
}

/// The environment variable that gives a new Terminal a command to type at its prompt.
pub const TERMINAL_COMMAND_ENV: &str = "ORUS_TERMINAL_COMMAND";

/// What the Office tile runs when LibreOffice is not installed. `sudo` asks for the
/// password in the Terminal, so the install needs no privileges of its own.
pub const OFFICE_INSTALL_COMMAND: &str = "sudo apt-get update && sudo apt-get install -y libreoffice-writer libreoffice-calc libreoffice-impress libreoffice-gtk3 && echo 'LibreOffice is installed. Open Office from the launcher.'";

/// Whether `program` is in a directory of `PATH`.
pub fn on_path(program: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
}

/// Start LibreOffice, or, when it is not installed, a Terminal that installs it.
pub fn launch_office(screen: (f64, f64)) -> io::Result<()> {
    if on_path("libreoffice") {
        return spawn_tracked("office", Command::new("libreoffice"));
    }
    launch_app_with(
        crate::app_kind::AppKind::Terminal,
        screen,
        &[(TERMINAL_COMMAND_ENV, OFFICE_INSTALL_COMMAND)],
    )
}

/// Key under which [`launch_chromium`] is tracked.
pub const CHROMIUM_KEY: &str = "chromium";

/// Start Debian's `chromium` through the session wrapper, which selects Wayland.
pub fn launch_chromium() -> io::Result<()> {
    spawn_tracked(CHROMIUM_KEY, Command::new("chromium"))
}

/// Start a program installed with the package manager.
pub fn launch_desktop(argv: &[String]) -> io::Result<()> {
    let mut command = Command::new(&argv[0]);
    command.args(&argv[1..]);
    spawn_tracked(&format!("desktop-{}", argv[0]), command)
}

/// Start the Orus OS installer from the live session.
pub fn launch_installer() -> io::Result<()> {
    spawn_tracked("installer", Command::new("orus-install"))
}
