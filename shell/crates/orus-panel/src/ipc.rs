//! How the compositor's keybindings reach the panel: `orus-linux --toggle-launcher`
//! writes a command line to a socket in the runtime directory, and the panel reads it.

use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

use anyhow::{Context, Result};

/// The command that opens or closes the launcher.
pub const TOGGLE_LAUNCHER: &str = "toggle-launcher";

pub fn socket_path() -> Result<PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR is not set")?;
    Ok(PathBuf::from(dir).join("orus-panel.sock"))
}

/// Listen for commands. A socket left by a panel that died is replaced.
pub fn listen() -> Result<UnixListener> {
    let path = socket_path()?;
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)
        .with_context(|| format!("cannot listen on {}", path.display()))?;
    listener.set_nonblocking(true)?;
    Ok(listener)
}

/// Tell the running panel to do `command`.
pub fn send(command: &str) -> Result<()> {
    let path = socket_path()?;
    let mut stream = UnixStream::connect(&path)
        .with_context(|| format!("the panel is not running ({})", path.display()))?;
    writeln!(stream, "{command}")?;
    Ok(())
}
