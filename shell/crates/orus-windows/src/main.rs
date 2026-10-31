//! Orus OS Universal — Windows launcher.

mod platform;

use anyhow::Result;
use orus_shell::runtime;
use platform::WindowsPlatformHooks;

fn main() -> Result<()> {
    env_logger::init();
    runtime::run(WindowsPlatformHooks)
}
