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
fn launch_target_passes_a_directory_or_bare_html_directly_to_waid() {
    use orus_shell::web_app::launch_target;
    let dir = tempfile::tempdir().unwrap();
    let index = dir.path().join("index.html");
    std::fs::write(&index, "<html></html>").unwrap();

    assert_eq!(launch_target(&index), index);

    std::fs::write(dir.path().join("waid-package.json"), "{}").unwrap();
    assert_eq!(launch_target(&index), dir.path());
}
