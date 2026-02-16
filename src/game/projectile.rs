use super::Vec2;
use super::ship;

#[derive(Debug, Clone)]
pub struct Projectile {
    pub position: Vec2,
    pub velocity: Vec2,
    pub damage: i32,
    pub owner: usize,
}

impl Projectile {
    pub fn spawn_primary(ship: &ship::Ship, owner: usize) -> Self {
        let rad = ship.heading.to_radians();
        let direction = Vec2::new(rad.cos(), rad.sin());
        Self {
            position: ship.position + direction * 5.0,
            velocity: direction * ship::PRIMARY_PROJECTILE_SPEED + ship.velocity,
            damage: ship::PRIMARY_DAMAGE,
            owner,
        }
    }

    pub fn spawn_secondary(ship: &ship::Ship, owner: usize) -> Self {
        let rad = ship.heading.to_radians();
        let direction = Vec2::new(rad.cos(), rad.sin());
        Self {
            position: ship.position + direction * 5.0,
            velocity: direction * ship::SECONDARY_PROJECTILE_SPEED + ship.velocity,
            damage: ship::SECONDARY_DAMAGE,
            owner,
        }
    }

    pub fn update(&mut self) {
        self.position = self.position + self.velocity;
    }

    /// Update position with a fractional time step.
    /// `dt` should be 1.0/substeps so the total displacement per full turn
    /// is the same as a single `update()` call.
    pub fn update_substep(&mut self, dt: f64) {
        self.position = self.position + self.velocity * dt;
    }

    pub fn is_in_bounds(&self, arena_width: f64, arena_height: f64) -> bool {
        self.position.x >= 0.0
            && self.position.x <= arena_width
            && self.position.y >= 0.0
            && self.position.y <= arena_height
    }

    pub fn hits_ship(&self, ship: &ship::Ship, hit_radius: f64) -> bool {
        self.position.distance_to(ship.position) <= hit_radius
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_spawns_ahead_of_ship() {
        let ship = ship::Ship::new(Vec2::new(100.0, 100.0), 0.0);
        let proj = Projectile::spawn_primary(&ship, 0);
        assert!(proj.position.x > ship.position.x);
    }

    #[test]
    fn projectile_moves_on_update() {
        let ship = ship::Ship::new(Vec2::new(100.0, 100.0), 0.0);
        let mut proj = Projectile::spawn_primary(&ship, 0);
        let old_x = proj.position.x;
        proj.update();
        assert!(proj.position.x > old_x);
    }

    #[test]
    fn projectile_out_of_bounds() {
        let ship = ship::Ship::new(Vec2::new(-10.0, 100.0), 180.0);
        let mut proj = Projectile::spawn_primary(&ship, 0);
        proj.position = Vec2::new(-5.0, 100.0);
        assert!(!proj.is_in_bounds(800.0, 400.0));
    }

    #[test]
    fn substep_update_scales_movement() {
        let ship = ship::Ship::new(Vec2::new(100.0, 100.0), 0.0);
        let mut proj = Projectile::spawn_primary(&ship, 0);
        let start_x = proj.position.x;
        proj.update_substep(0.25);
        let quarter_move = proj.position.x - start_x;
        // A quarter-step should move roughly 1/4 of the full velocity
        let expected = proj.velocity.x * 0.25;
        assert!(
            (quarter_move - expected).abs() < 1e-10,
            "quarter_move={} expected={}",
            quarter_move,
            expected
        );
    }
}
