use orus_core::appearance::{
    self, Accent, Appearance, AppearanceError, Theme, Wallpaper, with_theme_name,
};

#[test]
fn a_missing_file_is_the_default_dark_teal_appearance() {
    let dir = tempfile::tempdir().unwrap();
    let a = appearance::load(dir.path()).unwrap();
    assert_eq!(a, Appearance::default());
    assert_eq!(
        (a.theme, a.accent, a.reduce_motion),
        (Theme::Dark, Accent::Teal, false)
    );
    assert_eq!(a.wallpaper, None);
}

#[test]
fn saved_settings_load_back_and_the_file_has_no_temp_left_behind() {
    let dir = tempfile::tempdir().unwrap();
    let a = Appearance {
        theme: Theme::Light,
        accent: Accent::Rose,
        reduce_motion: true,
        wallpaper: Some(Wallpaper::Dunes),
    };
    appearance::save(dir.path(), &a).unwrap();
    assert_eq!(appearance::load(dir.path()).unwrap(), a);
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, vec!["appearance.json"]);
}

#[test]
fn a_file_with_an_unknown_value_is_an_error_not_a_silent_default() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("appearance.json"), r#"{"theme":"sepia"}"#).unwrap();
    assert!(matches!(
        appearance::load(dir.path()),
        Err(AppearanceError::Invalid(_))
    ));
}

#[test]
fn missing_fields_take_their_defaults() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("appearance.json"), r#"{"theme":"light"}"#).unwrap();
    let a = appearance::load(dir.path()).unwrap();
    assert_eq!((a.theme, a.accent), (Theme::Light, Accent::Teal));
}

#[test]
fn the_wallpaper_follows_the_theme_unless_one_is_chosen() {
    let mut a = Appearance::default();
    assert_eq!(a.effective_wallpaper(), Wallpaper::Dunes);
    a.theme = Theme::Light;
    assert_eq!(a.effective_wallpaper(), Wallpaper::DunesLight);
    a.wallpaper = Some(Wallpaper::Dunes);
    assert_eq!(a.effective_wallpaper(), Wallpaper::Dunes);
}

#[test]
fn the_theme_name_in_rc_xml_is_replaced_and_nothing_else_changes() {
    let rc = "<labwc_config><theme><name>orus</name><cornerRadius>14</cornerRadius></theme>\
              <windowRules><windowRule title=\"x\"><name>keep</name></windowRule></windowRules></labwc_config>";
    let out = with_theme_name(rc, "orus-light").unwrap();
    assert!(out.contains("<theme><name>orus-light</name><cornerRadius>14</cornerRadius>"));
    assert!(out.contains("<name>keep</name>"));
    assert_eq!(with_theme_name("<labwc_config/>", "x"), None);
}

#[test]
fn the_session_config_is_the_system_rc_with_the_chosen_theme() {
    let system = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    std::fs::write(
        system.path().join("rc.xml"),
        "<labwc_config><theme><name>orus</name></theme></labwc_config>",
    )
    .unwrap();
    appearance::write_session_config(
        system.path(),
        &user.path().join("labwc"),
        Theme::Light,
        &Default::default(),
        &Default::default(),
    )
    .unwrap();
    let written = std::fs::read_to_string(user.path().join("labwc/rc.xml")).unwrap();
    assert!(written.contains("<name>orus-light</name>"));
}

#[test]
fn accent_and_selection_colors_match_the_concepts() {
    use orus_core::appearance::{accent_color, selection_color};
    assert_eq!(accent_color(Theme::Dark, Accent::Teal), 0x63b1ac);
    assert_eq!(accent_color(Theme::Light, Accent::Rose), 0xe06e8f);
    // The concepts' selected rows are #2f4849 (dark) and #d7edee (light).
    assert_eq!(selection_color(Theme::Dark, Accent::Teal), 0x2e4849);
    assert_eq!(selection_color(Theme::Light, Accent::Teal), 0xd9e9eb);
}
