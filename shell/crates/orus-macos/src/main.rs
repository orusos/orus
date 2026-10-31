//! Orus OS Universal — macOS launcher.

mod platform;

use anyhow::Result;
use orus_shell::runtime;
use platform::MacosPlatformHooks;

fn main() -> Result<()> {
    env_logger::init();
    runtime::run(MacosPlatformHooks)
}
