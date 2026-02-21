# Lua Scripting for Ship Control

**Date:** 2026-02-20
**Status:** Approved

## Overview

Replace the per-turn Gemini LLM API calls with pre-written Lua scripts that run locally. Scripts are authored by LLM agents based on user feedback, validated instantly via a CLI command, and executed in a sandboxed Lua VM each turn.

## Motivation

The current architecture sends game state to Gemini each turn (~5s network latency, API cost, non-deterministic). Lua scripts run in microseconds, are deterministic, and create a tight feedback loop: LLM writes script, validates it, user watches the match, gives feedback, LLM iterates.

## Script Interface

Each ship gets a `.lua` file that defines a `think` function:

```lua
function think(state, memory)
    -- state: read-only game state table
    -- memory: read-write table that persists across turns
    -- returns: command table
    return {
        thrust = 1.0,       -- 0.0 to 1.0
        turn = 0,            -- -30 to +30 degrees
        fire_primary = false,
        fire_secondary = false,
        shield = false
    }
end
```

Missing command fields default to `0`/`false`.

### State Table Structure

```lua
state = {
    turn = 5,
    self_ship = {
        position = { x = 100.0, y = 200.0 },
        velocity = { x = 5.0, y = -3.0 },
        heading = 45.0,
        health = 85,
        energy = 70,
        shield_active = false,
        weapon_cooldowns = { primary = 0, secondary = 3 }
    },
    enemy = {  -- nil if outside sensor range
        position = { x = 300.0, y = 150.0 },
        velocity = { x = -2.0, y = 1.0 },
        heading = 180.0,
        distance = 120.5,
        bearing = 30.0,
        turn_to_aim = 15.0,
        lead_turn_to_aim = 18.5,
        closing_speed = 7.0
    },
    detected_projectiles = {
        { position = { x = 150.0, y = 180.0 }, velocity = { x = 10.0, y = 0.0 } }
    },
    arena = { width = 800, height = 600 },
    sensor_range = 150.0,
    detected_by_enemy = true
}
```

## Architecture Changes

### New flow

```
GameState -> Lua table -> call think() (microseconds) -> Lua table -> ShipCommand
```

Replaces:

```
GameState -> JSON -> Gemini API (5s network call) -> JSON -> ShipCommand
```

### What changes

- **New `ScriptAgent`** in `src/ai/` replaces `AiAgent`. Holds a Lua VM instance, the persistent `memory` table, and calls `think()` each turn.
- **Remove `AiAgent` / Gemini integration**: `client.rs`, `history.rs`, `reqwest` dependency all removed.
- **Simplify game loop**: No more async API calls or "render while waiting" phase. Turn becomes synchronous: call both scripts, advance simulation, render interpolation.
- **CLI change**: `--ship1 scripts/hunter.lua --ship2 scripts/duelist.lua`
- **Script files** live in `scripts/` instead of `prompts/`.

### What stays the same

- All of `src/game/` (simulation, physics, combat)
- All of `src/canvas/` and `src/ui/`
- `ShipCommand` struct
- `GameStateMessage` / protocol types (reused to build Lua state table)

## Sandbox & Safety

### Allowed Lua libraries

- `math` (trig, min/max, random)
- `string` (manipulation, formatting)
- `table` (insert, sort, etc.)

### Removed entirely

- `io` (no file access)
- `os` (no system calls)
- `debug` (no VM introspection)
- `loadfile` / `dofile` (no loading external code)
- `require` (no module system)

### Runtime limits

- Instruction count limit (~100,000 per `think()` call) to prevent infinite loops
- On error or limit exceeded, return default drift command (same as current Gemini timeout behavior)

### Memory table

- Persists across turns for that ship
- Plain Lua table, scripts store whatever they want
- Resets each match

## Validation & Test Harness

### CLI validation command

```
cargo run -- validate scripts/hunter.lua
```

Runs 5 checks:

1. **Parse check**: Does the Lua compile?
2. **Function check**: Does a `think` function exist?
3. **Execution check**: Call `think()` with mock state (enemy visible), does it return without error?
4. **Return type check**: Valid field types? (thrust: number 0-1, turn: number -30 to 30, booleans are booleans)
5. **Nil enemy check**: Call `think()` with `state.enemy = nil`, does the script handle fog of war?

Output on success:

```
Validating scripts/hunter.lua...
  [PASS] Lua compiles
  [PASS] think() function exists
  [PASS] think() runs with enemy visible
  [PASS] Return value is valid ShipCommand
  [PASS] think() runs with enemy nil (fog of war)

  Result: 5/5 checks passed

  Command output (enemy visible):
    thrust=1.0  turn=18.5  fire_primary=true  fire_secondary=false  shield=false
  Command output (enemy nil):
    thrust=0.8  turn=15.0  fire_primary=false  fire_secondary=false  shield=false
```

Output on failure:

```
  [FAIL] think() runs with enemy nil (fog of war)
    Error: scripts/hunter.lua:12: attempt to index a nil value (field 'enemy')
```

### Integration tests

`tests/lua_scripts.rs` validates all scripts in `scripts/` automatically, catches regressions if the state interface changes.

## Example Scripts

### `scripts/aggressive.lua`

```lua
function think(state, memory)
    if state.enemy then
        return {
            thrust = 1.0,
            turn = state.enemy.lead_turn_to_aim,
            fire_primary = state.enemy.distance < 120,
            fire_secondary = state.enemy.distance < 80
        }
    end
    return { thrust = 0.8, turn = 15 }
end
```

### `scripts/defensive.lua`

```lua
function think(state, memory)
    memory.turns_since_seen = (memory.turns_since_seen or 0) + 1

    if state.enemy then
        memory.turns_since_seen = 0
        memory.last_enemy_pos = state.enemy.position

        if state.enemy.distance < 50 then
            return {
                thrust = 1.0,
                turn = state.enemy.turn_to_aim + 180,
                shield = state.self_ship.energy > 30
            }
        elseif state.enemy.distance < 100 then
            return {
                thrust = 0.6,
                turn = state.enemy.lead_turn_to_aim + 12,
                fire_primary = true,
                fire_secondary = state.self_ship.energy > 40
            }
        else
            return { thrust = 0.7, turn = state.enemy.lead_turn_to_aim }
        end
    end

    return { thrust = 0.6, turn = 20 }
end
```
