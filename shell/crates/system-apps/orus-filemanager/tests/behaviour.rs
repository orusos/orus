use std::fs;
use std::path::Path;
use std::sync::Arc;

use jag_draw::{RasterizedGlyph, TextProvider};
use orus_filemanager::layout;
use orus_filemanager::{FileManagerApp, FileManagerInput as In, FileManagerKey as K};

struct NoText;

impl TextProvider for NoText {
    fn rasterize_run(&self, _: &jag_draw::TextRun) -> Vec<RasterizedGlyph> {
        Vec::new()
    }
}

/// A home with Documents, Projects/{Design/, notes.md}, and a.txt, b.txt.
fn home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let p = home.path();
    fs::create_dir_all(p.join("Documents")).unwrap();
    fs::create_dir_all(p.join("Projects/Design")).unwrap();
    fs::write(p.join("Projects/notes.md"), "x").unwrap();
    fs::write(p.join("a.txt"), "x").unwrap();
    fs::write(p.join("b.txt"), "x").unwrap();
    home
}

fn app(home: &Path) -> FileManagerApp {
    FileManagerApp::new(home.to_path_buf(), Arc::new(NoText))
}

fn click(app: &mut FileManagerApp, x: f32, y: f32) {
    app.handle_input(In::Click { x, y });
}

fn click_cell(app: &mut FileManagerApp, position: usize) {
    let cell = layout::cell_rect(position, 910.0);
    click(app, cell.x + cell.w * 0.5, cell.y + 40.0);
}

#[test]
fn starts_in_home_with_folders_before_files() {
    let h = home();
    let app = app(h.path());
    assert_eq!(app.current_dir(), h.path());
    assert_eq!(
        app.visible_names(),
        ["Documents", "Projects", "a.txt", "b.txt"]
    );
    assert_eq!(app.tab_count(), 1);
}

#[test]
fn a_sidebar_place_opens_and_back_returns() {
    let h = home();
    let mut app = app(h.path());
    let side = layout::sidebar(2, 1);
    // Favorites are Home and Documents (the folders that exist).
    click(&mut app, 100.0, side.favorite_centers[1]);
    assert_eq!(app.current_dir(), h.path().join("Documents"));
    click(&mut app, layout::BACK_CENTER_X, layout::TOOLBAR_CENTER_Y);
    assert_eq!(app.current_dir(), h.path());
    click(&mut app, layout::FORWARD_CENTER_X, layout::TOOLBAR_CENTER_Y);
    assert_eq!(app.current_dir(), h.path().join("Documents"));
}

#[test]
fn double_click_opens_a_folder_and_backspace_goes_up() {
    let h = home();
    let mut app = app(h.path());
    let cell = layout::cell_rect(1, 910.0);
    app.handle_input(In::DoubleClick {
        x: cell.x + 60.0,
        y: cell.y + 40.0,
    });
    assert_eq!(app.current_dir(), h.path().join("Projects"));
    assert_eq!(app.visible_names(), ["Design", "notes.md"]);
    app.handle_input(In::KeyPress(K::Backspace));
    assert_eq!(app.current_dir(), h.path());
}

#[test]
fn clicking_selects_and_empty_space_clears() {
    let h = home();
    let mut app = app(h.path());
    click_cell(&mut app, 2);
    assert_eq!(app.selected_name(), Some("a.txt"));
    let empty = layout::cell_rect(6, 910.0);
    click(&mut app, empty.x + 20.0, empty.y + 20.0);
    assert_eq!(app.selected_name(), None);
}

#[test]
fn arrow_keys_move_by_one_and_by_a_row() {
    let h = home();
    let mut app = app(h.path());
    app.handle_input(In::KeyPress(K::ArrowRight));
    assert_eq!(app.selected_name(), Some("Documents"));
    app.handle_input(In::KeyPress(K::ArrowRight));
    assert_eq!(app.selected_name(), Some("Projects"));
    app.handle_input(In::KeyPress(K::ArrowDown));
    // One row down from column 1 is past the end of the 4 entries: stays on the last.
    assert_eq!(app.selected_name(), Some("b.txt"));
    app.handle_input(In::KeyPress(K::ArrowLeft));
    assert_eq!(app.selected_name(), Some("a.txt"));
}

#[test]
fn tabs_open_switch_and_close_but_the_last_one_stays() {
    let h = home();
    let mut app = app(h.path());
    click(&mut app, layout::new_tab_rect(1, 0).x + 10.0, 20.0);
    assert_eq!(app.tab_count(), 2);
    // The new tab is open; go into Projects there, then switch to the first tab.
    let cell = layout::cell_rect(1, 910.0);
    app.handle_input(In::DoubleClick {
        x: cell.x + 60.0,
        y: cell.y + 40.0,
    });
    let first = layout::tab_rect(0, 1);
    click(&mut app, first.x + 20.0, 20.0);
    assert_eq!(app.current_dir(), h.path());
    // Close the second tab through its close button.
    let second = layout::tab_rect(1, 0);
    let close = layout::tab_close_rect(second);
    click(&mut app, close.x + 5.0, close.y + 5.0);
    assert_eq!(app.tab_count(), 1);
    let only = layout::tab_rect(0, 0);
    let close = layout::tab_close_rect(only);
    click(&mut app, close.x + 5.0, close.y + 5.0);
    assert_eq!(app.tab_count(), 1);
}

#[test]
fn search_filters_by_name_and_escape_restores() {
    let h = home();
    let mut app = app(h.path());
    let pill = layout::toolbar(910.0).pill;
    let search = layout::search_rect(pill);
    click(&mut app, search.x + 10.0, search.y + 10.0);
    for c in "TXT".chars() {
        app.handle_input(In::KeyPress(K::Char(c)));
    }
    assert_eq!(app.visible_names(), ["a.txt", "b.txt"]);
    app.handle_input(In::KeyPress(K::Backspace));
    app.handle_input(In::KeyPress(K::Backspace));
    app.handle_input(In::KeyPress(K::Backspace));
    app.handle_input(In::KeyPress(K::Char('P')));
    assert_eq!(app.visible_names(), ["Projects"]);
    app.handle_input(In::KeyPress(K::Escape));
    assert_eq!(app.visible_names().len(), 4);
}

#[test]
fn rename_changes_the_file_and_refuses_a_name_with_a_slash() {
    let h = home();
    let mut app = app(h.path());
    click_cell(&mut app, 2);
    app.handle_input(In::KeyPress(K::F2));
    app.handle_input(In::KeyPress(K::Backspace)); // a.tx
    app.handle_input(In::KeyPress(K::Char('t')));
    app.handle_input(In::KeyPress(K::Char('/')));
    app.handle_input(In::KeyPress(K::Enter));
    assert!(
        h.path().join("a.txt").exists(),
        "a name with a slash is refused"
    );
    click_cell(&mut app, 2);
    app.handle_input(In::KeyPress(K::F2));
    app.handle_input(In::KeyPress(K::Char('2')));
    app.handle_input(In::KeyPress(K::Enter));
    assert!(h.path().join("a.txt2").exists());
    assert!(!h.path().join("a.txt").exists());
}

#[test]
fn right_click_on_empty_space_creates_a_folder() {
    let h = home();
    let mut app = app(h.path());
    let empty = layout::cell_rect(6, 910.0);
    let (x, y) = (empty.x + 20.0, empty.y + 20.0);
    app.handle_input(In::RightClick { x, y });
    // The menu's first row is "New folder".
    click(&mut app, x + 20.0, y + 16.0);
    for c in "Fresh".chars() {
        app.handle_input(In::KeyPress(K::Char(c)));
    }
    app.handle_input(In::KeyPress(K::Enter));
    assert!(h.path().join("Fresh").is_dir());
    assert!(app.visible_names().contains(&"Fresh"));
}

#[test]
fn delete_needs_a_confirming_enter() {
    let h = home();
    let mut app = app(h.path());
    click_cell(&mut app, 3);
    app.handle_input(In::KeyPress(K::Delete));
    app.handle_input(In::KeyPress(K::Escape));
    assert!(h.path().join("b.txt").exists());
    app.handle_input(In::KeyPress(K::Delete));
    app.handle_input(In::KeyPress(K::Enter));
    assert!(!h.path().join("b.txt").exists());
}
