use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

/// A pixel buffer that renders using half-block characters.
/// Each terminal cell represents 2 vertical pixels using ▀ with
/// foreground = top pixel, background = bottom pixel.
pub struct PixelCanvas {
    width: usize,
    height: usize, // must be even — represents pixel rows
    pixels: Vec<Color>,
}

impl PixelCanvas {
    /// Create a canvas. `pixel_height` will be rounded up to even.
    pub fn new(width: usize, pixel_height: usize) -> Self {
        let height = if pixel_height % 2 == 0 {
            pixel_height
        } else {
            pixel_height + 1
        };
        Self {
            width,
            height,
            pixels: vec![Color::Black; width * height],
        }
    }

    pub fn pixel_width(&self) -> usize {
        self.width
    }

    pub fn pixel_height(&self) -> usize {
        self.height
    }

    /// Terminal rows needed = pixel_height / 2
    pub fn cell_height(&self) -> usize {
        self.height / 2
    }

    pub fn clear(&mut self) {
        self.pixels.fill(Color::Black);
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = color;
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Color {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x]
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

/// Widget implementation — renders the pixel buffer into a ratatui area.
impl Widget for &PixelCanvas {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rows = self.cell_height().min(area.height as usize);
        let cols = self.width.min(area.width as usize);

        for row in 0..rows {
            for col in 0..cols {
                let top = self.get_pixel(col, row * 2);
                let bottom = self.get_pixel(col, row * 2 + 1);
                let cell = &mut buf[(area.x + col as u16, area.y + row as u16)];
                cell.set_char('\u{2580}');
                cell.set_fg(top);
                cell.set_bg(bottom);
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
        assert_eq!(c.cell_height(), 24);
    }

    #[test]
    fn canvas_odd_height_rounds_up() {
        let c = PixelCanvas::new(80, 47);
        assert_eq!(c.pixel_height(), 48);
    }

    #[test]
    fn set_and_get_pixel() {
        let mut c = PixelCanvas::new(10, 10);
        c.set_pixel(5, 5, Color::Red);
        assert_eq!(c.get_pixel(5, 5), Color::Red);
        assert_eq!(c.get_pixel(0, 0), Color::Black);
    }

    #[test]
    fn out_of_bounds_ignored() {
        let mut c = PixelCanvas::new(10, 10);
        c.set_pixel(100, 100, Color::Red);
        assert_eq!(c.get_pixel(100, 100), Color::Black);
    }

    #[test]
    fn clear_resets_all() {
        let mut c = PixelCanvas::new(10, 10);
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
