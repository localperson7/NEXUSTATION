use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};

/// Virtual pixel buffer using Unicode upper/lower half-blocks (`▀`, `▄`, `█`)
/// to achieve 2x vertical pixel resolution in any monospace terminal.
pub struct HalfBlockCanvas {
    width: usize,
    height_px: usize,
    pixels: Vec<Option<Color>>,
}

impl HalfBlockCanvas {
    pub fn new(width_cols: u16, height_rows: u16) -> Self {
        let width = usize::from(width_cols);
        let height_px = usize::from(height_rows) * 2;
        Self {
            width,
            height_px,
            pixels: vec![None; width * height_px],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height_px(&self) -> usize {
        self.height_px
    }

    pub fn set(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && y >= 0 {
            let ux = x as usize;
            let uy = y as usize;
            if ux < self.width && uy < self.height_px {
                self.pixels[uy * self.width + ux] = Some(color);
            }
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color) {
        for py in y..(y + h) {
            for px in x..(x + w) {
                self.set(px, py, color);
            }
        }
    }

    /// Draws an ASCII sprite pattern (`'#'` or non-space = `color`, `' '` = transparent).
    pub fn draw_sprite(&mut self, x: i32, y: i32, rows: &[&str], color: Color) {
        for (ry, row) in rows.iter().enumerate() {
            for (rx, ch) in row.chars().enumerate() {
                if ch != ' ' && ch != '.' {
                    self.set(x + rx as i32, y + ry as i32, color);
                }
            }
        }
    }

    /// Draws a 3x5 pixel digit (`0..=9`) at pixel coordinates `(x, y)`.
    pub fn draw_digit(&mut self, x: i32, y: i32, digit: u32, color: Color) {
        const DIGITS: [[&str; 5]; 10] = [
            ["###", "# #", "# #", "# #", "###"],
            [" # ", "## ", " # ", " # ", "###"],
            ["###", "  #", "###", "#  ", "###"],
            ["###", "  #", "###", "  #", "###"],
            ["# #", "# #", "###", "  #", "  #"],
            ["###", "#  ", "###", "  #", "###"],
            ["###", "#  ", "###", "# #", "###"],
            ["###", "  #", "  #", "  #", "  #"],
            ["###", "# #", "###", "# #", "###"],
            ["###", "# #", "###", "  #", "###"],
        ];
        let idx = (digit % 10) as usize;
        self.draw_sprite(x, y, &DIGITS[idx], color);
    }

    /// Blits the virtual pixel canvas into a Ratatui `Buffer` within `area`.
    pub fn render(&self, area: Rect, buf: &mut Buffer, base_bg: Color, alt_bg: Color, scan: bool) {
        let rows = usize::from(area.height).min(self.height_px / 2);
        let cols = usize::from(area.width).min(self.width);

        for row in 0..rows {
            let screen_y = area.y + row as u16;
            let bg = if scan && screen_y % 2 == 1 {
                alt_bg
            } else {
                base_bg
            };
            let top_y = row * 2;
            let bot_y = top_y + 1;

            for col in 0..cols {
                let screen_x = area.x + col as u16;
                let top = self.pixels[top_y * self.width + col];
                let bot = self.pixels[bot_y * self.width + col];

                let cell = &mut buf[(screen_x, screen_y)];
                match (top, bot) {
                    (None, None) => {
                        cell.set_symbol(" ").set_style(Style::default().bg(bg));
                    }
                    (Some(tc), None) => {
                        cell.set_symbol("▀")
                            .set_style(Style::default().fg(tc).bg(bg));
                    }
                    (None, Some(bc)) => {
                        cell.set_symbol("▄")
                            .set_style(Style::default().fg(bc).bg(bg));
                    }
                    (Some(tc), Some(bc)) if tc == bc => {
                        cell.set_symbol("█")
                            .set_style(Style::default().fg(tc).bg(tc));
                    }
                    (Some(tc), Some(bc)) => {
                        cell.set_symbol("▀")
                            .set_style(Style::default().fg(tc).bg(bc));
                    }
                }
            }
        }
    }
}
