use orus_core::desktop_apps::{list, parse};

fn entry(extra: &str) -> String {
    format!("[Desktop Entry]\nType=Application\nName=Gimp\nExec=gimp-2.10 %U\n{extra}")
}

#[test]
fn reads_the_name_and_command_without_field_codes() {
    let app = parse(&entry("")).unwrap();
    assert_eq!(app.name, "Gimp");
    assert_eq!(app.argv, ["gimp-2.10"]);
    let quoted = parse("[Desktop Entry]\nType=Application\nName=X\nExec=sh -c \"echo %% hi\" %f\n").unwrap();
    assert_eq!(quoted.argv, ["sh", "-c", "echo % hi"]);
}

#[test]
fn skips_what_the_launcher_should_not_show() {
    for extra in ["NoDisplay=true\n", "Hidden=true\n", "Terminal=true\n", "OnlyShowIn=GNOME;\n", "TryExec=/no/such/program\n"] {
        assert!(parse(&entry(extra)).is_none(), "{extra}");
    }
    assert!(parse("[Desktop Entry]\nType=Link\nName=X\nExec=x\n").is_none());
    // Keys of another section do not count.
    assert!(parse("[Desktop Entry]\nType=Application\nName=X\nExec=x\n[Desktop Action new]\nNoDisplay=true\n").is_some());
}

#[test]
fn lists_by_name_with_the_first_folder_winning() {
    let user = tempfile::tempdir().unwrap();
    let system = tempfile::tempdir().unwrap();
    std::fs::write(user.path().join("b.desktop"), "[Desktop Entry]\nType=Application\nName=Bee user\nExec=bee\n").unwrap();
    std::fs::write(system.path().join("b.desktop"), "[Desktop Entry]\nType=Application\nName=Bee system\nExec=bee\n").unwrap();
    std::fs::write(system.path().join("a.desktop"), "[Desktop Entry]\nType=Application\nName=Ant\nExec=ant\n").unwrap();
    std::fs::write(system.path().join("c.desktop"), "[Desktop Entry]\nType=Application\nName=Chromium\nExec=chromium\n").unwrap();
    std::fs::write(system.path().join("notes.txt"), "x").unwrap();
    let names: Vec<String> = list(
        &[user.path().into(), system.path().into()],
        |a| a.argv[0] == "chromium",
    )
    .into_iter()
    .map(|a| a.name)
    .collect();
    assert_eq!(names, ["Ant", "Bee user"]);
}
