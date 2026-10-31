use std::time::{Duration, Instant};

use orus_panel::dock_state::{DockVisibility, HIDE_DELAY};
use orus_panel::toplevels::{DESKTOP_TITLE, Toplevels};

fn states(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_ne_bytes()).collect()
}

#[test]
fn state_array_sets_the_flags_and_clears_old_ones() {
    let mut t = Toplevels::default();
    t.entry(1).set_states(&states(&[0, 2]));
    assert!(t.entry(1).maximized && t.entry(1).activated && !t.entry(1).minimized);
    t.entry(1).set_states(&states(&[1]));
    assert!(t.entry(1).minimized && !t.entry(1).maximized && !t.entry(1).activated);
    t.entry(1).set_states(&states(&[3, 99]));
    assert!(t.entry(1).fullscreen);
}

#[test]
fn a_maximized_window_covers_unless_minimized_or_the_desktop() {
    let mut t = Toplevels::default();
    assert!(!t.covers_screen());
    let desktop = t.entry(1);
    desktop.title = DESKTOP_TITLE.into();
    desktop.set_states(&states(&[0, 2]));
    assert!(!t.covers_screen(), "the desktop is always maximized");
    t.entry(2).set_states(&states(&[0]));
    assert!(t.covers_screen());
    t.entry(2).set_states(&states(&[0, 1]));
    assert!(!t.covers_screen());
    t.entry(3).set_states(&states(&[3]));
    assert!(t.covers_screen());
    t.remove(3);
    assert!(!t.covers_screen());
}

#[test]
fn the_bar_names_the_focused_app_not_the_desktop() {
    let mut t = Toplevels::default();
    t.entry(1).title = DESKTOP_TITLE.into();
    t.entry(1).set_states(&states(&[2]));
    assert_eq!(t.active_title(), None);
    t.entry(2).title = "Files".into();
    t.entry(2).set_states(&states(&[2]));
    assert_eq!(t.active_title(), Some("Files"));
}

#[test]
fn the_bar_names_chromium_not_its_page() {
    let mut t = Toplevels::default();
    t.entry(1).app_id = "chromium".into();
    t.entry(1).title = "Example Domain - Chromium".into();
    t.entry(1).set_states(&states(&[2]));
    assert_eq!(t.active_title(), Some("Chromium"));
}

#[test]
fn the_dock_stays_out_when_nothing_covers_the_screen() {
    let mut d = DockVisibility::default();
    let now = Instant::now();
    assert!(d.is_open());
    assert!(!d.tick(now + Duration::from_secs(10)));
    assert!(d.is_open());
}

#[test]
fn a_covering_window_hides_the_dock_after_the_delay() {
    let mut d = DockVisibility::default();
    let now = Instant::now();
    d.set_covered(true, now);
    assert!(d.is_open(), "not before the delay");
    assert!(!d.tick(now + HIDE_DELAY / 2));
    assert!(d.tick(now + HIDE_DELAY));
    assert!(!d.is_open());
}

#[test]
fn the_pointer_brings_the_hidden_dock_back_and_it_hides_after_leaving() {
    let mut d = DockVisibility::default();
    let t0 = Instant::now();
    d.set_covered(true, t0);
    d.tick(t0 + HIDE_DELAY);
    d.pointer_entered();
    assert!(d.is_open());
    assert!(
        !d.tick(t0 + HIDE_DELAY * 5),
        "stays while the pointer is on it"
    );
    d.pointer_left(t0 + HIDE_DELAY * 5);
    assert!(!d.tick(t0 + HIDE_DELAY * 5 + HIDE_DELAY / 2));
    assert!(d.tick(t0 + HIDE_DELAY * 6));
    assert!(!d.is_open());
}

#[test]
fn uncovering_the_screen_shows_the_dock_at_once() {
    let mut d = DockVisibility::default();
    let t0 = Instant::now();
    d.set_covered(true, t0);
    d.tick(t0 + HIDE_DELAY);
    d.set_covered(false, t0 + HIDE_DELAY * 2);
    assert!(d.is_open());
}

#[test]
fn the_pointer_returning_before_the_delay_cancels_the_hide() {
    let mut d = DockVisibility::default();
    let t0 = Instant::now();
    d.set_covered(true, t0);
    d.pointer_entered();
    d.pointer_left(t0);
    d.pointer_entered();
    assert!(!d.tick(t0 + HIDE_DELAY * 2));
    assert!(d.is_open());
}

mod dock {
    use orus_panel::dock_model::{SETTINGS_TITLE, items, tiles, window_to_raise};
    use orus_panel::toplevels::{DESKTOP_TITLE, Toplevels};
    use orus_shell::dock_bar::{DockApp, DockIcon};

    fn open(t: &mut Toplevels, id: u32, app_id: &str, title: &str) {
        let w = t.entry(id);
        w.app_id = app_id.into();
        w.title = title.into();
    }

    #[test]
    fn with_no_windows_the_dock_is_its_five_fixed_tiles_and_none_is_running() {
        let list = items(&Toplevels::default());
        assert_eq!(list.len(), 5);
        let t = tiles(&list);
        assert_eq!(t[0].running, None, "the launcher has no dot");
        assert!(t[1..].iter().all(|tile| tile.running == Some(false)));
    }

    #[test]
    fn a_window_of_a_fixed_app_marks_its_tile_running_without_adding_one() {
        let mut t = Toplevels::default();
        open(&mut t, 1, "orus-files", "Files");
        open(&mut t, 2, "orus-files", "Files");
        open(&mut t, 3, "chromium", "Chromium");
        open(&mut t, 4, "", SETTINGS_TITLE);
        let list = items(&t);
        assert_eq!(list.len(), 5);
        assert_eq!(list[1].fixed_app(), Some(DockApp::Files));
        assert_eq!(list[1].windows, vec![1, 2], "two windows share one tile");
        assert!(list[2].running() && list[4].running());
        assert!(!list[3].running(), "the terminal is not open");
    }

    #[test]
    fn other_apps_get_a_tile_each_after_the_fixed_ones_in_opening_order() {
        let mut t = Toplevels::default();
        open(&mut t, 5, "orus-viewer", "Viewer");
        open(&mut t, 6, "orus-textedit", "Text Edit");
        open(&mut t, 7, "", "react-shadcn-suite");
        open(&mut t, 8, "orus-textedit", "Text Edit");
        open(&mut t, 9, "", "Store");
        open(&mut t, 10, "libreoffice-writer", "Untitled 1");
        open(&mut t, 11, "libreoffice-calc", "Untitled 2");
        let list = items(&t);
        let icons: Vec<_> = list[5..].iter().map(|i| i.icon).collect();
        assert_eq!(
            icons,
            [
                DockIcon::Viewer,
                DockIcon::TextEdit,
                DockIcon::App,
                DockIcon::Store,
                DockIcon::Office
            ],
            "Writer and Calc share one tile"
        );
        assert_eq!(list[6].windows, vec![6, 8]);
        assert!(tiles(&list)[5..].iter().all(|t| t.running == Some(true)));
    }

    #[test]
    fn the_desktop_window_never_shows() {
        let mut t = Toplevels::default();
        open(&mut t, 1, "", DESKTOP_TITLE);
        assert_eq!(items(&t).len(), 5);
    }

    #[test]
    fn a_click_raises_the_next_window_of_the_app_or_the_first() {
        let mut t = Toplevels::default();
        open(&mut t, 1, "orus-files", "Files");
        open(&mut t, 2, "orus-files", "Files");
        open(&mut t, 3, "orus-files", "Files");
        let list = items(&t);
        let files = &list[1];
        assert_eq!(window_to_raise(files, None), Some(1));
        assert_eq!(
            window_to_raise(files, Some(99)),
            Some(1),
            "focus is elsewhere"
        );
        assert_eq!(window_to_raise(files, Some(1)), Some(2));
        assert_eq!(window_to_raise(files, Some(3)), Some(1), "wraps round");
        assert_eq!(
            window_to_raise(&list[3], None),
            None,
            "nothing open to raise"
        );
    }
}
