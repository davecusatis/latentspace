use super::ship::{Ship, SHIP_COLLISION_RADIUS};

pub const HIT_RADIUS: f64 = SHIP_COLLISION_RADIUS;
pub const BOUNDARY_DAMAGE: i32 = 5;

#[derive(Debug, Clone)]
pub struct Arena {
    pub width: f64,
    pub height: f64,
}

impl Arena {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// Clamp ship to arena bounds, accounting for ship collision radius.
    /// Returns true if ship hit boundary.
    pub fn enforce_boundary(&self, ship: &mut Ship) -> bool {
        let r = SHIP_COLLISION_RADIUS;
        let mut hit = false;
        if ship.position.x < r {
            ship.position.x = r;
            ship.velocity.x = ship.velocity.x.abs();
            hit = true;
        } else if ship.position.x > self.width - r {
            ship.position.x = self.width - r;
            ship.velocity.x = -ship.velocity.x.abs();
            hit = true;
        }
        if ship.position.y < r {
            ship.position.y = r;
            ship.velocity.y = ship.velocity.y.abs();
            hit = true;
        } else if ship.position.y > self.height - r {
            ship.position.y = self.height - r;
            ship.velocity.y = -ship.velocity.y.abs();
            hit = true;
        }
        hit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Vec2;

    #[test]
    fn boundary_bounces_ship() {
        let arena = Arena::new(800.0, 400.0);
        let mut ship = Ship::new(Vec2::new(-5.0, 200.0), 180.0);
        ship.velocity = Vec2::new(-10.0, 0.0);
        let hit = arena.enforce_boundary(&mut ship);
        assert!(hit);
        assert_eq!(ship.position.x, SHIP_COLLISION_RADIUS);
        assert!(ship.velocity.x > 0.0);
    }

    #[test]
    fn no_bounce_when_inside() {
        let arena = Arena::new(800.0, 400.0);
        let mut ship = Ship::new(Vec2::new(400.0, 200.0), 0.0);
        let hit = arena.enforce_boundary(&mut ship);
        assert!(!hit);
    }
}
