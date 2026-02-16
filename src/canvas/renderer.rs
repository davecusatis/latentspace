use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

/// A pixel buffer that renders using Unicode braille characters (U+2800–U+28FF).
/// Each terminal cell represents a 2×4 dot grid, giving 4× the resolution
/// of half-block rendering.
pub struct PixelCanvas {
    width: usize,  // pixel columns, must be multiple of 2
    height: usize, // pixel rows, must be multiple of 4
    dots: Vec<bool>,
    cell_colors: Vec<Color>, // one per terminal cell (last-write-wins)
}

impl PixelCanvas {
    /// Create a canvas. Width is rounded up to even, height to multiple of 4.
    pub fn new(width: usize, pixel_height: usize) -> Self {
        let w = if width.is_multiple_of(2) { width } else { width + 1 };
        let h = pixel_height.div_ceil(4) * 4;
        let cell_w = w / 2;
        let cell_h = h / 4;
        Self {
            width: w,
            height: h,
            dots: vec![false; w * h],
            cell_colors: vec![Color::Black; cell_w * cell_h],
        }
    }

    pub fn pixel_width(&self) -> usize {
        self.width
    }

    pub fn pixel_height(&self) -> usize {
        self.height
    }

    /// Terminal rows needed = pixel_height / 4
    pub fn cell_height(&self) -> usize {
        self.height / 4
    }

    pub fn clear(&mut self) {
        self.dots.fill(false);
        self.cell_colors.fill(Color::Black);
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x < self.width && y < self.height {
            self.dots[y * self.width + x] = true;
            let cell_w = self.width / 2;
            self.cell_colors[(y / 4) * cell_w + (x / 2)] = color;
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Color {
        if x < self.width && y < self.height && self.dots[y * self.width + x] {
            let cell_w = self.width / 2;
            self.cell_colors[(y / 4) * cell_w + (x / 2)]
        } else {
            Color::Black
        }
    }

    /// Draw a filled circle.
    pub fn draw_circle(&mut self, cx: f64, cy: f64, radius: f64, color: Color) {
        let r2 = radius * radius;
        let min_x = ((cx - radius).floor() as isize).max(0) as usize;
        let max_x = ((cx + radius).ceil() as isize).min(self.width as isize - 1) as usize;
        let min_y = ((cy - radius).floor() as isize).max(0) as usize;
        let max_y = ((cy + radius).ceil() as isize).min(self.height as isize - 1) as usize;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = px as f64 - cx;
                let dy = py as f64 - cy;
                if dx * dx + dy * dy <= r2 {
                    self.set_pixel(px, py, color);
                }
            }
        }
    }

    /// Draw a line using Bresenham's algorithm.
    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut x = x0;
        let mut y = y0;

        loop {
            if x >= 0 && y >= 0 {
                self.set_pixel(x as usize, y as usize, color);
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Draw a ring (unfilled circle outline).
    pub fn draw_ring(&mut self, cx: f64, cy: f64, radius: f64, thickness: f64, color: Color) {
        let outer_r2 = radius * radius;
        let inner_r2 = (radius - thickness).max(0.0).powi(2);
        let min_x = ((cx - radius).floor() as isize).max(0) as usize;
        let max_x = ((cx + radius).ceil() as isize).min(self.width as isize - 1) as usize;
        let min_y = ((cy - radius).floor() as isize).max(0) as usize;
        let max_y = ((cy + radius).ceil() as isize).min(self.height as isize - 1) as usize;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = px as f64 - cx;
                let dy = py as f64 - cy;
                let dist2 = dx * dx + dy * dy;
                if dist2 <= outer_r2 && dist2 >= inner_r2 {
                    self.set_pixel(px, py, color);
                }
            }
        }
    }
}

/// Widget implementation — renders the pixel buffer as braille characters.
impl Widget for &PixelCanvas {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cell_rows = self.cell_height().min(area.height as usize);
        let cell_cols = (self.width / 2).min(area.width as usize);

        for row in 0..cell_rows {
            for col in 0..cell_cols {
                let px = col * 2;
                let py = row * 4;

                // Encode 2×4 dots into braille pattern.
                // Braille dot positions:
                //   Col 0: bits 0,1,2,6  (rows 0–3)
                //   Col 1: bits 3,4,5,7  (rows 0–3)
                let mut pattern: u8 = 0;
                if self.dots[py * self.width + px]           { pattern |= 0x01; }
                if self.dots[(py + 1) * self.width + px]     { pattern |= 0x02; }
                if self.dots[(py + 2) * self.width + px]     { pattern |= 0x04; }
                if self.dots[(py + 3) * self.width + px]     { pattern |= 0x40; }
                if self.dots[py * self.width + px + 1]       { pattern |= 0x08; }
                if self.dots[(py + 1) * self.width + px + 1] { pattern |= 0x10; }
                if self.dots[(py + 2) * self.width + px + 1] { pattern |= 0x20; }
                if self.dots[(py + 3) * self.width + px + 1] { pattern |= 0x80; }

                if pattern != 0 {
                    let ch = char::from_u32(0x2800 + pattern as u32).unwrap();
                    let cell_w = self.width / 2;
                    let color = self.cell_colors[row * cell_w + col];
                    let cell = &mut buf[(area.x + col as u16, area.y + row as u16)];
                    cell.set_char(ch);
                    cell.set_fg(color);
                    cell.set_bg(Color::Black);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_dimensions() {
        let c = PixelCanvas::new(80, 48);
        assert_eq!(c.pixel_width(), 80);
        assert_eq!(c.pixel_height(), 48);
        assert_eq!(c.cell_height(), 12);
    }

    #[test]
    fn canvas_height_rounds_to_multiple_of_4() {
        let c = PixelCanvas::new(80, 47);
        assert_eq!(c.pixel_height(), 48);

        let c = PixelCanvas::new(80, 45);
        assert_eq!(c.pixel_height(), 48);
    }

    #[test]
    fn canvas_odd_width_rounds_up() {
        let c = PixelCanvas::new(79, 48);
        assert_eq!(c.pixel_width(), 80);
    }

    #[test]
    fn set_and_get_pixel() {
        let mut c = PixelCanvas::new(10, 12);
        c.set_pixel(5, 5, Color::Red);
        assert_eq!(c.get_pixel(5, 5), Color::Red);
        assert_eq!(c.get_pixel(0, 0), Color::Black);
    }

    #[test]
    fn out_of_bounds_ignored() {
        let mut c = PixelCanvas::new(10, 12);
        c.set_pixel(100, 100, Color::Red);
        assert_eq!(c.get_pixel(100, 100), Color::Black);
    }

    #[test]
    fn clear_resets_all() {
        let mut c = PixelCanvas::new(10, 12);
        c.set_pixel(5, 5, Color::Red);
        c.clear();
        assert_eq!(c.get_pixel(5, 5), Color::Black);
    }

    #[test]
    fn circle_draws_pixels() {
        let mut c = PixelCanvas::new(20, 20);
        c.draw_circle(10.0, 10.0, 3.0, Color::Cyan);
        assert_eq!(c.get_pixel(10, 10), Color::Cyan);
        assert_eq!(c.get_pixel(0, 0), Color::Black);
    }
}
