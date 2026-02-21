# AI Module

Runs Lua scripts that control each ship. Each ship gets its own sandboxed Lua VM; scripts return commands each turn via a `think()` function.

## Files

- **script.rs** — `ScriptAgent` struct. Creates a Lua 5.4 VM (`mlua`), loads user script, sandboxes it (removes `io`/`os`/`debug`/`loadfile`/`dofile`/`require`), sets 100k instruction limit per call. Provides a persistent `__memory` table across turns. Calls `think(state, memory)` and parses the returned Lua table into `ShipCommand`. Returns default drift command on any error.
- **protocol.rs** — Builds `GameStateMessage` (fog-filtered per observer) and serializes it to a Lua table via `mlua` serde. Computes bearing, `turn_to_aim`, and `lead_turn_to_aim` for scripts. Also contains `parse_command()` for legacy JSON parsing.
- **validate.rs** — 5-check validation harness run via `cargo run -- validate <script>`. Checks: (1) Lua compiles, (2) `think()` function exists, (3) `think()` runs with enemy visible, (4) return values in range (thrust 0..=1, turn -30..=30), (5) `think()` runs with enemy nil (fog of war). Early-returns on compile or missing-function failure.

## Key Types

```rust
// Passed to think() as a Lua table
GameStateMessage { turn, self_ship, enemy: Option<...>, detected_projectiles, arena, sensor_range, detected_by_enemy }

// Returned from think() as a Lua table
ShipCommand { thrust: 0.0–1.0, turn: -30..30, fire_primary, fire_secondary, shield }
```

## Lua Script Contract

Scripts must define:
```lua
function think(state, memory)
    -- state: GameStateMessage table (state.self_ship, state.enemy, etc.)
    -- memory: persistent table across turns (store whatever you want)
    -- return: { thrust, turn, fire_primary, fire_secondary, shield }
    return { thrust = 1.0, turn = 0, fire_primary = true, fire_secondary = false, shield = false }
end
```

- `state.enemy` is `nil` when no enemy is within sensor range (fog of war)
- Missing return fields default to zero/false
- Any runtime error returns default drift command (all zeros)

## Conventions

- AI never sees enemies outside `SENSOR_RANGE` (150 units)
- Lead angle accounts for target velocity and projectile travel time
- Bearing is absolute angle (0=right, 90=down); `turn_to_aim` is signed delta from current heading
- Default command: all zeros (drift, no fire, no shield)
- Game loop is fully synchronous — no async, no network calls
