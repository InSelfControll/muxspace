use super::screen::{Cell, CellStyle, ScreenBuffer};
use vte::Params;

impl ScreenBuffer {
    pub(crate) fn erase_display(&mut self, mode: usize) {
        match mode {
            0 => {
                for c in self.cursor_col..self.cols {
                    self.grid[self.cursor_row][c] = Cell::default();
                }
                for r in (self.cursor_row + 1)..self.rows {
                    self.grid[r] = vec![Cell::default(); self.cols];
                }
            }
            1 => {
                for r in 0..self.cursor_row {
                    self.grid[r] = vec![Cell::default(); self.cols];
                }
                for c in 0..=self.cursor_col.min(self.cols.saturating_sub(1)) {
                    self.grid[self.cursor_row][c] = Cell::default();
                }
            }
            2 | 3 => self.clear_screen(),
            _ => {}
        }
    }

    pub(crate) fn erase_line(&mut self, mode: usize) {
        match mode {
            0 => {
                for c in self.cursor_col..self.cols {
                    self.grid[self.cursor_row][c] = Cell::default();
                }
            }
            1 => {
                for c in 0..=self.cursor_col.min(self.cols.saturating_sub(1)) {
                    self.grid[self.cursor_row][c] = Cell::default();
                }
            }
            2 => self.grid[self.cursor_row] = vec![Cell::default(); self.cols],
            _ => {}
        }
    }

    pub(crate) fn insert_lines(&mut self, n: usize) {
        for _ in 0..n {
            if self.cursor_row < self.rows {
                self.grid
                    .insert(self.cursor_row, vec![Cell::default(); self.cols]);
                if self.grid.len() > self.rows {
                    self.grid.pop();
                }
            }
        }
    }

    pub(crate) fn delete_lines(&mut self, n: usize) {
        for _ in 0..n {
            if self.cursor_row < self.grid.len() {
                self.grid.remove(self.cursor_row);
                self.grid.push(vec![Cell::default(); self.cols]);
            }
        }
    }

    pub(crate) fn delete_chars(&mut self, n: usize) {
        let row = &mut self.grid[self.cursor_row];
        for _ in 0..n {
            if self.cursor_col < row.len() {
                row.remove(self.cursor_col);
                row.push(Cell::default());
            }
        }
    }

    pub(crate) fn scroll_down(&mut self, n: usize) {
        for _ in 0..n {
            if !self.grid.is_empty() {
                self.grid.pop();
                self.grid.insert(0, vec![Cell::default(); self.cols]);
            }
        }
    }

    pub(crate) fn erase_chars(&mut self, n: usize) {
        for i in 0..n {
            let col = self.cursor_col + i;
            if col < self.cols {
                self.grid[self.cursor_row][col] = Cell::default();
            }
        }
    }

    pub(crate) fn insert_chars(&mut self, n: usize) {
        let row = &mut self.grid[self.cursor_row];
        for _ in 0..n {
            row.insert(self.cursor_col, Cell::default());
            if row.len() > self.cols {
                row.pop();
            }
        }
    }

    pub(crate) fn apply_sgr(&mut self, params: &Params) {
        for value in params.iter().flat_map(|slice| slice.iter().copied()) {
            match value {
                0 => self.style = CellStyle::default(),
                1 => self.style.bold = true,
                22 => self.style.bold = false,
                30..=37 => self.style.fg = (value - 30) as u8,
                39 => self.style.fg = 0,
                40..=47 => self.style.bg = (value - 40) as u8,
                49 => self.style.bg = 0,
                90..=97 => self.style.fg = (value - 90 + 8) as u8,
                100..=107 => self.style.bg = (value - 100 + 8) as u8,
                _ => {}
            }
        }
    }

    fn clear_screen(&mut self) {
        for r in 0..self.rows {
            self.grid[r] = vec![Cell::default(); self.cols];
        }
    }
}

pub(crate) fn param(params: &Params, idx: usize, default: usize) -> usize {
    params
        .iter()
        .nth(idx)
        .and_then(|s| s.first().copied())
        .unwrap_or(default as u16) as usize
}

pub(crate) fn move_limit(current: usize, delta: usize, limit: usize) -> usize {
    (current + delta).min(limit.saturating_sub(1))
}

pub(crate) fn absolute(value: usize, limit: usize) -> usize {
    value.saturating_sub(1).min(limit.saturating_sub(1))
}
