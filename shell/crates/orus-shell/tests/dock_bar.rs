use jag_draw::Rect;
use orus_shell::dock_bar::{BAND_HEIGHT, DOCK_WIDTH, DockApp, DockBar, dock_width};

const SCREEN: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 1280.0,
    h: 800.0,
};

#[test]
fn the_dock_is_centered_and_sits_inside_the_bottom_band() {
    let dock = DockBar::new().rect(SCREEN);
    assert!((dock.x + dock.w / 2.0 - SCREEN.w / 2.0).abs() < 0.01);
    assert!(dock.y >= SCREEN.h - BAND_HEIGHT);
    assert!(dock.y + dock.h < SCREEN.h);
}

#[test]
fn each_icon_cell_hits_its_own_app_in_order() {
    let bar = DockBar::new();
    let dock = DockBar::new().rect(SCREEN);
    let y = dock.y + dock.h / 2.0;
    for (i, app) in DockApp::ALL.iter().enumerate() {
        let x = dock.x + 21.0 + i as f32 * 65.0 + 20.0;
        assert_eq!(bar.hit_test(SCREEN, x, y), Some(*app), "cell {i}");
    }
}

#[test]
fn outside_the_dock_hits_nothing() {
    let bar = DockBar::new();
    let dock = DockBar::new().rect(SCREEN);
    assert_eq!(bar.hit_test(SCREEN, dock.x - 1.0, dock.y + 20.0), None);
    assert_eq!(bar.hit_test(SCREEN, dock.x + 40.0, dock.y - 1.0), None);
}

#[test]
fn the_dock_stays_out_unless_a_window_covers_the_screen() {
    let bar = DockBar::new();
    assert!(bar.wants_to_show(SCREEN, (10.0, 10.0), false));
    assert!(!bar.wants_to_show(SCREEN, (10.0, 10.0), true));
}

#[test]
fn under_a_covering_window_the_bottom_edge_or_the_dock_brings_it_back() {
    let bar = DockBar::new();
    let dock = DockBar::new().rect(SCREEN);
    assert!(bar.wants_to_show(SCREEN, (5.0, SCREEN.h - 1.0), true));
    assert!(!bar.wants_to_show(SCREEN, (5.0, SCREEN.h - 20.0), true));
    // While it is out, the pointer over it keeps it out.
    assert!(bar.wants_to_show(SCREEN, (dock.x + 10.0, dock.y + 10.0), true));
    assert!(!bar.wants_to_show(SCREEN, (dock.x - 10.0, dock.y + 10.0), true));
}

#[test]
fn a_hidden_dock_is_not_clickable_and_slides_back_with_reduce_motion_off() {
    let mut bar = DockBar::new();
    let dock = DockBar::new().rect(SCREEN);
    let hit = (dock.x + 40.0, dock.y + dock.h / 2.0);
    assert!(!bar.tick(false, true), "instant moves all the way");
    assert!(bar.is_hidden());
    assert_eq!(bar.hit_test(SCREEN, hit.0, hit.1), None);
    // One short step does not finish the slide.
    assert!(bar.tick(true, false));
    assert!(!bar.is_hidden());
    // The step is capped per tick, so two ticks a while apart finish the slide.
    for _ in 0..2 {
        std::thread::sleep(std::time::Duration::from_millis(120));
        bar.tick(true, false);
    }
    assert!(bar.hit_test(SCREEN, hit.0, hit.1).is_some());
}

#[test]
fn each_added_tile_widens_the_dock_by_one_pitch_and_keeps_it_centered() {
    assert_eq!(dock_width(5), DOCK_WIDTH);
    assert_eq!(dock_width(7) - dock_width(5), 2.0 * 65.0);
    let mut bar = DockBar::new();
    bar.set_tile_count(8);
    let dock = bar.rect(SCREEN);
    assert_eq!(dock.w, dock_width(8));
    assert!((dock.x + dock.w / 2.0 - SCREEN.w / 2.0).abs() < 0.01);
}

#[test]
fn a_wider_dock_hits_the_tiles_it_added_and_nothing_past_the_last() {
    let mut bar = DockBar::new();
    bar.set_tile_count(7);
    let dock = bar.rect(SCREEN);
    let y = dock.y + dock.h / 2.0;
    for i in 0..7 {
        let x = dock.x + 21.0 + i as f32 * 65.0 + 20.0;
        assert_eq!(bar.hit_tile(SCREEN, x, y), Some(i), "tile {i}");
    }
    assert_eq!(bar.hit_tile(SCREEN, dock.x + dock.w + 1.0, y), None);
}

#[test]
fn every_dock_and_launcher_icon_is_embedded_in_both_themes() {
    use orus_core::appearance::Theme;
    use orus_shell::dock_bar::DockIcon;
    use orus_shell::icons::builtin_svg_bytes;
    for icon in [
        DockIcon::Launcher,
        DockIcon::Files,
        DockIcon::Browser,
        DockIcon::Terminal,
        DockIcon::Settings,
        DockIcon::TextEdit,
        DockIcon::Viewer,
        DockIcon::Install,
        DockIcon::Store,
        DockIcon::Office,
        DockIcon::App,
    ] {
        for theme in [Theme::Dark, Theme::Light] {
            let name = icon.name(theme);
            assert!(
                builtin_svg_bytes(std::path::Path::new(name)).is_some(),
                "{name} is not embedded"
            );
        }
    }
}
