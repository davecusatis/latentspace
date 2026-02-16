use super::Vec2;
use super::ship;

#[derive(Debug, Clone)]
pub struct Projectile {
    pub position: Vec2,
    pub velocity: Vec2,
    pub damage: i32,
    pub owner: usize,
    pub distance_traveled: f64,
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
            distance_traveled: 0.0,
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
            distance_traveled: 0.0,
        }
    }

    /// Returns the effective damage after applying the arming distance scaling.
    /// Damage scales from PROJECTILE_MIN_DAMAGE_FRACTION at d=0 to 100% at
    /// d >= PROJECTILE_ARM_DISTANCE.
    pub fn effective_damage(&self) -> i32 {
        let t = (self.distance_traveled / ship::PROJECTILE_ARM_DISTANCE).min(1.0);
        let fraction = ship::PROJECTILE_MIN_DAMAGE_FRACTION
            + (1.0 - ship::PROJECTILE_MIN_DAMAGE_FRACTION) * t;
        (self.damage as f64 * fraction).round() as i32
    }

    pub fn update(&mut self) {
        let step = self.velocity.magnitude();
        self.distance_traveled += step;
        self.position = self.position + self.velocity;
    }

    /// Update position with a fractional time step.
    /// `dt` should be 1.0/substeps so the total displacement per full turn
    /// is the same as a single `update()` call.
    pub fn update_substep(&mut self, dt: f64) {
        let step = (self.velocity * dt).magnitude();
        self.distance_traveled += step;
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

    #[test]
    fn effective_damage_min_at_zero_distance() {
        let proj = Projectile {
            position: Vec2::new(0.0, 0.0),
            velocity: Vec2::new(20.0, 0.0),
            damage: 10,
            owner: 0,
            distance_traveled: 0.0,
        };
        let dmg = proj.effective_damage();
        // 25% of 10 = 2.5, rounded to 3
        assert_eq!(dmg, 3);
    }

    #[test]
    fn effective_damage_full_at_arm_distance() {
        let proj = Projectile {
            position: Vec2::new(0.0, 0.0),
            velocity: Vec2::new(20.0, 0.0),
            damage: 10,
            owner: 0,
            distance_traveled: ship::PROJECTILE_ARM_DISTANCE,
        };
        assert_eq!(proj.effective_damage(), 10);
    }

    #[test]
    fn effective_damage_full_beyond_arm_distance() {
        let proj = Projectile {
            position: Vec2::new(0.0, 0.0),
            velocity: Vec2::new(20.0, 0.0),
            damage: 25,
            owner: 0,
            distance_traveled: 100.0,
        };
        assert_eq!(proj.effective_damage(), 25);
    }

    #[test]
    fn distance_tracked_on_update() {
        let ship = ship::Ship::new(Vec2::new(100.0, 100.0), 0.0);
        let mut proj = Projectile::spawn_primary(&ship, 0);
        assert!((proj.distance_traveled - 0.0).abs() < 1e-10);
        proj.update();
        assert!(proj.distance_traveled > 0.0);
    }

    #[test]
    fn distance_tracked_on_substep() {
        let ship = ship::Ship::new(Vec2::new(100.0, 100.0), 0.0);
        let mut proj = Projectile::spawn_primary(&ship, 0);
        proj.update_substep(0.25);
        assert!(proj.distance_traveled > 0.0);
    }
}
