//! Exercise the real distro CLI's process arguments with an isolated runtime recorder.
#![cfg(unix)]
mod common;

use common::{Env, bundle, yes};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn installed_apps_open_with_the_direct_waid_path_and_not_a_browser_url() {
    let env = Env::new();
    let source = bundle(&env.path("source with spaces"), "launch-test", "1.0.0", &[]);
    env.store.install_dir(&source, &mut yes).unwrap();
    let bin = env.path("bin with spaces");
    std::fs::create_dir_all(&bin).unwrap();
    let cli = bin.join("orus");
    std::fs::copy(env!("CARGO_BIN_EXE_orus"), &cli).unwrap();
    let runtime = bin.join("waid-shell");
    std::fs::write(
        &runtime,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$SS_T14_ARGUMENTS\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&runtime, std::fs::Permissions::from_mode(0o755)).unwrap();
    let recorded = env.path("arguments");
    let output = Command::new(&cli)
        .args(["open", "launch-test"])
        .env("HOME", env.path("home"))
        .env("ORUS_APPS_DIR", &env.store.paths.apps_dir)
        .env("SS_T14_ARGUMENTS", &recorded)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    let text = loop {
        if let Ok(text) = std::fs::read_to_string(&recorded)
            && !text.is_empty()
        {
            break text;
        }
        assert!(Instant::now() < deadline, "runtime was not launched");
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [env.store
            .paths
            .apps_dir
            .join("launch-test")
            .to_str()
            .unwrap()]
    );
}
