# Latentspace

AI Spaceship Deathmatch Arena — two AI agents (Google Gemini) pilot ships in a 2D arena rendered in the terminal using braille characters.

## Architecture

```
src/
  ai/       — Gemini API client, conversation history, game state protocol
  canvas/   — Braille pixel canvas renderer + sprite drawing
  game/     — Pure simulation: physics, combat, fog of war
  ui/       — Ratatui widgets: HUD, layout, marquee, debug overlay
  main.rs   — Game loop (Phase 1: AI wait + render, Phase 2: interpolation)
  lib.rs    — Re-exports all modules
```

## Game Loop (main.rs)

Each turn has two render phases:

1. **Phase 1** — Send game state JSON to both AIs concurrently. Render at ~60 FPS while waiting, extrapolating projectile positions. Tick ongoing explosions.
2. **Advance** — `game.advance([cmd1, cmd2])` processes one simulation turn. Spawn explosions from `ShipHit`/`ShipDestroyed` events.
3. **Phase 2** — Interpolate ship positions from old to new over `interp_dur` (150–1000ms based on `--speed`). Render projectiles at final positions. Tick explosions.

## Key Conventions

- **Coordinate system**: origin top-left, +x right, +y down. Heading 0=right, 90=down.
- **Braille rendering**: each terminal cell = 2x4 pixels. Canvas dimensions = terminal width*2 x height*4.
- **Fog of war**: ships/projectiles only visible within `SENSOR_RANGE` (150 units). AI receives filtered state.
- **Graceful degradation**: AI timeout or parse error -> default drift command. Out-of-bounds drawing silently ignored.
- **Events**: `GameEvent` variants generated during `advance()`, consumed by marquee and explosion spawning, cleared each turn.

## Running

```bash
cargo run -- --ship1 prompts/aggressive.txt --ship2 prompts/aggressive.txt --api-key $GOOGLE_API_KEY
```

Press `d` to toggle debug overlay, `Ctrl+C` to quit.

## Testing

```bash
cargo test        # 104 tests across all modules + integration
cargo clippy      # Must pass with zero warnings
```
