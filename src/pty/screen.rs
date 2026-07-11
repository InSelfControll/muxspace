use vte::{Params, Parser, Perform};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cell {
    pub ch: char,
    pub fg: u8,
    #[allow(dead_code)]
    pub bg: u8,
    pub bold: bool,
}

#[derive(Clone, Default, Debug)]
pub(crate) struct CellStyle {
    pub(crate) fg: u8,
    pub(crate) bg: u8,
    pub(crate) bold: bool,
}

pub struct ScreenBuffer {
    pub rows: usize,
    pub cols: usize,
    pub grid: Vec<Vec<Cell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scrollback: Vec<Vec<Cell>>,
    pub title: String,
    pub(crate) max_scrollback: usize,
    pub(crate) style: CellStyle,
    parser: Parser,
}

impl Clone for ScreenBuffer {
    fn clone(&self) -> Self {
        Self {
            rows: self.rows,
            cols: self.cols,
            grid: self.grid.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
            scrollback: self.scrollback.clone(),
            title: self.title.clone(),
            max_scrollback: self.max_scrollback,
            style: self.style.clone(),
            parser: Parser::new(),
        }
    }
}

impl PartialEq for ScreenBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.rows == other.rows
            && self.cols == other.cols
            && self.grid == other.grid
            && self.cursor_row == other.cursor_row
            && self.cursor_col == other.cursor_col
            && self.title == other.title
    }
}

impl std::fmt::Debug for ScreenBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScreenBuffer")
            .field("rows", &self.rows)
            .field("cols", &self.cols)
            .finish()
    }
}

impl ScreenBuffer {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            grid: vec![vec![Cell::default(); cols]; rows],
            cursor_row: 0,
            cursor_col: 0,
            scrollback: Vec::new(),
            title: String::new(),
            max_scrollback: 10_000,
            style: CellStyle::default(),
            parser: Parser::new(),
        }
    }

    pub fn process(&mut self, bytes: &[u8]) {
        let mut parser = std::mem::replace(&mut self.parser, Parser::new());
        for &byte in bytes {
            parser.advance(self, byte);
        }
        self.parser = parser;
    }

    pub(crate) fn scroll_up(&mut self) {
        let old = self.grid.remove(0);
        self.scrollback.push(old);
        if self.scrollback.len() > self.max_scrollback {
            self.scrollback.remove(0);
        }
        self.grid.push(vec![Cell::default(); self.cols]);
    }

    pub(crate) fn put_char(&mut self, ch: char) {
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.cursor_row += 1;
        }
        if self.cursor_row >= self.rows {
            self.scroll_up();
            self.cursor_row = self.rows.saturating_sub(1);
        }
        self.grid[self.cursor_row][self.cursor_col] = Cell {
            ch,
            fg: self.style.fg,
            bg: self.style.bg,
            bold: self.style.bold,
        };
        self.cursor_col += 1;
    }
}

impl Perform for ScreenBuffer {
    fn print(&mut self, c: char) {
        self.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0B | 0x0C => {
                self.cursor_row += 1;
                self.cursor_col = 0;
                if self.cursor_row >= self.rows {
                    self.scroll_up();
                    self.cursor_row = self.rows.saturating_sub(1);
                }
            }
            b'\r' => self.cursor_col = 0,
            b'\t' => {
                self.cursor_col = ((self.cursor_col / 8) + 1) * 8;
                if self.cursor_col >= self.cols {
                    self.cursor_col = self.cols.saturating_sub(1);
                }
            }
            0x08 => self.cursor_col = self.cursor_col.saturating_sub(1),
            0x07 => {}
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        match action {
            'A' => {
                self.cursor_row = self
                    .cursor_row
                    .saturating_sub(super::screen_ops::param(params, 0, 1))
            }
            'B' => {
                self.cursor_row = super::screen_ops::move_limit(
                    self.cursor_row,
                    super::screen_ops::param(params, 0, 1),
                    self.rows,
                )
            }
            'C' => {
                self.cursor_col = super::screen_ops::move_limit(
                    self.cursor_col,
                    super::screen_ops::param(params, 0, 1),
                    self.cols,
                )
            }
            'D' => {
                self.cursor_col = self
                    .cursor_col
                    .saturating_sub(super::screen_ops::param(params, 0, 1))
            }
            'E' => {
                self.cursor_row = super::screen_ops::move_limit(
                    self.cursor_row,
                    super::screen_ops::param(params, 0, 1),
                    self.rows,
                );
                self.cursor_col = 0;
            }
            'F' => {
                self.cursor_row = self
                    .cursor_row
                    .saturating_sub(super::screen_ops::param(params, 0, 1));
                self.cursor_col = 0;
            }
            'G' => {
                self.cursor_col =
                    super::screen_ops::absolute(super::screen_ops::param(params, 0, 1), self.cols)
            }
            'H' | 'f' => {
                self.cursor_row =
                    super::screen_ops::absolute(super::screen_ops::param(params, 0, 1), self.rows);
                self.cursor_col =
                    super::screen_ops::absolute(super::screen_ops::param(params, 1, 1), self.cols);
            }
            'J' => self.erase_display(super::screen_ops::param(params, 0, 0)),
            'K' => self.erase_line(super::screen_ops::param(params, 0, 0)),
            'L' => self.insert_lines(super::screen_ops::param(params, 0, 1)),
            'M' => self.delete_lines(super::screen_ops::param(params, 0, 1)),
            'P' => self.delete_chars(super::screen_ops::param(params, 0, 1)),
            'S' => {
                for _ in 0..super::screen_ops::param(params, 0, 1) {
                    self.scroll_up();
                }
            }
            'T' => self.scroll_down(super::screen_ops::param(params, 0, 1)),
            'X' => self.erase_chars(super::screen_ops::param(params, 0, 1)),
            '@' => self.insert_chars(super::screen_ops::param(params, 0, 1)),
            'd' => {
                self.cursor_row =
                    super::screen_ops::absolute(super::screen_ops::param(params, 0, 1), self.rows)
            }
            'm' => self.apply_sgr(params),
            'h' | 'l' | 'r' | 'n' | 'c' => {}
            _ => {}
        }
    }

    fn hook(&mut self, _: &Params, _: &[u8], _: bool, _: char) {}
    fn put(&mut self, _: u8) {}
    fn unhook(&mut self) {}
    fn esc_dispatch(&mut self, _: &[u8], _: bool, _: u8) {}

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.len() >= 2 && (params[0] == b"0" || params[0] == b"2") {
            if let Ok(title) = std::str::from_utf8(params[1]) {
                self.title = title.to_string();
            }
        }
    }
}
