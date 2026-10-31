use jag_draw::Rect;
use orus_greeter::layout::{FORM_H, FORM_W, Layout, contains};

fn viewport(w: f32, h: f32) -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w,
        h,
    }
}

#[test]
fn the_form_is_centered_and_ordered_top_to_bottom() {
    for (w, h) in [(1366.0, 768.0), (1280.0, 720.0), (1920.0, 1080.0)] {
        let l = Layout::new(viewport(w, h));
        assert!((l.password_field.x + FORM_W / 2.0 - w / 2.0).abs() < 0.01);
        assert!((l.avatar.x + l.avatar.w / 2.0 - w / 2.0).abs() < 0.01);
        assert!(l.top_center_y < l.avatar.y);
        assert!(l.avatar.y + l.avatar.h < l.name_center_y);
        assert!(l.name_center_y < l.password_field.y);
        // There is no sign-in button (Enter signs in), so "Switch user" and any message
        // follow the field directly.
        assert!(l.password_field.y + l.password_field.h < l.message_center_y);
        assert!(l.message_center_y < l.switch_hit.y + l.switch_hit.h);
        assert!(l.switch_hit.y + l.switch_hit.h < l.pill_origin[1]);
    }
}

#[test]
fn the_stack_sits_in_the_middle_of_the_screen_with_room_around_it() {
    for (w, h) in [(1366.0, 768.0), (1280.0, 720.0), (1920.0, 1080.0)] {
        let l = Layout::new(viewport(w, h));
        let top = l.avatar.y;
        let bottom = l.switch_hit.y + l.switch_hit.h;
        let middle = (top + bottom) / 2.0;
        // A little above the exact middle, never in the top or bottom third.
        assert!(
            middle > h * 0.33 && middle < h * 0.55,
            "{w}x{h}: stack middle {middle}"
        );
        // Spaced out: the stack takes less than half of the screen's height.
        assert!(bottom - top < h * 0.5, "{w}x{h}: stack {}", bottom - top);
    }
}

#[test]
fn the_password_field_is_a_small_pill() {
    let l = Layout::new(viewport(1366.0, 768.0));
    assert_eq!((l.password_field.w, l.password_field.h), (FORM_W, FORM_H));
    // 300 x 48 on the mock-up; the drawing layer shows it at 80%.
    assert_eq!((FORM_W, FORM_H), (300.0, 48.0));
    assert!(FORM_W < 466.0 && FORM_H < 56.0); // the old field was 466 x 56
    // The eye icon is inside the field, at its right.
    assert!(l.eye.x > l.password_field.x + FORM_W / 2.0);
    assert!(l.eye.x + l.eye.w < l.password_field.x + FORM_W);
}

#[test]
fn the_account_name_field_sits_above_the_password_field() {
    let l = Layout::new(viewport(1366.0, 768.0));
    assert!(l.username_field.y + l.username_field.h < l.password_field.y);
}

#[test]
fn the_bottom_controls_follow_the_bottom_edge() {
    let tall = Layout::new(viewport(1672.0, 941.0));
    let short = Layout::new(viewport(1672.0, 720.0));
    assert_eq!(tall.power.y - short.power.y, 221.0);
    assert_eq!(tall.pill_origin[1] - short.pill_origin[1], 221.0);
    // The power circle sits at the right margin, with the Wi-Fi circle beside it.
    assert_eq!(tall.power.x + tall.power.w, 1672.0 - 40.0);
    assert!(tall.wifi.x + tall.wifi.w < tall.power.x);
}

#[test]
fn the_power_menu_opens_above_the_power_button() {
    let l = Layout::new(viewport(1280.0, 720.0));
    assert!(l.menu.y + l.menu.h < l.power.y);
    assert_eq!(l.menu.x + l.menu.w, l.power.x + l.power.w);
}

#[test]
fn hit_testing_uses_half_open_bounds() {
    let r = Rect {
        x: 10.0,
        y: 10.0,
        w: 20.0,
        h: 20.0,
    };
    assert!(contains(r, 10.0, 10.0));
    assert!(!contains(r, 30.0, 20.0));
}
