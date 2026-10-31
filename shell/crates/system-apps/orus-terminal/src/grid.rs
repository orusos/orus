/// Per-cell style attributes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellAttrs {
    pub fg: [u8; 3],
    pub bg: [u8; 3],
    pub flags: u8,
}

impl CellAttrs {
    pub const BOLD: u8 = 1;
    pub const DIM: u8 = 2;
    pub const ITALIC: u8 = 4;
    pub const UNDERLINE: u8 = 8;
    pub const INVERSE: u8 = 16;

    pub const DEFAULT_FG: [u8; 3] = [204, 204, 204];
    pub const DEFAULT_BG: [u8; 3] = [30, 30, 30];
}

impl Default for CellAttrs {
    fn default() -> Self {
        Self {
            fg: Self::DEFAULT_FG,
            bg: Self::DEFAULT_BG,
            flags: 0,
        }
    }
}

/// A single character cell.
#[derive(Clone, Debug)]
pub struct Cell {
    pub c: char,
    pub attrs: CellAttrs,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            c: ' ',
            attrs: CellAttrs::default(),
        }
    }
}

pub type Row = Vec<Cell>;

fn blank_row(cols: usize) -> Row {
    vec![Cell::default(); cols]
}

/// Grid of character cells with scrollback buffer.
pub struct Grid {
    pub cells: Vec<Row>,
    pub scrollback: Vec<Row>,
    pub rows: usize,
    pub cols: usize,
}

impl Grid {
    pub fn new(cols: usize, rows: usize) -> Self {
        let cells = (0..rows).map(|_| blank_row(cols)).collect();
        Self {
            cells,
            scrollback: Vec::new(),
            rows,
            cols,
        }
    }

    pub fn cell(&self, row: usize, col: usize) -> &Cell {
        &self.cells[row][col]
    }

    pub fn cell_mut(&mut self, row: usize, col: usize) -> &mut Cell {
        &mut self.cells[row][col]
    }

    pub fn put(&mut self, row: usize, col: usize, c: char, attrs: CellAttrs) {
        if row < self.rows && col < self.cols {
            self.cells[row][col] = Cell { c, attrs };
        }
    }

    /// Scroll top n rows into scrollback, add blank rows at bottom.
    pub fn scroll_up(&mut self, n: usize) {
        let n = n.min(self.rows);
        for row in self.cells.drain(..n) {
            self.scrollback.push(row);
        }
        for _ in 0..n {
            self.cells.push(blank_row(self.cols));
        }
    }

    /// Scroll within a region: move rows [top..=bottom] up by n, discard top.
    pub fn scroll_region_up(&mut self, top: usize, bottom: usize, n: usize) {
        let n = n.min(bottom - top + 1);
        for _ in 0..n {
            self.cells.remove(top);
            self.cells.insert(bottom, blank_row(self.cols));
        }
    }

    /// Scroll within a region: move rows [top..=bottom] down by n.
    pub fn scroll_region_down(&mut self, top: usize, bottom: usize, n: usize) {
        let n = n.min(bottom - top + 1);
        for _ in 0..n {
            self.cells.remove(bottom);
            self.cells.insert(top, blank_row(self.cols));
        }
    }

    /// ED — erase in display.
    pub fn erase_display(&mut self, mode: u16, cursor_row: usize, cursor_col: usize) {
        match mode {
            0 => {
                // Below cursor
                self.erase_line(cursor_row, 0, cursor_col);
                for r in (cursor_row + 1)..self.rows {
                    self.cells[r] = blank_row(self.cols);
                }
            }
            1 => {
                // Above cursor
                for r in 0..cursor_row {
                    self.cells[r] = blank_row(self.cols);
                }
                self.erase_line(cursor_row, 1, cursor_col);
            }
            2 | 3 => {
                // Entire screen (3 also clears scrollback)
                for r in 0..self.rows {
                    self.cells[r] = blank_row(self.cols);
                }
                if mode == 3 {
                    self.scrollback.clear();
                }
            }
            _ => {}
        }
    }

    /// EL — erase in line.
    pub fn erase_line(&mut self, row: usize, mode: u16, cursor_col: usize) {
        if row >= self.rows {
            return;
        }
        match mode {
            0 => {
                // Right of cursor (inclusive)
                for c in cursor_col..self.cols {
                    self.cells[row][c] = Cell::default();
                }
            }
            1 => {
                // Left of cursor (inclusive)
                for c in 0..=cursor_col.min(self.cols - 1) {
                    self.cells[row][c] = Cell::default();
                }
            }
            2 => {
                self.cells[row] = blank_row(self.cols);
            }
            _ => {}
        }
    }

    /// ICH — insert blank characters at position, shifting right.
    pub fn insert_chars(&mut self, row: usize, col: usize, n: usize) {
        if row >= self.rows {
            return;
        }
        let r = &mut self.cells[row];
        for _ in 0..n {
            if col < r.len() {
                r.insert(col, Cell::default());
                r.truncate(self.cols);
            }
        }
    }

    /// DCH — delete characters at position, shifting left.
    pub fn delete_chars(&mut self, row: usize, col: usize, n: usize) {
        if row >= self.rows {
            return;
        }
        let r = &mut self.cells[row];
        let n = n.min(r.len().saturating_sub(col));
        for _ in 0..n {
            if col < r.len() {
                r.remove(col);
                r.push(Cell::default());
            }
        }
    }

    pub fn resize(&mut self, new_cols: usize, new_rows: usize) {
        for row in &mut self.cells {
            row.resize(new_cols, Cell::default());
        }
        while self.cells.len() < new_rows {
            self.cells.push(blank_row(new_cols));
        }
        while self.cells.len() > new_rows {
            self.cells.pop();
        }
        self.rows = new_rows;
        self.cols = new_cols;
    }

    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }
}
