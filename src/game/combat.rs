use super::arena::{Arena, HIT_RADIUS, BOUNDARY_DAMAGE};
use super::projectile::Projectile;
use super::ship::{self, Ship};

/// Events generated during a turn for the marquee.
#[derive(Debug, Clone)]
pub enum GameEvent {
    ShipFiredPrimary(usize),
    ShipFiredSecondary(usize),
    ShipHit { target: usize, damage: i32 },
    ShipDestroyed(usize),
    ShieldActivated(usize),
    ShieldDeactivated(usize),
    BoundaryHit(usize),
    RamDamage { ship: usize, damage: i32 },
}

/// Resolve one tick of combat. Returns events that occurred.
pub fn resolve_projectile_hits(
    ships: &mut [Ship; 2],
    projectiles: &mut Vec<Projectile>,
) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut to_remove = Vec::new();

    for (i, proj) in projectiles.iter().enumerate() {
        let target = 1 - proj.owner;
        if proj.hits_ship(&ships[target], HIT_RADIUS) {
            let dmg = ships[target].take_damage(proj.effective_damage());
            ships[proj.owner].shots_hit += 1;
            ships[proj.owner].damage_dealt += dmg;
            events.push(GameEvent::ShipHit { target, damage: dmg });
            if !ships[target].is_alive() {
                events.push(GameEvent::ShipDestroyed(target));
            }
            to_remove.push(i);
        }
    }

    // Remove hit projectiles in reverse order to preserve indices
    for i in to_remove.into_iter().rev() {
        projectiles.swap_remove(i);
    }

    events
}

/// Resolve ram damage when ships are close and approaching fast.
pub fn resolve_ram_damage(ships: &mut [Ship; 2]) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let dist = ships[0].position.distance_to(ships[1].position);
    if !(1e-10..ship::RAM_RADIUS).contains(&dist) {
        return events;
    }

    // Direction from ship 0 to ship 1
    let dir = (ships[1].position - ships[0].position) * (1.0 / dist);

    // Closing speed of each ship along the line between them
    // Positive = approaching the other ship
    let closing_0 = ships[0].velocity.x * dir.x + ships[0].velocity.y * dir.y;
    let closing_1 = -(ships[1].velocity.x * dir.x + ships[1].velocity.y * dir.y);

    let total_closing = closing_0 + closing_1;
    if total_closing <= ship::RAM_SPEED_THRESHOLD {
        return events;
    }

    let proximity_factor = 1.0 - dist / ship::RAM_RADIUS;
    let closings = [closing_0, closing_1];

    for (i, ship) in ships.iter_mut().enumerate() {
        let my_closing = closings[i];
        let other_closing = closings[1 - i];
        let speed_total = (my_closing + other_closing).max(1.0);

        // Rammer damage: proportional to your own closing speed
        let rammer_dmg = (ship::RAM_DAMAGE_RAMMER as f64
            * (my_closing / speed_total)
            * proximity_factor)
            .round() as i32;
        // Receiver damage: proportional to the other's closing speed
        let receiver_dmg = (ship::RAM_DAMAGE_RECEIVER as f64
            * (other_closing / speed_total)
            * proximity_factor)
            .round() as i32;

        let total = (rammer_dmg + receiver_dmg).max(0);
        if total > 0 {
            ship.take_damage(total);
            events.push(GameEvent::RamDamage { ship: i, damage: total });
            if !ship.is_alive() {
                events.push(GameEvent::ShipDestroyed(i));
            }
        }
    }

    events
}

/// Apply repulsion force when ships overlap within KNOCKBACK_RADIUS.
pub fn apply_proximity_knockback(ships: &mut [Ship; 2]) {
    let dist = ships[0].position.distance_to(ships[1].position);
    if !(1e-10..ship::KNOCKBACK_RADIUS).contains(&dist) {
        return;
    }

    let strength = ship::KNOCKBACK_FORCE * (1.0 - dist / ship::KNOCKBACK_RADIUS);

    // Direction from ship 0 to ship 1
    let dir = (ships[1].position - ships[0].position) * (1.0 / dist);
    let push = dir * strength;

    // Ship 0 pushed away from ship 1 (negative direction)
    ships[0].velocity = ships[0].velocity - push;
    // Ship 1 pushed away from ship 0 (positive direction)
    ships[1].velocity = ships[1].velocity + push;

    // Re-clamp velocities to MAX_SPEED
    for s in ships.iter_mut() {
        let speed = s.velocity.magnitude();
        if speed > ship::MAX_SPEED {
            s.velocity = s.velocity * (ship::MAX_SPEED / speed);
        }
    }
}

/// Apply boundary rules and return events.
pub fn resolve_boundaries(
    ships: &mut [Ship; 2],
    arena: &Arena,
) -> Vec<GameEvent> {
    let mut events = Vec::new();
    for (i, ship) in ships.iter_mut().enumerate() {
        if arena.enforce_boundary(ship) {
            ship.take_damage(BOUNDARY_DAMAGE);
            events.push(GameEvent::BoundaryHit(i));
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Vec2;

    #[test]
    fn projectile_hits_target() {
        let mut ships = [
            Ship::new(Vec2::new(100.0, 100.0), 0.0),
            Ship::new(Vec2::new(105.0, 100.0), 180.0),
        ];
        let mut projectiles = vec![Projectile {
            position: Vec2::new(105.0, 100.0),
            velocity: Vec2::new(10.0, 0.0),
            damage: 10,
            owner: 0,
            distance_traveled: 50.0, // fully armed
        }];
        let events = resolve_projectile_hits(&mut ships, &mut projectiles);
        assert_eq!(ships[1].health, 90);
        assert!(projectiles.is_empty());
        assert!(events.iter().any(|e| matches!(e, GameEvent::ShipHit { target: 1, .. })));
    }

    #[test]
    fn projectile_misses() {
        let mut ships = [
            Ship::new(Vec2::new(100.0, 100.0), 0.0),
            Ship::new(Vec2::new(500.0, 500.0), 180.0),
        ];
        let mut projectiles = vec![Projectile {
            position: Vec2::new(200.0, 200.0),
            velocity: Vec2::new(10.0, 0.0),
            damage: 10,
            owner: 0,
            distance_traveled: 50.0,
        }];
        let events = resolve_projectile_hits(&mut ships, &mut projectiles);
        assert_eq!(ships[1].health, 100);
        assert_eq!(projectiles.len(), 1);
        assert!(events.is_empty());
    }

    #[test]
    fn projectile_point_blank_reduced_damage() {
        let mut ships = [
            Ship::new(Vec2::new(100.0, 100.0), 0.0),
            Ship::new(Vec2::new(105.0, 100.0), 180.0),
        ];
        let mut projectiles = vec![Projectile {
            position: Vec2::new(105.0, 100.0),
            velocity: Vec2::new(10.0, 0.0),
            damage: 10,
            owner: 0,
            distance_traveled: 0.0, // not armed at all
        }];
        let events = resolve_projectile_hits(&mut ships, &mut projectiles);
        // 25% of 10 = 2.5, rounded to 3
        assert_eq!(ships[1].health, 97);
        assert!(events.iter().any(|e| matches!(e, GameEvent::ShipHit { target: 1, damage: 3 })));
    }

    #[test]
    fn ram_damage_triggers_when_closing_fast() {
        let mut ships = [
            Ship::new(Vec2::new(100.0, 100.0), 0.0),
            Ship::new(Vec2::new(110.0, 100.0), 180.0), // 10 units apart
        ];
        // Both ships charging toward each other
        ships[0].velocity = Vec2::new(20.0, 0.0);
        ships[1].velocity = Vec2::new(-20.0, 0.0);
        let events = resolve_ram_damage(&mut ships);
        assert!(!events.is_empty(), "Ram events should fire");
        assert!(events.iter().any(|e| matches!(e, GameEvent::RamDamage { .. })));
        assert!(ships[0].health < 100);
        assert!(ships[1].health < 100);
    }

    #[test]
    fn no_ram_when_far_apart() {
        let mut ships = [
            Ship::new(Vec2::new(100.0, 100.0), 0.0),
            Ship::new(Vec2::new(200.0, 100.0), 180.0), // 100 units apart
        ];
        ships[0].velocity = Vec2::new(20.0, 0.0);
        ships[1].velocity = Vec2::new(-20.0, 0.0);
        let events = resolve_ram_damage(&mut ships);
        assert!(events.is_empty());
    }

    #[test]
    fn no_ram_when_orbiting() {
        let mut ships = [
            Ship::new(Vec2::new(100.0, 100.0), 0.0),
            Ship::new(Vec2::new(110.0, 100.0), 180.0), // 10 units apart
        ];
        // Both ships moving perpendicular - closing speed ~0
        ships[0].velocity = Vec2::new(0.0, 20.0);
        ships[1].velocity = Vec2::new(0.0, -20.0);
        let events = resolve_ram_damage(&mut ships);
        assert!(events.is_empty(), "No ram when orbiting (closing speed below threshold)");
    }

    #[test]
    fn knockback_pushes_ships_apart() {
        let mut ships = [
            Ship::new(Vec2::new(100.0, 100.0), 0.0),
            Ship::new(Vec2::new(108.0, 100.0), 180.0), // 8 units apart, within KNOCKBACK_RADIUS
        ];
        ships[0].velocity = Vec2::zero();
        ships[1].velocity = Vec2::zero();
        apply_proximity_knockback(&mut ships);
        // Ship 0 should be pushed left (negative x), ship 1 pushed right (positive x)
        assert!(ships[0].velocity.x < 0.0, "Ship 0 should be pushed away (negative x)");
        assert!(ships[1].velocity.x > 0.0, "Ship 1 should be pushed away (positive x)");
    }

    #[test]
    fn no_knockback_when_far() {
        let mut ships = [
            Ship::new(Vec2::new(100.0, 100.0), 0.0),
            Ship::new(Vec2::new(200.0, 100.0), 180.0), // 100 units apart
        ];
        apply_proximity_knockback(&mut ships);
        assert!((ships[0].velocity.magnitude()).abs() < 1e-10);
        assert!((ships[1].velocity.magnitude()).abs() < 1e-10);
    }
}
