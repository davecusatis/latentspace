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
}
