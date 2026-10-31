use jag_draw::Rect;
use orus_shell::app_kind::AppKind;
use orus_shell::app_window::AppWindow;
use orus_shell::layout::ShellLayout;

const ALL: [AppKind; 4] = [
    AppKind::Terminal,
    AppKind::Files,
    AppKind::TextEdit,
    AppKind::Viewer,
];

#[test]
fn each_app_round_trips_through_its_command_line_name() {
    for kind in ALL {
        assert_eq!(AppKind::parse(kind.name()), Some(kind));
    }
    assert_eq!(AppKind::parse("browser"), None);
    assert_eq!(AppKind::parse(""), None);
}

#[test]
fn each_app_has_its_own_wayland_app_id() {
    let ids: std::collections::HashSet<_> = ALL.iter().map(|k| k.app_id()).collect();
    assert_eq!(ids.len(), ALL.len());
    assert!(ALL.iter().all(|k| k.app_id().starts_with("orus-")));
}

#[test]
fn a_frameless_window_is_all_content_with_no_chrome_to_hit() {
    let bounds = Rect {
        x: 0.0,
        y: 0.0,
        w: 900.0,
        h: 600.0,
    };
    let w = AppWindow::frameless("Terminal", bounds);
    let c = w.content_rect();
    assert_eq!((c.x, c.y, c.w, c.h), (0.0, 0.0, 900.0, 600.0));
    assert!(w.is_maximized());
    assert_eq!(w.hit_test_buttons(880.0, 10.0), None);
    assert!(!w.hit_test_title_bar(450.0, 10.0));
    assert_eq!(w.hit_test_resize(0.0, 300.0), None);
}

#[test]
fn an_app_in_its_own_window_has_no_shell_bar() {
    let l = ShellLayout::without_bar(1000, 640);
    assert_eq!(l.shell_bar.h, 0.0);
    assert_eq!(l.app_viewport.y, 0.0);
    assert_eq!(l.app_viewport.h, 640.0);
}
