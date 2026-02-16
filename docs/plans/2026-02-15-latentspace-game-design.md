# LatentSpace — AI Spaceship Deathmatch Arena

## Overview

A terminal-based competitive arena where AI agents, directed via user-authored prompts, pilot spaceships in 1v1 deathmatch. The game uses Claude as the AI backend, renders with half-block pixels and true color via ratatui, and layers tachyonfx shader effects for visual polish. Turn-based logic with interpolated rendering creates a real-time feel.

## Game World & Physics

- **Arena:** Bounded 2D space (default 800x400 game units). Ships take damage or bounce at boundaries to force engagement.
- **Ships:** Position, velocity, heading, health (100 HP), shield (absorbs damage, drains energy), energy (shared resource for shields/weapons/boost), sensor range.
- **Physics:** Newtonian — thrust accelerates along heading, max speed cap, slight drag. Projectiles travel straight at fixed speed.
- **Weapons:**
  - Primary: rapid-fire projectiles (low damage, fast cooldown)
  - Secondary: heavy shot (high damage, slow cooldown, higher energy cost)
- **Win condition:** Reduce opponent HP to 0. Timeout after N turns (default 200) — highest HP wins.

## AI Agent Interface

### Game State (sent to AI each turn)

```json
{
  "turn": 42,
  "self": {
    "position": { "x": 350.0, "y": 200.0 },
    "velocity": { "x": 5.0, "y": -2.0 },
    "heading": 45.0,
    "health": 85,
    "energy": 60,
    "shield_active": false,
    "weapon_cooldowns": { "primary": 0, "secondary": 3 }
  },
  "enemy": null,
  "detected_projectiles": [
    { "position": { "x": 300.0, "y": 210.0 }, "velocity": { "x": -10.0, "y": 0.0 } }
  ],
  "arena": { "width": 800, "height": 400 },
  "sensor_range": 150.0
}
```

`enemy` is `null` when opponent is outside sensor range (fog of war). When visible, shows position, velocity, heading — but not health, energy, or cooldowns.

### Command (AI response)

```json
{
  "thrust": 0.8,
  "turn": -15.0,
  "fire_primary": true,
  "fire_secondary": false,
  "shield": false
}
```

- `thrust`: 0.0 to 1.0
- `turn`: -30 to +30 degrees per turn
- `fire_primary` / `fire_secondary`: booleans
- `shield`: toggle

### Prompt Structure

User provides a system prompt file defining the AI strategy. The game engine appends game state as a user message each turn. Previous turns included via sliding window to manage token cost. 5-second timeout — no response means the ship drifts.

## Turn Resolution & Interpolation

### Turn sequence

1. Send game state to both AIs simultaneously (parallel async)
2. Wait for responses (5s timeout)
3. Resolve atomically: apply thrust/rotation, update positions, spawn projectiles, move projectiles, check collisions, apply damage, update energy, recalculate fog of war, remove dead projectiles
4. Check win condition
5. Render interpolated frames between previous and new state

### Interpolation & Latency Masking

Between turns, the renderer smoothly animates ships and projectiles from old to new positions over ~0.5-1 second. During this animation, the next turn's AI requests fire in parallel. AI response time up to the animation duration is effectively free.

```
Turn N resolve -> Start animation + request Turn N+1 from AIs
                  Animation finishes -> Turn N+1 resolve -> ...
```

## Rendering & Visual Layout

```
+----------------------------------+-------------------+
|                                  |   SHIP 1 (cyan)   |
|                                  |   HP: ====-- 85   |
|            ARENA                 |   EN: ==---- 60   |
|     (half-block canvas)          |   Shield: OFF     |
|                                  |   Cooldowns: y/3  |
|                                  |                   |
|                                  +-------------------+
|                                  |   SHIP 2 (magenta)|
|                                  |   HP: ====== 100  |
|                                  |   EN: =====- 80   |
|                                  |   Shield: OFF     |
|                                  |   Cooldowns: y/y  |
|                                  +-------------------+
|                                  |   Turn: 42        |
|                                  |   Timer: 1.2s     |
+----------------------------------+-------------------+
| Ship 1 fires primary . Ship 2 activates shield .    |
| Ship 1 hit! -15 HP . Ship 2 turns hard left .       |
+------------------------------------------------------+
```

- **Arena (left, ~70-75% width, full height):** Half-block pixels with true color. Ships as small multi-cell sprites with rotation frames. Projectiles as bright colored dots/streaks. Sensor range as subtle circle overlay. Arena boundary as dim border/gradient.
- **HUD (right sidebar):** Ship 1 stats, Ship 2 stats, turn/timer info stacked vertically.
- **Marquee (bottom, full width, 2-3 rows):** Scrolling event ticker, color-coded by ship. Newest events appear right, scroll left and fade.

### Visual Effects (tachyonfx)

- Weapon fire: bright flash + glow at muzzle
- Hit: color burst dissolve at impact
- Shield: shimmer overlay when active
- Destruction: multi-frame dissolve/explosion
- Low health: ship flickers/pulses red

### Color Scheme

Dark background, cyan vs magenta for ships, bright white/yellow projectiles, red damage, blue shield glow.

## Match Flow & CLI

```bash
latentspace --ship1 "prompts/aggressive.txt" --ship2 "prompts/defensive.txt"
```

### Flags

- `--turns 200` — max turns (default 200)
- `--arena 800x400` — arena dimensions
- `--speed fast|normal|slow` — animation speed
- `--api-key` / `ANTHROPIC_API_KEY` env var

### Lifecycle

1. Intro screen — match title, ship names from filenames, countdown
2. Game loop — turns resolve, arena renders, marquee scrolls
3. End screen — winner, final stats (damage dealt, shots fired, accuracy, turns survived), explosion on loser

No interactive controls during match. Spectator only. Ctrl+C to quit.

## Project Structure

```
latentspace/
  Cargo.toml
  src/
    main.rs              # CLI parsing, match setup, main loop
    game/
      mod.rs
      arena.rs           # Arena bounds, boundary rules
      ship.rs            # Ship state, physics, energy
      projectile.rs      # Projectile movement, lifetime
      combat.rs          # Damage resolution, collisions
      fog.rs             # Sensor range, visibility checks
    ai/
      mod.rs
      client.rs          # Anthropic API calls (async)
      protocol.rs        # Game state <-> JSON, JSON <-> command
      history.rs         # Conversation sliding window
    canvas/
      mod.rs
      renderer.rs        # Half-block pixel buffer, drawing primitives
      sprites.rs         # Ship sprites, projectile visuals
      effects.rs         # tachyonfx effect wrappers
    ui/
      mod.rs
      layout.rs          # Arena + sidebar + marquee layout
      hud.rs             # HP/energy bars, turn info
      marquee.rs         # Scrolling event ticker
  prompts/
    aggressive.txt       # Example prompt
    defensive.txt        # Example prompt
```

### Dependencies

| Crate | Purpose |
|---|---|
| ratatui + crossterm | TUI framework + terminal backend |
| tachyonfx | Visual effects |
| tokio | Async runtime for parallel AI calls |
| reqwest | HTTP client for Anthropic API |
| serde / serde_json | JSON serialization |
| clap | CLI argument parsing |

## Future Considerations (not in scope)

- Extract canvas module to standalone crate
- Replay system (record turn data for playback)
- Multi-ship matches (2v2, free-for-all)
- Provider-agnostic AI interface
- Tournament mode
