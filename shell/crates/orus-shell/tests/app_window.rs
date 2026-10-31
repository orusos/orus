use jag_draw::Rect;
use orus_shell::app_window::{AppWindow, TITLE_BAR_HEIGHT, WindowButton};

fn window() -> AppWindow {
    AppWindow::new(
        "Text Edit",
        Rect {
            x: 100.0,
            y: 100.0,
            w: 600.0,
            h: 400.0,
        },
    )
}

#[test]
fn buttons_sit_at_the_right_end_in_minimize_maximize_close_order() {
    let w = window();
    let y = w.bounds.y + TITLE_BAR_HEIGHT * 0.5;
    let right = w.bounds.x + w.bounds.w;
    assert_eq!(
        w.hit_test_buttons(right - 28.0, y),
        Some(WindowButton::Close)
    );
    assert_eq!(
        w.hit_test_buttons(right - 78.0, y),
        Some(WindowButton::Maximize)
    );
    assert_eq!(
        w.hit_test_buttons(right - 128.0, y),
        Some(WindowButton::Minimize)
    );
}

#[test]
fn the_left_end_and_the_body_have_no_buttons() {
    let w = window();
    let y = w.bounds.y + TITLE_BAR_HEIGHT * 0.5;
    assert_eq!(w.hit_test_buttons(w.bounds.x + 14.0, y), None);
    assert_eq!(
        w.hit_test_buttons(w.bounds.x + w.bounds.w - 20.0, w.bounds.y + 200.0),
        None
    );
}
