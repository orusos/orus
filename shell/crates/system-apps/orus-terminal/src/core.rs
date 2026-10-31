use std::io;
use std::path::Path;

use crate::grid::{CellAttrs, Grid};
use crate::pty::Pty;

/// Key events that can be sent to the terminal.
pub enum TerminalKey {
    Char(char),
    Enter,
    Backspace,
    Tab,
    Escape,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    Ctrl(char),
}

impl TerminalKey {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Char(c) => {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            }
            Self::Enter => vec![b'\r'],
            Self::Backspace => vec![0x7f],
            Self::Tab => vec![b'\t'],
            Self::Escape => vec![0x1b],
            Self::ArrowUp => b"\x1b[A".to_vec(),
            Self::ArrowDown => b"\x1b[B".to_vec(),
            Self::ArrowRight => b"\x1b[C".to_vec(),
            Self::ArrowLeft => b"\x1b[D".to_vec(),
            Self::Home => b"\x1b[H".to_vec(),
            Self::End => b"\x1b[F".to_vec(),
            Self::PageUp => b"\x1b[5~".to_vec(),
            Self::PageDown => b"\x1b[6~".to_vec(),
            Self::Delete => b"\x1b[3~".to_vec(),
            Self::Ctrl(c) => {
                let byte = (c.to_ascii_uppercase() as u8) & 0x1f;
                vec![byte]
            }
        }
    }
}

// -- 256-color palette lookup --------------------------------------------------

fn ansi_color(idx: u8) -> [u8; 3] {
    match idx {
        0 => [0, 0, 0],
        1 => [205, 0, 0],
        2 => [0, 205, 0],
        3 => [205, 205, 0],
        4 => [0, 0, 238],
        5 => [205, 0, 205],
        6 => [0, 205, 205],
        7 => [229, 229, 229],
        8 => [127, 127, 127],
        9 => [255, 0, 0],
        10 => [0, 255, 0],
        11 => [255, 255, 0],
        12 => [92, 92, 255],
        13 => [255, 0, 255],
        14 => [0, 255, 255],
        15 => [255, 255, 255],
        // 6x6x6 color cube (indices 16..=231)
        16..=231 => {
            let idx = idx - 16;
            let b = (idx % 6) * 51;
            let g = ((idx / 6) % 6) * 51;
            let r = (idx / 36) * 51;
            [r, g, b]
        }
        // Grayscale ramp (indices 232..=255)
        _ => {
            let v = 8 + (idx - 232) * 10;
            [v, v, v]
        }
    }
}

// -- TerminalState (implements vte::Perform) ------------------------------------

pub struct TerminalState {
    pub grid: Grid,
    pub cursor: (usize, usize), // (row, col)
    saved_cursor: (usize, usize),
    pub attrs: CellAttrs,
    scroll_top: usize,
    scroll_bottom: usize,
    pub cursor_visible: bool,
    alt_grid: Option<Box<Grid>>,
    alt_cursor: (usize, usize),
}

impl TerminalState {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            grid: Grid::new(cols, rows),
            cursor: (0, 0),
            saved_cursor: (0, 0),
            attrs: CellAttrs::default(),
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            cursor_visible: true,
            alt_grid: None,
            alt_cursor: (0, 0),
        }
    }

    fn clamp_cursor(&mut self) {
        self.cursor.0 = self.cursor.0.min(self.grid.rows.saturating_sub(1));
        self.cursor.1 = self.cursor.1.min(self.grid.cols.saturating_sub(1));
    }

    fn linefeed(&mut self) {
        let (row, _col) = self.cursor;
        if row == self.scroll_bottom {
            // At bottom of scroll region — scroll up
            if self.scroll_top == 0 && self.scroll_bottom == self.grid.rows - 1 {
                self.grid.scroll_up(1);
            } else {
                self.grid
                    .scroll_region_up(self.scroll_top, self.scroll_bottom, 1);
            }
        } else if row + 1 < self.grid.rows {
            self.cursor.0 += 1;
        }
    }

    fn sgr(&mut self, params: &[u16]) {
        if params.is_empty() {
            self.attrs = CellAttrs::default();
            return;
        }
        let mut i = 0;
        while i < params.len() {
            match params[i] {
                0 => self.attrs = CellAttrs::default(),
                1 => self.attrs.flags |= CellAttrs::BOLD,
                2 => self.attrs.flags |= CellAttrs::DIM,
                3 => self.attrs.flags |= CellAttrs::ITALIC,
                4 => self.attrs.flags |= CellAttrs::UNDERLINE,
                7 => self.attrs.flags |= CellAttrs::INVERSE,
                22 => self.attrs.flags &= !(CellAttrs::BOLD | CellAttrs::DIM),
                23 => self.attrs.flags &= !CellAttrs::ITALIC,
                24 => self.attrs.flags &= !CellAttrs::UNDERLINE,
                27 => self.attrs.flags &= !CellAttrs::INVERSE,
                // Standard foreground colors
                30..=37 => self.attrs.fg = ansi_color((params[i] - 30) as u8),
                39 => self.attrs.fg = CellAttrs::DEFAULT_FG,
                // Standard background colors
                40..=47 => self.attrs.bg = ansi_color((params[i] - 40) as u8),
                49 => self.attrs.bg = CellAttrs::DEFAULT_BG,
                // Bright foreground
                90..=97 => self.attrs.fg = ansi_color((params[i] - 90 + 8) as u8),
                // Bright background
                100..=107 => self.attrs.bg = ansi_color((params[i] - 100 + 8) as u8),
                // Extended foreground: 38;5;N or 38;2;R;G;B
                38 => {
                    if i + 1 < params.len() {
                        match params[i + 1] {
                            5 if i + 2 < params.len() => {
                                self.attrs.fg = ansi_color(params[i + 2] as u8);
                                i += 2;
                            }
                            2 if i + 4 < params.len() => {
                                self.attrs.fg = [
                                    params[i + 2] as u8,
                                    params[i + 3] as u8,
                                    params[i + 4] as u8,
                                ];
                                i += 4;
                            }
                            _ => {}
                        }
                    }
                }
                // Extended background: 48;5;N or 48;2;R;G;B
                48 => {
                    if i + 1 < params.len() {
                        match params[i + 1] {
                            5 if i + 2 < params.len() => {
                                self.attrs.bg = ansi_color(params[i + 2] as u8);
                                i += 2;
                            }
                            2 if i + 4 < params.len() => {
                                self.attrs.bg = [
                                    params[i + 2] as u8,
                                    params[i + 3] as u8,
                                    params[i + 4] as u8,
                                ];
                                i += 4;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn enter_alt_screen(&mut self) {
        if self.alt_grid.is_some() {
            return;
        }
        let cols = self.grid.cols;
        let rows = self.grid.rows;
        let main_grid = std::mem::replace(&mut self.grid, Grid::new(cols, rows));
        self.alt_grid = Some(Box::new(main_grid));
        self.alt_cursor = self.cursor;
        self.cursor = (0, 0);
    }

    fn exit_alt_screen(&mut self) {
        if let Some(main_grid) = self.alt_grid.take() {
            self.grid = *main_grid;
            self.cursor = self.alt_cursor;
        }
    }
}

/// Flatten vte Params into a Vec<u16>.
fn collect_params(params: &vte::Params) -> Vec<u16> {
    params.iter().flat_map(|sub| sub.iter().copied()).collect()
}

impl vte::Perform for TerminalState {
    fn print(&mut self, c: char) {
        let (_row, col) = self.cursor;
        if col >= self.grid.cols {
            // Wrap
            self.cursor.1 = 0;
            self.linefeed();
        }
        let (row, col) = self.cursor;
        self.grid.put(row, col, c, self.attrs);
        self.cursor.1 = col + 1;
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0b | 0x0c => self.linefeed(),
            b'\r' => self.cursor.1 = 0,
            b'\t' => {
                let next = (self.cursor.1 + 8) & !7;
                self.cursor.1 = next.min(self.grid.cols.saturating_sub(1));
            }
            0x08 => {
                // Backspace
                self.cursor.1 = self.cursor.1.saturating_sub(1);
            }
            0x07 => {} // Bell — ignore
            _ => {}
        }
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
    }

    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        // Ignore OSC (window title, etc.)
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let ps = collect_params(params);
        let p0 = ps.first().copied().unwrap_or(0);
        let p1 = ps.get(1).copied().unwrap_or(0);

        match action {
            // SGR — Select Graphic Rendition
            'm' => self.sgr(&ps),

            // CUP / HVP — cursor position (1-based in the protocol)
            'H' | 'f' => {
                let row = (p0.max(1) - 1) as usize;
                let col = (p1.max(1) - 1) as usize;
                self.cursor = (
                    row.min(self.grid.rows.saturating_sub(1)),
                    col.min(self.grid.cols.saturating_sub(1)),
                );
            }

            // CUU — cursor up
            'A' => {
                let n = p0.max(1) as usize;
                self.cursor.0 = self.cursor.0.saturating_sub(n);
            }
            // CUD — cursor down
            'B' => {
                let n = p0.max(1) as usize;
                self.cursor.0 = (self.cursor.0 + n).min(self.grid.rows.saturating_sub(1));
            }
            // CUF — cursor forward (right)
            'C' => {
                let n = p0.max(1) as usize;
                self.cursor.1 = (self.cursor.1 + n).min(self.grid.cols.saturating_sub(1));
            }
            // CUB — cursor backward (left)
            'D' => {
                let n = p0.max(1) as usize;
                self.cursor.1 = self.cursor.1.saturating_sub(n);
            }
            // CHA — cursor horizontal absolute
            'G' => {
                let col = (p0.max(1) - 1) as usize;
                self.cursor.1 = col.min(self.grid.cols.saturating_sub(1));
            }
            // VPA — vertical line position absolute
            'd' => {
                let row = (p0.max(1) - 1) as usize;
                self.cursor.0 = row.min(self.grid.rows.saturating_sub(1));
            }

            // ED — erase in display
            'J' => {
                let (r, c) = self.cursor;
                self.grid.erase_display(p0, r, c);
            }
            // EL — erase in line
            'K' => {
                let (r, c) = self.cursor;
                self.grid.erase_line(r, p0, c);
            }

            // IL — insert lines
            'L' => {
                let n = p0.max(1) as usize;
                let row = self.cursor.0;
                if row >= self.scroll_top && row <= self.scroll_bottom {
                    self.grid.scroll_region_down(row, self.scroll_bottom, n);
                }
            }
            // DL — delete lines
            'M' => {
                let n = p0.max(1) as usize;
                let row = self.cursor.0;
                if row >= self.scroll_top && row <= self.scroll_bottom {
                    self.grid.scroll_region_up(row, self.scroll_bottom, n);
                }
            }

            // ICH — insert characters
            '@' => {
                let n = p0.max(1) as usize;
                let (r, c) = self.cursor;
                self.grid.insert_chars(r, c, n);
            }
            // DCH — delete characters
            'P' => {
                let n = p0.max(1) as usize;
                let (r, c) = self.cursor;
                self.grid.delete_chars(r, c, n);
            }
            // ECH — erase characters (replace with blanks without shifting)
            'X' => {
                let n = p0.max(1) as usize;
                let (r, c) = self.cursor;
                for col in c..(c + n).min(self.grid.cols) {
                    self.grid.put(r, col, ' ', CellAttrs::default());
                }
            }

            // DECSTBM — set scrolling region
            'r' => {
                let top = (p0.max(1) - 1) as usize;
                let bot = if p1 == 0 {
                    self.grid.rows - 1
                } else {
                    (p1 - 1) as usize
                };
                self.scroll_top = top.min(self.grid.rows.saturating_sub(1));
                self.scroll_bottom = bot.min(self.grid.rows.saturating_sub(1));
                self.cursor = (0, 0);
            }

            // SU — scroll up
            'S' => {
                let n = p0.max(1) as usize;
                if self.scroll_top == 0 && self.scroll_bottom == self.grid.rows - 1 {
                    self.grid.scroll_up(n);
                } else {
                    self.grid
                        .scroll_region_up(self.scroll_top, self.scroll_bottom, n);
                }
            }
            // SD — scroll down
            'T' => {
                let n = p0.max(1) as usize;
                self.grid
                    .scroll_region_down(self.scroll_top, self.scroll_bottom, n);
            }

            // DEC Private modes
            'h' | 'l' => {
                if intermediates.first() == Some(&b'?') {
                    let enable = action == 'h';
                    match p0 {
                        25 => self.cursor_visible = enable,
                        1049 => {
                            if enable {
                                self.enter_alt_screen();
                            } else {
                                self.exit_alt_screen();
                            }
                        }
                        _ => {}
                    }
                }
            }

            // DECSC / DECRC via CSI (rare, usually ESC 7/8)
            's' => self.saved_cursor = self.cursor,
            'u' => {
                self.cursor = self.saved_cursor;
                self.clamp_cursor();
            }

            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match (intermediates, byte) {
            // DECSC — save cursor
            ([], b'7') => self.saved_cursor = self.cursor,
            // DECRC — restore cursor
            ([], b'8') => {
                self.cursor = self.saved_cursor;
                self.clamp_cursor();
            }
            // RI — reverse index (scroll down at top margin)
            ([], b'M') => {
                if self.cursor.0 == self.scroll_top {
                    self.grid
                        .scroll_region_down(self.scroll_top, self.scroll_bottom, 1);
                } else {
                    self.cursor.0 = self.cursor.0.saturating_sub(1);
                }
            }
            _ => {}
        }
    }
}

// -- TerminalCore (top-level API) -----------------------------------------------

pub struct TerminalCore {
    pub state: TerminalState,
    parser: vte::Parser,
    pty: Pty,
    pub scroll_offset: usize,
}

impl TerminalCore {
    pub fn new(cols: usize, rows: usize, cwd: &Path) -> io::Result<Self> {
        let pty = Pty::spawn(cols as u16, rows as u16, cwd)?;
        Ok(Self {
            state: TerminalState::new(cols, rows),
            parser: vte::Parser::new(),
            pty,
            scroll_offset: 0,
        })
    }

    /// Read any pending PTY output and feed through the parser.
    /// Returns true if the terminal state changed.
    pub fn poll(&mut self) -> bool {
        let mut buf = [0u8; 4096];
        let mut changed = false;
        loop {
            match self.pty.try_read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    for &byte in &buf[..n] {
                        self.parser.advance(&mut self.state, byte);
                    }
                    changed = true;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        if changed {
            // Auto-scroll to bottom on new output
            self.scroll_offset = 0;
        }
        changed
    }

    pub fn send_key(&mut self, key: TerminalKey) {
        let data = key.encode();
        let _ = self.pty.write_all(&data);
    }

    pub fn write_bytes(&mut self, data: &[u8]) {
        let _ = self.pty.write_all(data);
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.state.grid.resize(cols, rows);
        self.state.scroll_top = 0;
        self.state.scroll_bottom = rows.saturating_sub(1);
        self.state.clamp_cursor();
        let _ = self.pty.resize(cols as u16, rows as u16);
    }

    pub fn scroll_view_up(&mut self, n: usize) {
        let max = self.state.grid.scrollback_len();
        self.scroll_offset = (self.scroll_offset + n).min(max);
    }

    pub fn scroll_view_down(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    pub fn is_alive(&mut self) -> bool {
        self.pty.is_alive()
    }

    pub fn total_rows(&self) -> usize {
        self.state.grid.scrollback_len() + self.state.grid.rows
    }
}
