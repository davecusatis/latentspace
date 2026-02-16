# Debug Overlay Design

## Summary

A toggleable debug overlay rendered on top of the arena canvas, activated by pressing `d`. Displays full telemetry for both ships, AI commands, targeting data, and match state.

## Interaction

- Press `d` to toggle the overlay on/off
- Overlay is off by default
- No other input changes — game continues running normally

## Visual Design

Dark-background panel in the top-left corner of the arena area. Text rendered into the ratatui Buffer over the arena widget using `Style::default().bg(Color::Black)` for readability.

## Panel Contents

```
[Ship 1: aggressive]          [Ship 2: defensive]
pos: (234.5, 178.2)           pos: (512.1, 89.7)
vel: (4.2, -1.8) spd: 4.6    vel: (-2.1, 3.4) spd: 4.0
hdg: 127.3  turn: 15.0       hdg: 302.1  turn: -8.0
hp: 80  nrg: 65  shld: OFF   hp: 95  nrg: 42  shld: ON
pri: RDY  sec: 3t             pri: 1t  sec: RDY
cmd: T=0.8 R=15.0 F1 --      cmd: T=0.4 R=-8.0 -- S
dist: 312.4  bearing: 45.2
projectiles: 3  turn: 42/100
```

## Architecture

- New module: `src/ui/debug_overlay.rs`
- New widget: `DebugOverlay` implementing ratatui `Widget`
- State: `debug_visible: bool` flag in the main game loop
- Last AI commands stored alongside game state for display
- Keybind: `KeyCode::Char('d')` added to both event-polling phases in `run_game`

## Data Sources

- Ship internals: `GameState.ships[i]` (position, velocity, heading, health, energy, shield, cooldowns)
- AI commands: captured from `agent.get_command()` return values each turn
- Targeting: distance and bearing computed from ship positions
- Match state: `GameState.turn`, `GameState.max_turns`, `GameState.projectiles.len()`
