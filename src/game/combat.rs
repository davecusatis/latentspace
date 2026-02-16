use super::arena::{Arena, HIT_RADIUS, BOUNDARY_DAMAGE};
use super::projectile::Projectile;
use super::ship::Ship;

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
            let dmg = ships[target].take_damage(proj.damage);
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
        }];
        let events = resolve_projectile_hits(&mut ships, &mut projectiles);
        assert_eq!(ships[1].health, 100);
        assert_eq!(projectiles.len(), 1);
        assert!(events.is_empty());
    }
}
