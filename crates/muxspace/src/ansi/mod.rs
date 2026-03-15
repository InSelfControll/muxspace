use std::collections::VecDeque;
use vte::{Params, Parser, Perform};

const MAX_SCROLLBACK: usize = 10_000;

#[derive(Clone, Debug, Default)]
pub struct Style {
    pub fg: Option<u8>,
    pub bg: Option<u8>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub reversed: bool,
}

#[derive(Clone, Debug)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Cell { ch: ' ', style: Style::default() }
    }
}

pub struct ScreenBuffer {
    pub rows: usize,
    pub cols: usize,
    pub grid: Vec<Vec<Cell>>,
    pub scrollback: VecDeque<Vec<Cell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    current_style: Style,
    parser: Parser,
}

impl ScreenBuffer {
    pub fn new(rows: usize, cols: usize) -> Self {
        let grid = vec![vec![Cell::default(); cols]; rows];
        ScreenBuffer {
            rows,
            cols,
            grid,
            scrollback: VecDeque::new(),
            cursor_row: 0,
            cursor_col: 0,
            current_style: Style::default(),
            parser: Parser::new(),
        }
    }

    /// Feed raw PTY bytes into the parser.
    pub fn process(&mut self, bytes: &[u8]) {
        // Take the parser out temporarily so we can pass &mut self as Perform.
        let mut parser = std::mem::replace(&mut self.parser, Parser::new());
        {
            let mut performer = ScreenPerformer(self);
            for &b in bytes {
                parser.advance(&mut performer, b);
            }
        }
        self.parser = parser;
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.rows = rows;
        self.cols = cols;
        self.grid.resize_with(rows, || vec![Cell::default(); cols]);
        for row in &mut self.grid {
            row.resize_with(cols, Cell::default);
        }
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
    }

    fn scroll_up(&mut self) {
        let old = self.grid.remove(0);
        self.scrollback.push_back(old);
        if self.scrollback.len() > MAX_SCROLLBACK {
            self.scrollback.pop_front();
        }
        self.grid.push(vec![Cell::default(); self.cols]);
    }

    fn put_char(&mut self, ch: char) {
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.cursor_row += 1;
        }
        if self.cursor_row >= self.rows {
            self.scroll_up();
            self.cursor_row = self.rows - 1;
        }
        let style = self.current_style.clone();
        self.grid[self.cursor_row][self.cursor_col] = Cell { ch, style };
        self.cursor_col += 1;
    }
}

// --- vte Perform impl --------------------------------------------------------

struct ScreenPerformer<'a>(&'a mut ScreenBuffer);

impl<'a> Perform for ScreenPerformer<'a> {
    fn print(&mut self, c: char) {
        self.0.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0B | 0x0C => {
                self.0.cursor_row += 1;
                if self.0.cursor_row >= self.0.rows {
                    self.0.scroll_up();
                    self.0.cursor_row = self.0.rows - 1;
                }
            }
            b'\r' => {
                self.0.cursor_col = 0;
            }
            0x08 => {
                if self.0.cursor_col > 0 {
                    self.0.cursor_col -= 1;
                }
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        let s = &mut self.0;
        match action {
            'A' => { let n = param(params, 0, 1); s.cursor_row = s.cursor_row.saturating_sub(n); }
            'B' => { let n = param(params, 0, 1); s.cursor_row = (s.cursor_row + n).min(s.rows - 1); }
            'C' => { let n = param(params, 0, 1); s.cursor_col = (s.cursor_col + n).min(s.cols - 1); }
            'D' => { let n = param(params, 0, 1); s.cursor_col = s.cursor_col.saturating_sub(n); }
            'H' | 'f' => {
                let row = param(params, 0, 1).saturating_sub(1);
                let col = param(params, 1, 1).saturating_sub(1);
                s.cursor_row = row.min(s.rows - 1);
                s.cursor_col = col.min(s.cols - 1);
            }
            'J' => match param(params, 0, 0) {
                0 => {
                    for col in s.cursor_col..s.cols { s.grid[s.cursor_row][col] = Cell::default(); }
                    for r in (s.cursor_row + 1)..s.rows { s.grid[r] = vec![Cell::default(); s.cols]; }
                }
                1 => {
                    for r in 0..s.cursor_row { s.grid[r] = vec![Cell::default(); s.cols]; }
                    for col in 0..=s.cursor_col { s.grid[s.cursor_row][col] = Cell::default(); }
                }
                2 => { for r in 0..s.rows { s.grid[r] = vec![Cell::default(); s.cols]; } }
                _ => {}
            },
            'K' => match param(params, 0, 0) {
                0 => { for col in s.cursor_col..s.cols { s.grid[s.cursor_row][col] = Cell::default(); } }
                1 => { for col in 0..=s.cursor_col { s.grid[s.cursor_row][col] = Cell::default(); } }
                2 => { s.grid[s.cursor_row] = vec![Cell::default(); s.cols]; }
                _ => {}
            },
            'm' => apply_sgr(&mut s.current_style, params),
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

fn param(params: &Params, idx: usize, default: usize) -> usize {
    params.iter().nth(idx).and_then(|s| s.first().copied()).unwrap_or(default as u16) as usize
}

fn apply_sgr(style: &mut Style, params: &Params) {
    let values: Vec<u16> = params.iter().flat_map(|s| s.iter().copied()).collect();
    let mut i = 0;
    while i < values.len() {
        match values[i] {
            0  => *style = Style::default(),
            1  => style.bold = true,
            3  => style.italic = true,
            4  => style.underline = true,
            7  => style.reversed = true,
            22 => style.bold = false,
            30..=37 => style.fg = Some((values[i] - 30) as u8),
            38 if values.get(i + 1) == Some(&5) => {
                style.fg = values.get(i + 2).map(|&v| v as u8);
                i += 2;
            }
            39 => style.fg = None,
            40..=47 => style.bg = Some((values[i] - 40) as u8),
            48 if values.get(i + 1) == Some(&5) => {
                style.bg = values.get(i + 2).map(|&v| v as u8);
                i += 2;
            }
            49  => style.bg = None,
            90..=97   => style.fg = Some((values[i] - 90 + 8) as u8),
            100..=107 => style.bg = Some((values[i] - 100 + 8) as u8),
            _ => {}
        }
        i += 1;
    }
}
