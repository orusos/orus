//! Where each part of the window sits. Sizes are the desktop concept's Files window
//! (910 wide), read 1:1 in logical pixels from the top-left of the content area
//! under the title bar. The sidebar is fixed; the rest follows the window size.

use jag_draw::Rect;

pub const TAB_ROW_H: f32 = 42.0;
pub const ACTIVE_TAB_W: f32 = 182.0;
pub const INACTIVE_TAB_W: f32 = 149.0;
/// Tabs start this far right of the sidebar's edge.
pub const TAB_LEFT: f32 = SIDEBAR_W + 12.0;
pub const TAB_GAP: f32 = 10.0;
pub const NEW_TAB_W: f32 = 36.0;

pub const SIDEBAR_W: f32 = 206.0;
pub const PLACE_PILL_W: f32 = 181.0;
pub const PLACE_PILL_H: f32 = 40.0;
pub const PLACE_PITCH: f32 = 42.5;
pub const PLACE_TOP: f32 = 67.0;
/// Gaps below the last place: to the separator, then the "Devices" label and the
/// first device's pill centers.
pub const SEPARATOR_GAP: f32 = 34.0;
pub const DEVICES_LABEL_GAP: f32 = 28.0;
pub const FIRST_DEVICE_GAP: f32 = 38.0;

pub const TOOLBAR_CENTER_Y: f32 = 78.0;
pub const BACK_CENTER_X: f32 = 234.0;
pub const FORWARD_CENTER_X: f32 = 270.0;
pub const NAV_HIT: f32 = 32.0;
pub const PILL_LEFT: f32 = 300.0;
pub const PILL_H: f32 = 41.0;
pub const PILL_RADIUS: f32 = 12.0;
pub const TOGGLE_BTN_W: f32 = 40.0;
pub const TOGGLE_BTN_H: f32 = 38.0;
pub const TOGGLE_GAP: f32 = 8.0;
/// Space between the view toggle and the window's right edge, and the pill and the toggle.
pub const TOOLBAR_RIGHT_PAD: f32 = 17.0;
pub const PILL_TOGGLE_GAP: f32 = 26.0;

pub const HEADING_X: f32 = 234.0;
pub const HEADING_CENTER_Y: f32 = 141.0;
pub const HEADING_SIZE: f32 = 24.0;
/// Where the heading and grid start to scroll under the toolbar.
pub const SCROLL_TOP: f32 = 108.0;

pub const GRID_TOP: f32 = 171.0;
pub const CELL_PITCH_Y: f32 = 178.0;
/// Narrowest a column goes before one fewer column is used.
pub const MIN_CELL_W: f32 = 170.0;
pub const CELL_ICON_TOP: f32 = 19.0;
pub const TILE_W: f32 = 131.0;
pub const TILE_RADIUS: f32 = 10.0;
/// From the tile's label center to its bottom edge.
pub const TILE_BELOW_LABEL: f32 = 24.0;
pub const FOLDER_SIZE: (f32, f32) = (102.0, 89.0);
pub const FILE_SIZE: (f32, f32) = (52.0, 68.0);
pub const THUMB_SIZE: (f32, f32) = (124.0, 91.0);
pub const THUMB_RADIUS: f32 = 8.0;
/// Icon bottom to label center: 28 measured under folders, 27 under files.
pub const LABEL_BELOW_ICON: f32 = 27.5;
/// Thumbnails sit closer to their label.
pub const LABEL_BELOW_THUMB: f32 = 20.0;

pub const STATUS_H: f32 = 46.0;
pub const STATUS_PAD: f32 = 25.0;

/// Space kept under the last row.
pub const BOTTOM_PAD: f32 = 20.0;

pub const LIST_ROW_H: f32 = 40.0;
/// The list starts this far above where the grid starts.
pub const LIST_TOP_LIFT: f32 = 20.0;

/// What a grid cell shows, which decides its icon size and label position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellKind {
    Folder,
    File,
    Thumbnail,
}

impl CellKind {
    pub fn icon_size(self) -> (f32, f32) {
        match self {
            CellKind::Folder => FOLDER_SIZE,
            CellKind::File => FILE_SIZE,
            CellKind::Thumbnail => THUMB_SIZE,
        }
    }

    /// Cell top to label center.
    pub fn label_center(self) -> f32 {
        let gap = if self == CellKind::Thumbnail {
            LABEL_BELOW_THUMB
        } else {
            LABEL_BELOW_ICON
        };
        CELL_ICON_TOP + self.icon_size().1 + gap
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

pub fn contains(r: Rect, x: f32, y: f32) -> bool {
    x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
}

/// The tab at `index` of `count`, `active` being the open one (wider).
pub fn tab_rect(index: usize, active: usize) -> Rect {
    let mut x = TAB_LEFT;
    for i in 0..index {
        x += if i == active {
            ACTIVE_TAB_W
        } else {
            INACTIVE_TAB_W
        } + TAB_GAP;
    }
    let w = if index == active {
        ACTIVE_TAB_W
    } else {
        INACTIVE_TAB_W
    };
    rect(x, 2.0, w, TAB_ROW_H - 2.0)
}

/// The tab's close button, a square at its right end.
pub fn tab_close_rect(tab: Rect) -> Rect {
    rect(tab.x + tab.w - 32.0, tab.y + 6.0, 24.0, 24.0)
}

pub fn new_tab_rect(count: usize, active: usize) -> Rect {
    let last = tab_rect(count - 1, active);
    rect(last.x + last.w + TAB_GAP, 4.0, NEW_TAB_W, TAB_ROW_H - 6.0)
}

/// The pill behind a sidebar item whose center is `center_y`.
pub fn place_pill(center_y: f32) -> Rect {
    rect(
        12.0,
        center_y - PLACE_PILL_H * 0.5,
        PLACE_PILL_W,
        PLACE_PILL_H,
    )
}

/// Vertical positions in the sidebar for `favorites` places and `devices` devices.
pub struct Sidebar {
    pub favorite_centers: Vec<f32>,
    pub separator_y: f32,
    pub devices_label_y: f32,
    pub device_centers: Vec<f32>,
}

pub fn sidebar(favorites: usize, devices: usize) -> Sidebar {
    let favorite_centers: Vec<f32> = (0..favorites)
        .map(|i| PLACE_TOP + PLACE_PILL_H * 0.5 + i as f32 * PLACE_PITCH)
        .collect();
    let last = favorite_centers.last().copied().unwrap_or(PLACE_TOP);
    let separator_y = last + SEPARATOR_GAP;
    let devices_label_y = separator_y + DEVICES_LABEL_GAP;
    let first = devices_label_y + FIRST_DEVICE_GAP;
    let device_centers = (0..devices)
        .map(|i| first + i as f32 * PLACE_PITCH)
        .collect();
    Sidebar {
        favorite_centers,
        separator_y,
        devices_label_y,
        device_centers,
    }
}

pub struct Toolbar {
    pub back: Rect,
    pub forward: Rect,
    pub pill: Rect,
    pub grid_btn: Rect,
    pub list_btn: Rect,
}

pub fn toolbar(w: f32) -> Toolbar {
    let list_x = w - TOOLBAR_RIGHT_PAD - TOGGLE_BTN_W;
    let grid_x = list_x - TOGGLE_GAP - TOGGLE_BTN_W;
    let btn = |x| {
        rect(
            x,
            TOOLBAR_CENTER_Y - TOGGLE_BTN_H * 0.5,
            TOGGLE_BTN_W,
            TOGGLE_BTN_H,
        )
    };
    let hit = |cx: f32| {
        rect(
            cx - NAV_HIT * 0.5,
            TOOLBAR_CENTER_Y - NAV_HIT * 0.5,
            NAV_HIT,
            NAV_HIT,
        )
    };
    Toolbar {
        back: hit(BACK_CENTER_X),
        forward: hit(FORWARD_CENTER_X),
        pill: rect(
            PILL_LEFT,
            TOOLBAR_CENTER_Y - PILL_H * 0.5,
            (grid_x - PILL_TOGGLE_GAP - PILL_LEFT).max(0.0),
            PILL_H,
        ),
        grid_btn: btn(grid_x),
        list_btn: btn(list_x),
    }
}

/// The search icon's hit area, at the right end of the path pill.
pub fn search_rect(pill: Rect) -> Rect {
    rect(pill.x + pill.w - 48.0, pill.y, 48.0, pill.h)
}

/// Columns of the grid for a window `w` wide.
pub fn grid_columns(w: f32) -> usize {
    (((w - SIDEBAR_W) / MIN_CELL_W).floor() as usize).max(1)
}

/// The cell of entry number `position` (among those shown), before scrolling.
pub fn cell_rect(position: usize, w: f32) -> Rect {
    let cols = grid_columns(w);
    let cell_w = (w - SIDEBAR_W) / cols as f32;
    let (col, row) = (position % cols, position / cols);
    rect(
        SIDEBAR_W + col as f32 * cell_w,
        GRID_TOP + row as f32 * CELL_PITCH_Y,
        cell_w,
        CELL_PITCH_Y,
    )
}

/// The selection tile for a cell of `kind`: centered in the cell.
pub fn tile_rect(cell: Rect, kind: CellKind) -> Rect {
    rect(
        cell.x + (cell.w - TILE_W) * 0.5,
        cell.y,
        TILE_W,
        kind.label_center() + TILE_BELOW_LABEL,
    )
}

/// Height of everything under the toolbar for `count` entries in the grid.
pub fn grid_content_height(count: usize, w: f32) -> f32 {
    let rows = count.div_ceil(grid_columns(w)).max(1);
    GRID_TOP + rows as f32 * CELL_PITCH_Y - BOTTOM_PAD
}

/// The row of entry number `position` in the list view, before scrolling.
pub fn list_row(position: usize, w: f32) -> Rect {
    rect(
        SIDEBAR_W + 12.0,
        GRID_TOP - LIST_TOP_LIFT + position as f32 * LIST_ROW_H,
        w - SIDEBAR_W - 24.0,
        LIST_ROW_H,
    )
}

pub fn list_content_height(count: usize) -> f32 {
    GRID_TOP - LIST_TOP_LIFT + count as f32 * LIST_ROW_H + BOTTOM_PAD
}
