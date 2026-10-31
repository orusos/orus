use orus_core::appearance::{self, Accent, Appearance, Theme};
use orus_shell::appearance_watch::AppearanceWatch;

fn light() -> Appearance {
    Appearance {
        theme: Theme::Light,
        ..Appearance::default()
    }
}

#[test]
fn a_new_watch_starts_with_the_file_or_the_default() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        AppearanceWatch::new(dir.path()).current,
        Appearance::default()
    );
    appearance::save(dir.path(), &light()).unwrap();
    assert_eq!(AppearanceWatch::new(dir.path()).current.theme, Theme::Light);
}

#[test]
fn poll_reports_a_change_once_and_then_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut watch = AppearanceWatch::new(dir.path());
    assert!(!watch.poll());
    appearance::save(dir.path(), &light()).unwrap();
    assert!(watch.poll());
    assert_eq!(watch.current.theme, Theme::Light);
    assert!(!watch.poll());
}

#[test]
fn a_file_that_does_not_parse_keeps_the_current_appearance() {
    let dir = tempfile::tempdir().unwrap();
    appearance::save(dir.path(), &light()).unwrap();
    let mut watch = AppearanceWatch::new(dir.path());
    std::fs::write(dir.path().join("appearance.json"), "{ not json").unwrap();
    assert!(!watch.poll());
    assert_eq!(watch.current.theme, Theme::Light);
    appearance::save(
        dir.path(),
        &Appearance {
            accent: Accent::Rose,
            ..light()
        },
    )
    .unwrap();
    assert!(watch.poll());
    assert_eq!(watch.current.accent, Accent::Rose);
}
