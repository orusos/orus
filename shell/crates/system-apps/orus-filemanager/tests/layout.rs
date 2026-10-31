use orus_filemanager::layout::*;

const CONCEPT_W: f32 = 910.0;

#[test]
fn concept_window_has_four_columns_centered_on_the_concept_columns() {
    assert_eq!(grid_columns(CONCEPT_W), 4);
    let centers: Vec<f32> = (0..4)
        .map(|i| {
            let c = cell_rect(i, CONCEPT_W);
            c.x + c.w * 0.5
        })
        .collect();
    // The concept's folder columns, read from the screenshot.
    for (got, want) in centers.iter().zip([295.0, 470.0, 644.0, 819.0]) {
        assert!((got - want).abs() <= 3.0, "{got} vs {want}");
    }
    assert_eq!(cell_rect(4, CONCEPT_W).y, GRID_TOP + CELL_PITCH_Y);
}

#[test]
fn narrow_windows_drop_columns_and_never_reach_zero() {
    assert_eq!(grid_columns(206.0 + 3.0 * MIN_CELL_W), 3);
    assert_eq!(grid_columns(300.0), 1);
    assert_eq!(grid_columns(0.0), 1);
}

#[test]
fn tabs_sit_side_by_side_the_open_one_wider() {
    let first = tab_rect(0, 0);
    let second = tab_rect(1, 0);
    assert_eq!((first.x, first.w), (SIDEBAR_W + 12.0, ACTIVE_TAB_W));
    assert_eq!((second.x, second.w), (SIDEBAR_W + 204.0, INACTIVE_TAB_W));
    let close = tab_close_rect(first);
    assert!(contains(first, close.x, close.y) && close.x + close.w <= first.x + first.w);
    assert!(new_tab_rect(2, 0).x > second.x + second.w);
}

#[test]
fn toolbar_keeps_the_pill_and_toggle_inside_the_window() {
    for w in [700.0, CONCEPT_W, 1400.0] {
        let bar = toolbar(w);
        assert_eq!(bar.list_btn.x + bar.list_btn.w + TOOLBAR_RIGHT_PAD, w);
        assert!(bar.pill.x + bar.pill.w + PILL_TOGGLE_GAP <= bar.grid_btn.x + 0.001);
        assert!(bar.pill.w > 0.0);
    }
}

#[test]
fn sidebar_starts_at_the_concept_and_devices_follow_the_separator() {
    let side = sidebar(4, 2);
    assert_eq!(side.favorite_centers[0], 87.0);
    assert_eq!(
        side.favorite_centers[1] - side.favorite_centers[0],
        PLACE_PITCH
    );
    assert!(side.separator_y > side.favorite_centers[3]);
    assert!(side.devices_label_y > side.separator_y);
    assert!(side.device_centers[0] > side.devices_label_y);
    assert_eq!(sidebar(0, 1).favorite_centers.len(), 0);
}

#[test]
fn labels_sit_below_their_icons_and_tiles_wrap_the_label() {
    assert!(CellKind::Folder.label_center() > CELL_ICON_TOP + FOLDER_SIZE.1);
    assert!(CellKind::Thumbnail.label_center() - CELL_ICON_TOP - THUMB_SIZE.1 < LABEL_BELOW_ICON);
    let cell = cell_rect(0, CONCEPT_W);
    let tile = tile_rect(cell, CellKind::File);
    assert_eq!(tile.w, TILE_W);
    assert_eq!(tile.h, CellKind::File.label_center() + TILE_BELOW_LABEL);
    // The concept's file tile is 138 tall.
    assert!((tile.h - 138.0).abs() <= 2.0);
}

#[test]
fn content_height_grows_a_row_at_a_time() {
    let one_row = grid_content_height(4, CONCEPT_W);
    let two_rows = grid_content_height(5, CONCEPT_W);
    assert_eq!(two_rows - one_row, CELL_PITCH_Y);
    assert_eq!(grid_content_height(0, CONCEPT_W), one_row);
}
