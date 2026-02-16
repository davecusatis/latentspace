# Game Module

Pure simulation logic — no rendering, no IO. All state lives in `GameState`.

## Files

- **mod.rs** — `Vec2` type (position/velocity). Implements `Add`, `Mul<f64>`, `lerp`, `distance_to`, `magnitude`.
- **simulation.rs** — `GameState` and turn processing. `advance([ShipCommand; 2])` runs one full turn. `result()` checks win conditions.
- **ship.rs** — `Ship` entity with physics, weapons, energy, shield. All game constants defined here.
- **projectile.rs** — `Projectile` entity. Spawned ahead of ship with inherited velocity. Sub-stepped movement for accurate collision detection.
- **combat.rs** — `GameEvent` enum and hit resolution. `resolve_projectile_hits` checks collisions, `resolve_boundaries` handles arena walls.
- **arena.rs** — `Arena` boundary enforcement. Ships bounce off walls (velocity reflected) and take 5 damage.
- **fog.rs** — Fog of war. `is_visible` checks sensor range. `visible_projectiles` filters by range.

## Turn Sequence (simulation.rs::advance)

1. Process shield activation/deactivation
2. Apply thrust and turning to ships
3. Fire weapons (check cooldowns + energy, spawn projectiles, emit events)
4. Update ship positions (velocity * 1 + drag)
5. Sub-stepped projectile updates (4 sub-steps per turn):
   - Move projectiles by `velocity * 0.25`
   - Detect projectile-ship collisions
   - Remove out-of-bounds projectiles
6. Enforce arena boundaries (bounce + damage)
7. Tick weapon cooldowns
8. Regenerate energy

## Constants (ship.rs)

| Physics | Value | Weapons | Primary | Secondary |
|---------|-------|---------|---------|-----------|
| MAX_SPEED | 35 | DAMAGE | 10 | 25 |
| THRUST_FORCE | 10 | COOLDOWN | 2 turns | 5 turns |
| DRAG | 0.92 | ENERGY_COST | 5 | 15 |
| MAX_TURN_RATE | 30 deg | PROJ_SPEED | 20 | 16 |
| SENSOR_RANGE | 150 | | | |
| COLLISION_RADIUS | 8 | | | |
| MAX_HEALTH | 100 | SHIELD_REDUCTION | 0.5 | |
| MAX_ENERGY | 100 | SHIELD_COST | 5/turn | |
| ENERGY_REGEN | 5/turn | BOUNDARY_DMG | 5 | |

## GameEvent Variants

```rust
ShipFiredPrimary(usize), ShipFiredSecondary(usize),
ShipHit { target: usize, damage: i32 }, ShipDestroyed(usize),
ShieldActivated(usize), ShieldDeactivated(usize), BoundaryHit(usize)
```

Events are cleared at the start of each `advance()` call.

## Conventions

- Ships start at (20%, 50%) and (80%, 50%) of arena, facing center
- Projectile sub-stepping (4 steps) prevents tunneling through ships
- Shield reduces damage by 50%, costs 5 energy/turn, auto-deactivates at 0 energy
- Heading wraps at 360 degrees
