use orus_core::appearance::{self, Theme};
use orus_core::display::{self, Display, OutputSetting};
use orus_core::input::{self, Input, with_input};

fn setting(mode: &str, scale: f32) -> OutputSetting {
    OutputSetting {
        mode: Some(mode.into()),
        scale,
        ..Default::default()
    }
}

#[test]
fn input_settings_change_by_key_and_bad_values_are_refused() {
    let mut i = Input::default();
    i.set("touchpad.tap", "off").unwrap();
    i.set("mouse.speed", "-0.5").unwrap();
    i.set("mouse.left-handed", "on").unwrap();
    assert!(!i.touchpad.tap_to_click);
    assert_eq!(i.mouse.pointer_speed, -0.5);
    assert!(i.mouse.left_handed);
    assert!(i.set("mouse.speed", "2").is_err());
    assert!(i.set("touchpad.tap", "maybe").is_err());
    assert!(i.set("keyboard.rate", "on").is_err());
}

#[test]
fn input_becomes_one_libinput_element_inside_the_config() {
    let mut i = Input::default();
    i.set("touchpad.natural-scroll", "off").unwrap();
    i.set("mouse.flat-acceleration", "on").unwrap();
    let rc = with_input("<labwc_config><theme/></labwc_config>", &i).unwrap();
    assert!(rc.ends_with("</labwc_config>"));
    assert_eq!(rc.matches("<libinput>").count(), 1);
    let touchpad = &rc[rc.find("category=\"touchpad\"").unwrap()..rc.find("non-touch").unwrap()];
    assert!(touchpad.contains("<naturalScroll>no</naturalScroll>"));
    assert!(touchpad.contains("<tap>yes</tap>"));
    let mouse = &rc[rc.find("non-touch").unwrap()..];
    assert!(mouse.contains("<accelProfile>flat</accelProfile>"));
    assert!(with_input("<other/>", &i).is_none());
}

#[test]
fn input_and_display_files_round_trip_and_a_missing_file_is_the_default() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(input::load(dir.path()).unwrap(), Input::default());
    assert_eq!(display::load(dir.path()).unwrap(), Display::default());
    let mut i = Input::default();
    i.set("mouse.speed", "0.25").unwrap();
    input::save(dir.path(), &i).unwrap();
    assert_eq!(input::load(dir.path()).unwrap(), i);
    let mut d = Display::default();
    d.set("Virtual-1", setting("1920x1080@60.000", 1.5))
        .unwrap();
    display::save(dir.path(), &d).unwrap();
    assert_eq!(display::load(dir.path()).unwrap(), d);
}

#[test]
fn display_values_that_could_reach_a_shell_are_refused() {
    let mut d = Display::default();
    assert!(d.set("eDP-1; rm -rf ~", setting("1920x1080", 1.0)).is_err());
    assert!(d.set("eDP-1", setting("1920x1080 && x", 1.0)).is_err());
    assert!(d.set("eDP-1", setting("wide", 1.0)).is_err());
    assert!(d.set("eDP-1", setting("99999x99999", 1.0)).is_err());
    assert!(d.set("eDP-1", setting("10x10", 1.0)).is_err());
    assert!(d.set("eDP-1", setting("1920x1080", 9.0)).is_err());
    let mut turned = setting("1920x1080", 1.0);
    turned.transform = "90; reboot".into();
    assert!(d.set("eDP-1", turned).is_err());
    assert!(d.outputs.is_empty());

    // A hand-edited file is checked on load too.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("display.json"),
        r#"{"outputs":{"a b":{"mode":"1x1","scale":1.0}}}"#,
    )
    .unwrap();
    assert!(display::load(dir.path()).is_err());
}

#[test]
fn the_session_config_carries_input_and_an_autostart_for_the_screen() {
    let system = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    std::fs::write(
        system.path().join("rc.xml"),
        "<labwc_config><theme><name>orus</name></theme></labwc_config>",
    )
    .unwrap();
    let mut d = Display::default();
    d.set("Virtual-1", setting("1280x800", 2.0)).unwrap();
    appearance::write_session_config(
        system.path(),
        &user.path().join("labwc"),
        Theme::Dark,
        &Input::default(),
        &d,
    )
    .unwrap();
    let rc = std::fs::read_to_string(user.path().join("labwc/rc.xml")).unwrap();
    assert!(rc.contains("<libinput>"));
    let autostart = std::fs::read_to_string(user.path().join("labwc/autostart")).unwrap();
    assert_eq!(
        autostart,
        "wlr-randr --output Virtual-1 --on --mode 1280x800 --scale 2 --transform normal &\n"
    );
}

#[test]
fn a_screen_can_be_placed_rotated_and_turned_off() {
    let mut right = OutputSetting {
        position: Some((1920, 0)),
        transform: "90".into(),
        ..Default::default()
    };
    assert_eq!(
        right.randr_args("HDMI-A-1"),
        [
            "--output",
            "HDMI-A-1",
            "--on",
            "--scale",
            "1",
            "--transform",
            "90",
            "--pos",
            "1920,0"
        ]
    );
    right.enabled = false;
    assert_eq!(
        right.randr_args("HDMI-A-1"),
        ["--output", "HDMI-A-1", "--off"]
    );
    let mut d = Display::default();
    assert!(
        d.set(
            "HDMI-A-1",
            OutputSetting {
                position: Some((9_999_999, 0)),
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn an_older_display_file_without_the_new_fields_still_loads() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("display.json"),
        r#"{"outputs":{"Virtual-1":{"mode":"1280x800","scale":1.5}}}"#,
    )
    .unwrap();
    let d = display::load(dir.path()).unwrap();
    let o = &d.outputs["Virtual-1"];
    assert!(o.enabled && o.position.is_none() && o.transform == "normal");
}

#[test]
fn a_custom_mode_is_passed_as_custom_and_checked_like_any_mode() {
    let custom = OutputSetting {
        mode: Some("2560x1600".into()),
        custom_mode: true,
        ..Default::default()
    };
    assert_eq!(
        custom.randr_args("Virtual-1"),
        [
            "--output",
            "Virtual-1",
            "--on",
            "--custom-mode",
            "2560x1600",
            "--scale",
            "1",
            "--transform",
            "normal"
        ]
    );
    let mut d = Display::default();
    d.set("Virtual-1", custom).unwrap();
    let bad = OutputSetting {
        mode: Some("2560x1600; reboot".into()),
        custom_mode: true,
        ..Default::default()
    };
    assert!(d.set("Virtual-1", bad).is_err());
}
