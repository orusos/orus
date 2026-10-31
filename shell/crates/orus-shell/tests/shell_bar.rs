use jag_draw::Rect;
use orus_shell::shell_bar::ShellBar;

const BAR: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 1280.0,
    h: 36.0,
};

#[test]
fn apps_button_covers_the_left_end_of_the_bar() {
    assert!(ShellBar::apps_button_contains(BAR, 0.0, 0.0));
    assert!(ShellBar::apps_button_contains(BAR, 55.9, 35.9));
}

#[test]
fn apps_button_excludes_the_rest_of_the_bar() {
    assert!(!ShellBar::apps_button_contains(BAR, 56.0, 10.0));
    assert!(!ShellBar::apps_button_contains(BAR, 640.0, 10.0));
    assert!(!ShellBar::apps_button_contains(BAR, 10.0, 36.0));
}
