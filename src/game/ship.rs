use serde::{Deserialize, Serialize};
use super::Vec2;

// -- Constants --

pub const MAX_HEALTH: i32 = 100;
pub const MAX_ENERGY: i32 = 100;
pub const ENERGY_REGEN_PER_TURN: i32 = 3;
pub const SHIELD_ENERGY_COST: i32 = 5;
pub const PRIMARY_DAMAGE: i32 = 10;
pub const PRIMARY_COOLDOWN: i32 = 2;
pub const PRIMARY_ENERGY_COST: i32 = 5;
pub const SECONDARY_DAMAGE: i32 = 25;
pub const SECONDARY_COOLDOWN: i32 = 8;
pub const SECONDARY_ENERGY_COST: i32 = 15;
pub const MAX_SPEED: f64 = 15.0;
pub const THRUST_FORCE: f64 = 3.0;
pub const DRAG: f64 = 0.95;
pub const MAX_TURN_RATE: f64 = 30.0;
pub const SENSOR_RANGE: f64 = 150.0;
pub const SHIELD_DAMAGE_REDUCTION: f64 = 0.5;
pub const PRIMARY_PROJECTILE_SPEED: f64 = 20.0;
pub const SECONDARY_PROJECTILE_SPEED: f64 = 12.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ship {
    pub position: Vec2,
    pub velocity: Vec2,
    pub heading: f64,
    pub health: i32,
    pub energy: i32,
    pub shield_active: bool,
    pub primary_cooldown: i32,
    pub secondary_cooldown: i32,
    pub shots_fired: i32,
    pub shots_hit: i32,
    pub damage_dealt: i32,
}

impl Ship {
    pub fn new(position: Vec2, heading: f64) -> Self {
        Self {
            position,
            velocity: Vec2::zero(),
            heading,
            health: MAX_HEALTH,
            energy: MAX_ENERGY,
            shield_active: false,
            primary_cooldown: 0,
            secondary_cooldown: 0,
            shots_fired: 0,
            shots_hit: 0,
            damage_dealt: 0,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0
    }

    pub fn apply_thrust(&mut self, thrust: f64) {
        let thrust = thrust.clamp(0.0, 1.0);
        let rad = self.heading.to_radians();
        let accel = Vec2::new(rad.cos(), rad.sin()) * (THRUST_FORCE * thrust);
        self.velocity = self.velocity + accel;
        let speed = self.velocity.magnitude();
        if speed > MAX_SPEED {
            self.velocity = self.velocity * (MAX_SPEED / speed);
        }
    }

    pub fn apply_turn(&mut self, turn_degrees: f64) {
        let turn = turn_degrees.clamp(-MAX_TURN_RATE, MAX_TURN_RATE);
        self.heading = (self.heading + turn) % 360.0;
        if self.heading < 0.0 {
            self.heading += 360.0;
        }
    }

    pub fn update_position(&mut self) {
        self.position = self.position + self.velocity;
        self.velocity = self.velocity * DRAG;
    }

    pub fn tick_cooldowns(&mut self) {
        self.primary_cooldown = (self.primary_cooldown - 1).max(0);
        self.secondary_cooldown = (self.secondary_cooldown - 1).max(0);
    }

    pub fn regen_energy(&mut self) {
        if self.shield_active {
            self.energy = (self.energy - SHIELD_ENERGY_COST).max(0);
            if self.energy == 0 {
                self.shield_active = false;
            }
        } else {
            self.energy = (self.energy + ENERGY_REGEN_PER_TURN).min(MAX_ENERGY);
        }
    }

    pub fn can_fire_primary(&self) -> bool {
        self.primary_cooldown == 0 && self.energy >= PRIMARY_ENERGY_COST
    }

    pub fn can_fire_secondary(&self) -> bool {
        self.secondary_cooldown == 0 && self.energy >= SECONDARY_ENERGY_COST
    }

    pub fn fire_primary(&mut self) {
        self.primary_cooldown = PRIMARY_COOLDOWN;
        self.energy -= PRIMARY_ENERGY_COST;
        self.shots_fired += 1;
    }

    pub fn fire_secondary(&mut self) {
        self.secondary_cooldown = SECONDARY_COOLDOWN;
        self.energy -= SECONDARY_ENERGY_COST;
        self.shots_fired += 1;
    }

    pub fn take_damage(&mut self, damage: i32) -> i32 {
        let actual = if self.shield_active {
            (damage as f64 * SHIELD_DAMAGE_REDUCTION) as i32
        } else {
            damage
        };
        self.health = (self.health - actual).max(0);
        actual
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ship_thrust_increases_velocity() {
        let mut ship = Ship::new(Vec2::new(100.0, 100.0), 0.0);
        ship.apply_thrust(1.0);
        assert!(ship.velocity.x > 0.0);
        assert!(ship.velocity.y.abs() < 1e-10);
    }

    #[test]
    fn ship_thrust_clamped() {
        let mut ship = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        ship.apply_thrust(2.0);
        let v1 = ship.velocity.magnitude();
        let mut ship2 = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        ship2.apply_thrust(1.0);
        assert!((v1 - ship2.velocity.magnitude()).abs() < 1e-10);
    }

    #[test]
    fn ship_max_speed_cap() {
        let mut ship = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        for _ in 0..100 {
            ship.apply_thrust(1.0);
        }
        assert!(ship.velocity.magnitude() <= MAX_SPEED + 1e-10);
    }

    #[test]
    fn ship_turn_wraps() {
        let mut ship = Ship::new(Vec2::new(0.0, 0.0), 350.0);
        ship.apply_turn(20.0);
        assert!((ship.heading - 10.0).abs() < 1e-10);
    }

    #[test]
    fn ship_shield_reduces_damage() {
        let mut ship = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        ship.shield_active = true;
        let dmg = ship.take_damage(20);
        assert_eq!(dmg, 10);
        assert_eq!(ship.health, 90);
    }

    #[test]
    fn ship_shield_drains_energy() {
        let mut ship = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        ship.shield_active = true;
        ship.regen_energy();
        assert_eq!(ship.energy, MAX_ENERGY - SHIELD_ENERGY_COST);
    }

    #[test]
    fn ship_fire_primary_cooldown() {
        let mut ship = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        assert!(ship.can_fire_primary());
        ship.fire_primary();
        assert!(!ship.can_fire_primary());
        assert_eq!(ship.primary_cooldown, PRIMARY_COOLDOWN);
    }

    #[test]
    fn ship_dead_at_zero_health() {
        let mut ship = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        ship.take_damage(100);
        assert!(!ship.is_alive());
    }
}
