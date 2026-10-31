//! Clicks, keys and scrolling.

use std::path::PathBuf;

use jag_draw::Rect;

use crate::layout::{self, contains};
use crate::model::ViewMode;
use crate::{
    ContextAction, ContextMenu, EditMode, FileManagerApp, FileManagerInput, FileManagerKey,
};

pub const MENU_W: f32 = 168.0;
pub const MENU_ITEM_H: f32 = 34.0;
pub const MENU_PAD: f32 = 6.0;

enum Hit {
    Tab(usize),
    TabClose(usize),
    NewTab,
    Favorite(usize),
    Device(usize),
    Back,
    Forward,
    Crumb(PathBuf),
    Search,
    Grid,
    List,
    Entry(usize),
    Empty,
    Nothing,
}

impl FileManagerApp {
    pub fn handle_input(&mut self, input: FileManagerInput) -> bool {
        match input {
            FileManagerInput::Click { x, y } => self.click(x, y),
            FileManagerInput::DoubleClick { x, y } => self.double_click(x, y),
            FileManagerInput::RightClick { x, y } => self.right_click(x, y),
            FileManagerInput::KeyPress(key) => self.key(key),
            FileManagerInput::Scroll { dy } => self.scroll(dy),
            FileManagerInput::Resize { w, h } => {
                self.update_viewport(w, h);
                true
            }
        }
    }

    /// The context menu's items for its target.
    pub(crate) fn menu_items(menu: &ContextMenu) -> [(&'static str, ContextAction); 2] {
        match menu.target {
            Some(_) => [
                ("Rename", ContextAction::Rename),
                ("Delete", ContextAction::Delete),
            ],
            None => [
                ("New folder", ContextAction::NewFolder),
                ("New file", ContextAction::NewFile),
            ],
        }
    }

    pub(crate) fn menu_rect(menu: &ContextMenu) -> Rect {
        Rect {
            x: menu.x,
            y: menu.y,
            w: MENU_W,
            h: MENU_ITEM_H * 2.0 + MENU_PAD * 2.0,
        }
    }

    fn menu_hit(menu: &ContextMenu, x: f32, y: f32) -> Option<ContextAction> {
        let rect = Self::menu_rect(menu);
        if !contains(rect, x, y) {
            return None;
        }
        let row = ((y - rect.y - MENU_PAD) / MENU_ITEM_H).floor();
        Self::menu_items(menu)
            .get(row.max(0.0) as usize)
            .map(|i| i.1)
    }

    /// The scrolled rect of entry number `position` among those shown.
    pub(crate) fn item_rect(&self, position: usize) -> Rect {
        let mut rect = match self.view {
            ViewMode::Grid => layout::cell_rect(position, self.viewport_w),
            ViewMode::List => layout::list_row(position, self.viewport_w),
        };
        rect.y -= self.tab().scroll_y;
        rect
    }

    fn content_bottom(&self) -> f32 {
        self.viewport_h - layout::STATUS_H
    }

    fn hit(&self, x: f32, y: f32) -> Hit {
        let count = self.tabs.len();
        if y < layout::TAB_ROW_H && x >= layout::SIDEBAR_W {
            for i in 0..count {
                let tab = layout::tab_rect(i, self.active);
                if contains(layout::tab_close_rect(tab), x, y) {
                    return Hit::TabClose(i);
                }
                if contains(tab, x, y) {
                    return Hit::Tab(i);
                }
            }
            if contains(layout::new_tab_rect(count, self.active), x, y) {
                return Hit::NewTab;
            }
            return Hit::Nothing;
        }
        if x < layout::SIDEBAR_W {
            let side = layout::sidebar(self.places.favorites.len(), self.places.devices.len());
            for (i, cy) in side.favorite_centers.iter().enumerate() {
                if contains(layout::place_pill(*cy), x, y) {
                    return Hit::Favorite(i);
                }
            }
            for (i, cy) in side.device_centers.iter().enumerate() {
                if contains(layout::place_pill(*cy), x, y) {
                    return Hit::Device(i);
                }
            }
            return Hit::Nothing;
        }
        if y < layout::SCROLL_TOP {
            let bar = layout::toolbar(self.viewport_w);
            if contains(bar.back, x, y) {
                return Hit::Back;
            }
            if contains(bar.forward, x, y) {
                return Hit::Forward;
            }
            if contains(bar.grid_btn, x, y) {
                return Hit::Grid;
            }
            if contains(bar.list_btn, x, y) {
                return Hit::List;
            }
            if contains(layout::search_rect(bar.pill), x, y) {
                return Hit::Search;
            }
            if self.search.is_none() {
                for (rect, path) in self.crumb_hits.borrow().iter() {
                    if contains(*rect, x, y) {
                        return Hit::Crumb(path.clone());
                    }
                }
            }
            return Hit::Nothing;
        }
        if y >= self.content_bottom() {
            return Hit::Nothing;
        }
        for (position, entry) in self.visible.iter().enumerate() {
            if contains(self.item_rect(position), x, y) {
                return Hit::Entry(*entry);
            }
        }
        Hit::Empty
    }

    fn click(&mut self, x: f32, y: f32) -> bool {
        if let Some(menu) = self.context_menu.take() {
            if let Some(action) = Self::menu_hit(&menu, x, y) {
                self.run_context_action(action, menu.target);
            }
            return true;
        }
        match self.hit(x, y) {
            Hit::Tab(i) => self.switch_tab(i),
            Hit::TabClose(i) => self.close_tab(i),
            Hit::NewTab => self.new_tab(),
            Hit::Favorite(i) => {
                let path = self.places.favorites[i].path.clone();
                self.navigate(&path);
            }
            Hit::Device(i) => {
                let path = self.places.devices[i].path.clone();
                self.navigate(&path);
            }
            Hit::Back => self.go_back(),
            Hit::Forward => self.go_forward(),
            Hit::Crumb(path) => self.navigate(&path),
            Hit::Search => {
                self.search = match self.search {
                    Some(_) => None,
                    None => Some(String::new()),
                };
                self.apply_search();
            }
            Hit::Grid => self.view = ViewMode::Grid,
            Hit::List => self.view = ViewMode::List,
            Hit::Entry(i) => {
                self.edit_mode = EditMode::None;
                self.tab_mut().selected = Some(i);
            }
            Hit::Empty => {
                self.edit_mode = EditMode::None;
                self.tab_mut().selected = None;
            }
            Hit::Nothing => {}
        }
        true
    }

    fn double_click(&mut self, x: f32, y: f32) -> bool {
        if let Hit::Entry(i) = self.hit(x, y) {
            self.open_entry(i);
            return true;
        }
        false
    }

    fn right_click(&mut self, x: f32, y: f32) -> bool {
        let target = match self.hit(x, y) {
            Hit::Entry(i) => {
                self.tab_mut().selected = Some(i);
                Some(i)
            }
            Hit::Empty => None,
            _ => return false,
        };
        self.context_menu = Some(ContextMenu { x, y, target });
        true
    }

    /// The selected entry's place among those shown.
    fn selected_position(&self) -> Option<usize> {
        let selected = self.tab().selected?;
        self.visible.iter().position(|&i| i == selected)
    }

    fn select_position(&mut self, position: usize) {
        if let Some(&entry) = self.visible.get(position) {
            self.tab_mut().selected = Some(entry);
            self.ensure_visible(position);
        }
    }

    fn move_selection(&mut self, delta: isize) {
        if self.visible.is_empty() {
            return;
        }
        let next = match self.selected_position() {
            Some(p) => (p as isize + delta).clamp(0, self.visible.len() as isize - 1) as usize,
            None => 0,
        };
        self.select_position(next);
    }

    fn key(&mut self, key: FileManagerKey) -> bool {
        if self.edit_key(&key) {
            return true;
        }
        if let Some(query) = self.search.as_mut() {
            match &key {
                FileManagerKey::Char(c) => query.push(*c),
                FileManagerKey::Backspace => {
                    query.pop();
                }
                FileManagerKey::Escape => self.search = None,
                _ => return self.navigation_key(key),
            }
            self.apply_search();
            self.cursor_epoch = std::time::Instant::now();
            return true;
        }
        self.navigation_key(key)
    }

    fn navigation_key(&mut self, key: FileManagerKey) -> bool {
        let cols = match self.view {
            ViewMode::Grid => layout::grid_columns(self.viewport_w) as isize,
            ViewMode::List => 1,
        };
        match key {
            FileManagerKey::ArrowUp => self.move_selection(-cols),
            FileManagerKey::ArrowDown => self.move_selection(cols),
            FileManagerKey::ArrowLeft => self.move_selection(-1),
            FileManagerKey::ArrowRight => self.move_selection(1),
            FileManagerKey::Enter => {
                if let Some(i) = self.tab().selected {
                    self.open_entry(i);
                }
            }
            FileManagerKey::Backspace => self.go_up(),
            FileManagerKey::F2 => {
                if let Some(index) = self.tab().selected {
                    self.run_context_action(ContextAction::Rename, Some(index));
                }
            }
            FileManagerKey::Delete => {
                if let Some(index) = self.tab().selected {
                    self.run_context_action(ContextAction::Delete, Some(index));
                }
            }
            _ => return false,
        }
        true
    }

    fn max_scroll(&self) -> f32 {
        let content = match self.view {
            ViewMode::Grid => layout::grid_content_height(self.visible.len(), self.viewport_w),
            ViewMode::List => layout::list_content_height(self.visible.len()),
        };
        (content - self.content_bottom()).max(0.0)
    }

    fn scroll(&mut self, dy: f32) -> bool {
        let max = self.max_scroll();
        let tab = self.tab_mut();
        tab.scroll_y = (tab.scroll_y - dy * 30.0).clamp(0.0, max);
        true
    }

    fn ensure_visible(&mut self, position: usize) {
        let rect = self.item_rect(position);
        let (top, bottom) = (layout::SCROLL_TOP, self.content_bottom());
        let max = self.max_scroll();
        let tab = self.tab_mut();
        if rect.y < top {
            tab.scroll_y = (tab.scroll_y - (top - rect.y)).max(0.0);
        } else if rect.y + rect.h > bottom {
            tab.scroll_y = (tab.scroll_y + rect.y + rect.h - bottom).min(max);
        }
    }
}
