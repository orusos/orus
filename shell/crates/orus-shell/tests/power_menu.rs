use jag_draw::Rect;
use orus_shell::power_menu::{PowerAction, PowerMenu};
use orus_shell::shell_bar::ShellBar;

const BAR: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 1280.0,
    h: 36.0,
};

#[test]
fn the_menu_hangs_below_the_bar_at_the_right_edge() {
    let menu = PowerMenu::rect(BAR);
    assert!(menu.y > BAR.y + BAR.h);
    assert!(menu.x + menu.w < BAR.w);
}

#[test]
fn each_row_picks_its_action_in_order() {
    let menu = PowerMenu::rect(BAR);
    let x = menu.x + menu.w / 2.0;
    let expected = [
        PowerAction::Sleep,
        PowerAction::Restart,
        PowerAction::ShutDown,
        PowerAction::LogOut,
    ];
    for (i, action) in expected.iter().enumerate() {
        let y = menu.y + 6.0 + 40.0 * i as f32 + 20.0;
        assert_eq!(PowerMenu::action_at(BAR, x, y), Some(*action), "row {i}");
    }
    assert_eq!(PowerMenu::action_at(BAR, menu.x - 5.0, menu.y + 30.0), None);
    assert_eq!(PowerMenu::action_at(BAR, x, menu.y + menu.h + 5.0), None);
}

#[test]
fn the_actions_run_systemctl_or_ask_labwc_to_exit() {
    assert_eq!(
        PowerAction::Sleep.command(),
        ("systemctl", &["suspend"][..])
    );
    assert_eq!(
        PowerAction::Restart.command(),
        ("systemctl", &["reboot"][..])
    );
    assert_eq!(
        PowerAction::ShutDown.command(),
        ("systemctl", &["poweroff"][..])
    );
    assert_eq!(PowerAction::LogOut.command(), ("labwc", &["--exit"][..]));
}

#[test]
fn the_right_end_of_the_bar_opens_the_menu_and_the_rest_does_not() {
    assert!(ShellBar::status_area_contains(BAR, 1279.0, 10.0));
    assert!(ShellBar::status_area_contains(BAR, 1080.0, 10.0));
    assert!(!ShellBar::status_area_contains(BAR, 1079.0, 10.0));
    assert!(!ShellBar::status_area_contains(BAR, 1279.0, 36.0));
    assert!(!ShellBar::status_area_contains(BAR, 640.0, 10.0));
}
