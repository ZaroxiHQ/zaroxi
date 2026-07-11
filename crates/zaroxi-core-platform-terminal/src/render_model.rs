//! Renderer-agnostic terminal grid model.
//!
//! [`build_grid`] projects a `vt100::Screen` (the live emulator state, including
//! any active scrollback offset) into a flat cell grid the interface layer can
//! turn into draw commands. This is a pure transformation — no I/O, no
//! rendering — so it is fully unit-testable by feeding bytes through a parser
//! and asserting on the resulting grid.

use crate::palette::{Rgba, TerminalPalette};

/// A single rendered terminal cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerminalCell {
    /// The primary character in the cell (`' '` when empty).
    pub ch: char,
    /// Foreground color (already inverse-resolved).
    pub fg: Rgba,
    /// Background color (already inverse-resolved).
    pub bg: Rgba,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// True when this cell is the left half of a double-width glyph.
    pub wide: bool,
    /// True when this cell is the right-half placeholder of a wide glyph.
    pub wide_continuation: bool,
}

impl TerminalCell {
    fn blank(palette: &TerminalPalette) -> Self {
        Self {
            ch: ' ',
            fg: palette.foreground,
            bg: palette.background,
            bold: false,
            italic: false,
            underline: false,
            wide: false,
            wide_continuation: false,
        }
    }
}

/// Cursor position + visibility, in cell coordinates (0-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCursor {
    pub row: usize,
    pub col: usize,
    pub visible: bool,
}

/// A full grid snapshot ready for rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct TerminalGrid {
    pub rows: usize,
    pub cols: usize,
    /// Row-major cells (`rows * cols` entries).
    pub cells: Vec<TerminalCell>,
    pub cursor: TerminalCursor,
    /// Whether the emulator is showing its alternate screen (full-screen apps).
    pub alternate_screen: bool,
}

impl TerminalGrid {
    /// Borrow the cell at `(row, col)`, if in bounds.
    pub fn cell(&self, row: usize, col: usize) -> Option<&TerminalCell> {
        if row >= self.rows || col >= self.cols {
            return None;
        }
        self.cells.get(row * self.cols + col)
    }

    /// The plain (uncolored) text of a row, trailing blanks trimmed.
    pub fn row_text(&self, row: usize) -> String {
        if row >= self.rows {
            return String::new();
        }
        let start = row * self.cols;
        let mut s: String = self.cells[start..start + self.cols]
            .iter()
            .filter(|c| !c.wide_continuation)
            .map(|c| c.ch)
            .collect();
        let trimmed = s.trim_end().len();
        s.truncate(trimmed);
        s
    }
}

/// Project a `vt100::Screen` into a [`TerminalGrid`] using `palette`.
pub fn build_grid(screen: &vt100::Screen, palette: &TerminalPalette) -> TerminalGrid {
    let (rows_u16, cols_u16) = screen.size();
    let rows = rows_u16 as usize;
    let cols = cols_u16 as usize;
    let mut cells = vec![TerminalCell::blank(palette); rows * cols];

    for row in 0..rows_u16 {
        for col in 0..cols_u16 {
            let Some(src) = screen.cell(row, col) else { continue };
            let bold = src.bold();
            let inverse = src.inverse();

            let mut fg = palette.resolve(src.fgcolor(), true, bold);
            let mut bg = palette.resolve(src.bgcolor(), false, false);
            if inverse {
                std::mem::swap(&mut fg, &mut bg);
            }

            let ch = src.contents().chars().next().unwrap_or(' ');
            let dst = &mut cells[row as usize * cols + col as usize];
            *dst = TerminalCell {
                ch: if src.has_contents() { ch } else { ' ' },
                fg,
                bg,
                bold,
                italic: src.italic(),
                underline: src.underline(),
                wide: src.is_wide(),
                wide_continuation: src.is_wide_continuation(),
            };
        }
    }

    let (cur_row, cur_col) = screen.cursor_position();
    let cursor = TerminalCursor {
        row: (cur_row as usize).min(rows.saturating_sub(1)),
        col: (cur_col as usize).min(cols.saturating_sub(1)),
        // The cursor is only meaningful when the user is viewing the live
        // bottom of the buffer; hide it while scrolled back into history.
        visible: !screen.hide_cursor() && screen.scrollback() == 0,
    };

    TerminalGrid { rows, cols, cells, cursor, alternate_screen: screen.alternate_screen() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(bytes: &[u8]) -> vt100::Parser {
        let mut p = vt100::Parser::new(6, 20, 100);
        p.process(bytes);
        p
    }

    #[test]
    fn plain_text_lands_in_the_grid() {
        let p = feed(b"hello");
        let grid = build_grid(p.screen(), &TerminalPalette::default());
        assert_eq!(grid.rows, 6);
        assert_eq!(grid.cols, 20);
        assert_eq!(grid.row_text(0), "hello");
        // Cursor advanced past the text.
        assert_eq!(grid.cursor.row, 0);
        assert_eq!(grid.cursor.col, 5);
        assert!(grid.cursor.visible);
    }

    #[test]
    fn newlines_move_rows() {
        let p = feed(b"ab\r\ncd");
        let grid = build_grid(p.screen(), &TerminalPalette::default());
        assert_eq!(grid.row_text(0), "ab");
        assert_eq!(grid.row_text(1), "cd");
        assert_eq!(grid.cursor.row, 1);
    }

    #[test]
    fn sgr_colors_are_applied() {
        // Red foreground (SGR 31) then a char.
        let p = feed(b"\x1b[31mX");
        let palette = TerminalPalette::default();
        let grid = build_grid(p.screen(), &palette);
        let cell = grid.cell(0, 0).unwrap();
        assert_eq!(cell.ch, 'X');
        assert_eq!(cell.fg, palette.ansi[1]);
    }

    #[test]
    fn inverse_swaps_fg_and_bg() {
        let p = feed(b"\x1b[7mZ");
        let palette = TerminalPalette::default();
        let grid = build_grid(p.screen(), &palette);
        let cell = grid.cell(0, 0).unwrap();
        // Inverse video swaps the default fg/bg.
        assert_eq!(cell.fg, palette.background);
        assert_eq!(cell.bg, palette.foreground);
    }
}
