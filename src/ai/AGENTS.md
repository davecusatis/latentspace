# AI Module

Manages communication with Google Gemini API to get ship commands each turn.

## Files

- **client.rs** — `AiAgent` struct. Sends game state JSON, receives `ShipCommand`. Uses Gemini 2.5 Flash Lite with 5s timeout and 256 max tokens. Returns default drift command on any failure.
- **protocol.rs** — Builds `GameStateMessage` (fog-filtered per observer) and parses AI responses into `ShipCommand`. Supports raw JSON and markdown code blocks. Computes bearing, `turn_to_aim`, and `lead_turn_to_aim` for the AI.
- **history.rs** — `ConversationHistory` maintains up to 40 messages (20 turns). Roles are `"user"` and `"model"` (Gemini convention).

## Key Types

```rust
// Sent to AI (JSON)
GameStateMessage { turn, self_ship, enemy: Option<...>, detected_projectiles, arena, sensor_range, detected_by_enemy }

// Received from AI (parsed)
ShipCommand { thrust: 0.0–1.0, turn: -30..30, fire_primary, fire_secondary, shield }
```

## Conventions

- AI never sees enemies outside `SENSOR_RANGE` (150 units)
- Lead angle accounts for target velocity and projectile travel time
- Bearing is absolute angle (0=right, 90=down); `turn_to_aim` is signed delta from current heading
- Default command: all zeros (drift, no fire, no shield)
