//! The `orus` binary: the output the store app reads and the capability check.

mod common;

use std::process::Command;

use common::{Env, bundle};
use orus::{catalog, serve};

fn orus(env: &Env, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_orus"))
        .args(args)
        .env("HOME", env.path("home"))
        .env("ORUS_APPS_DIR", env.path("home/.orus/apps"))
        .output()
        .unwrap()
}

fn serve_dev(env: &Env, capabilities: &[&str]) -> serve::Running {
    let b = bundle(&env.path("src"), "hello", "1.0.0", capabilities);
    catalog::build(&[b], &env.path("catalog"), "Dev", None).unwrap();
    let running = serve::start(&env.path("catalog"), "127.0.0.1:0").unwrap();
    let url = format!("http://127.0.0.1:{}", running.addr.port());
    let out = orus(
        env,
        &["sources", "add", "--development", "--name", "dev", &url],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = orus(env, &["update"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    running
}

#[test]
fn search_list_and_sources_print_json() {
    let env = Env::new();
    let running = serve_dev(&env, &["media.camera.capture"]);

    let search: serde_json::Value =
        serde_json::from_slice(&orus(&env, &["search", "--json", "hel"]).stdout).unwrap();
    assert_eq!(search[0]["id"], "hello");
    assert_eq!(search[0]["source"], "dev");
    assert_eq!(search[0]["unsigned"], true);
    assert_eq!(search[0]["capabilities"][0], "media.camera.capture");
    assert!(search[0]["installed"].is_null());

    let sources: serde_json::Value =
        serde_json::from_slice(&orus(&env, &["sources", "list", "--json"]).stdout).unwrap();
    assert_eq!(sources[0]["name"], "dev");
    assert_eq!(sources[0]["development"], true);

    let list: serde_json::Value =
        serde_json::from_slice(&orus(&env, &["list", "--json"]).stdout).unwrap();
    assert_eq!(list, serde_json::json!([]));
    running.stop();
}

#[test]
fn allow_installs_only_what_was_shown() {
    let env = Env::new();
    let running = serve_dev(&env, &["media.camera.capture"]);

    let refused = orus(
        &env,
        &["install", "--allow", "media.audio.playback", "hello"],
    );
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("cancelled"));

    let ok = orus(
        &env,
        &["install", "--allow", "media.camera.capture", "hello"],
    );
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stderr)
    );
    let list: serde_json::Value =
        serde_json::from_slice(&orus(&env, &["list", "--json"]).stdout).unwrap();
    assert_eq!(list[0]["id"], "hello");
    assert_eq!(list[0]["canRollBack"], false);
    assert_eq!(list[0]["system"], false);
    running.stop();
}
