#[test]
fn page_title_reads_the_title_element() {
    use orus_shell::web_app::page_title;
    let dir = tempfile::tempdir().unwrap();
    let write = |name: &str, html: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, html).unwrap();
        path
    };

    let titled = write("a.html", "<HEAD><Title>\n  Release Ops \n</Title></HEAD>");
    assert_eq!(page_title(&titled).as_deref(), Some("Release Ops"));
    let empty = write("b.html", "<title>  </title>");
    assert_eq!(page_title(&empty), None);
    let missing = write("c.html", "<html></html>");
    assert_eq!(page_title(&missing), None);
}

#[test]
fn launch_target_uses_the_app_shell_only_for_apps_with_a_manifest() {
    use orus_shell::web_app::launch_target;
    let dir = tempfile::tempdir().unwrap();
    let index = dir.path().join("index.html");
    std::fs::write(&index, "<html></html>").unwrap();

    let (name, value) = launch_target(&index);
    assert_eq!(name, "DETIR_INITIAL_URL");
    assert_eq!(value, format!("file://{}", index.display()));

    std::fs::write(dir.path().join("waid-package.json"), "{}").unwrap();
    let (name, value) = launch_target(&index);
    assert_eq!(name, "DETIR_WAID_APP_PATH");
    assert_eq!(value, dir.path().display().to_string());
}
