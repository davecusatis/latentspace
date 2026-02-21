# Latentspace

AI Spaceship Deathmatch Arena — two Lua-scripted ships battle in a 2D arena rendered in the terminal using braille characters.

## Architecture

```
src/
  ai/       — Lua script runner, validation harness, game state protocol
  canvas/   — Braille pixel canvas renderer + sprite drawing
  game/     — Pure simulation: physics, combat, fog of war
  ui/       — Ratatui widgets: HUD, layout, marquee, debug overlay
  main.rs   — Game loop (synchronous: get commands, advance, interpolate)
  lib.rs    — Re-exports all modules
scripts/    — Lua ship AI scripts (.lua files)
```

## Game Loop (main.rs)

Each turn:

1. **Build state** — `build_game_state()` creates fog-filtered `GameStateMessage` for each ship.
2. **Call scripts** — `ScriptAgent::get_command()` calls `think(state, memory)` in each ship's Lua VM (instant, synchronous).
3. **Advance** — `game.advance([cmd1, cmd2])` processes one simulation turn. Spawn explosions from `ShipHit`/`ShipDestroyed`/`RamDamage` events.
4. **Interpolate** — Interpolate ship positions from old to new over `interp_dur` (150–1000ms based on `--speed`). Render projectiles at final positions. Tick explosions.

## Key Conventions

- **Coordinate system**: origin top-left, +x right, +y down. Heading 0=right, 90=down.
- **Braille rendering**: each terminal cell = 2x4 pixels. Canvas dimensions = terminal width*2 x height*4.
- **Fog of war**: ships/projectiles only visible within `SENSOR_RANGE` (150 units). Scripts receive filtered state.
- **Graceful degradation**: script error -> default drift command. Out-of-bounds drawing silently ignored.
- **Events**: `GameEvent` variants generated during `advance()`, consumed by marquee and explosion spawning, cleared each turn.

## Running

```bash
# Play a match between two Lua scripts
cargo run -- play --ship1 scripts/a.lua --ship2 scripts/b.lua

# Validate a script (5 checks: compile, think exists, runs with enemy, valid types, runs with nil enemy)
cargo run -- validate scripts/a.lua
```

Press `d` to toggle debug overlay, `Ctrl+C` to quit.

## Testing

```bash
cargo test        # unit + integration tests across all modules
cargo clippy      # Must pass with zero warnings
```
