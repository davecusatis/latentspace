use ratatui::style::Color;

use super::renderer::PixelCanvas;
use crate::game::ship::Ship;
use crate::game::projectile::Projectile;
use crate::game::ship::SENSOR_RANGE;

/// Map game coordinates to pixel coordinates on the canvas.
pub struct Viewport {
    pub game_width: f64,
    pub game_height: f64,
    pub pixel_width: usize,
    pub pixel_height: usize,
}

impl Viewport {
    pub fn new(game_width: f64, game_height: f64, pixel_width: usize, pixel_height: usize) -> Self {
        Self { game_width, game_height, pixel_width, pixel_height }
    }

    pub fn to_pixel_x(&self, game_x: f64) -> f64 {
        (game_x / self.game_width) * self.pixel_width as f64
    }

    pub fn to_pixel_y(&self, game_y: f64) -> f64 {
        (game_y / self.game_height) * self.pixel_height as f64
    }

    pub fn to_pixel_radius(&self, game_radius: f64) -> f64 {
        (game_radius / self.game_width) * self.pixel_width as f64
    }
}

const SHIP_COLORS: [Color; 2] = [Color::Cyan, Color::Magenta];
const PROJECTILE_COLOR: Color = Color::Yellow;
const SENSOR_COLOR: Color = Color::Rgb(30, 30, 60);

/// Draw a ship as a small triangle-like shape pointing in its heading direction.
pub fn draw_ship(canvas: &mut PixelCanvas, ship: &Ship, ship_idx: usize, vp: &Viewport) {
    let cx = vp.to_pixel_x(ship.position.x);
    let cy = vp.to_pixel_y(ship.position.y);
    let color = SHIP_COLORS[ship_idx];

    canvas.draw_circle(cx, cy, 6.0, color);

    let rad = ship.heading.to_radians();
    let nose_x = cx + rad.cos() * 10.0;
    let nose_y = cy + rad.sin() * 10.0;
    canvas.draw_line(cx as i32, cy as i32, nose_x as i32, nose_y as i32, color);
}

/// Draw sensor range ring around a ship.
pub fn draw_sensor_range(canvas: &mut PixelCanvas, ship: &Ship, vp: &Viewport) {
    let cx = vp.to_pixel_x(ship.position.x);
    let cy = vp.to_pixel_y(ship.position.y);
    let radius = vp.to_pixel_radius(SENSOR_RANGE);
    canvas.draw_ring(cx, cy, radius, 1.0, SENSOR_COLOR);
}

/// Draw a projectile as a bright dot.
pub fn draw_projectile(canvas: &mut PixelCanvas, proj: &Projectile, vp: &Viewport) {
    let px = vp.to_pixel_x(proj.position.x);
    let py = vp.to_pixel_y(proj.position.y);
    canvas.draw_circle(px, py, 2.0, PROJECTILE_COLOR);
}

/// Draw the arena boundary as a dim border.
pub fn draw_arena_border(canvas: &mut PixelCanvas) {
    let w = canvas.pixel_width();
    let h = canvas.pixel_height();
    let color = Color::DarkGray;
    canvas.draw_line(0, 0, w as i32 - 1, 0, color);
    canvas.draw_line(0, 0, 0, h as i32 - 1, color);
    canvas.draw_line(w as i32 - 1, 0, w as i32 - 1, h as i32 - 1, color);
    canvas.draw_line(0, h as i32 - 1, w as i32 - 1, h as i32 - 1, color);
}

/// Draw shield glow around a ship when active.
pub fn draw_shield(canvas: &mut PixelCanvas, ship: &Ship, ship_idx: usize, vp: &Viewport) {
    if ship.shield_active {
        let cx = vp.to_pixel_x(ship.position.x);
        let cy = vp.to_pixel_y(ship.position.y);
        let color = Color::Rgb(80, 80, 255);
        canvas.draw_ring(cx, cy, 10.0, 2.0, color);
    }
    let _ = ship_idx;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Vec2;

    #[test]
    fn viewport_maps_coordinates() {
        let vp = Viewport::new(800.0, 400.0, 160, 80);
        assert!((vp.to_pixel_x(400.0) - 80.0).abs() < 1e-10);
        assert!((vp.to_pixel_y(200.0) - 40.0).abs() < 1e-10);
    }

    #[test]
    fn draw_ship_does_not_panic() {
        let mut canvas = PixelCanvas::new(80, 40);
        let ship = Ship::new(Vec2::new(400.0, 200.0), 45.0);
        let vp = Viewport::new(800.0, 400.0, 80, 40);
        draw_ship(&mut canvas, &ship, 0, &vp);
        assert_eq!(canvas.get_pixel(40, 20), Color::Cyan);
    }
}
