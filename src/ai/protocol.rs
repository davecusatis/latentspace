use serde::{Deserialize, Serialize};
use crate::game::Vec2;
use crate::game::fog;
use crate::game::projectile::Projectile;
use crate::game::ship::Ship;

// -- Types sent TO the AI --

#[derive(Debug, Serialize)]
pub struct GameStateMessage {
    pub turn: i32,
    #[serde(rename = "self")]
    pub self_ship: SelfShipView,
    pub enemy: Option<EnemyShipView>,
    pub detected_projectiles: Vec<ProjectileView>,
    pub arena: ArenaView,
    pub sensor_range: f64,
    pub detected_by_enemy: bool,
}

#[derive(Debug, Serialize)]
pub struct SelfShipView {
    pub position: Vec2,
    pub velocity: Vec2,
    pub heading: f64,
    pub health: i32,
    pub energy: i32,
    pub shield_active: bool,
    pub weapon_cooldowns: WeaponCooldowns,
}

#[derive(Debug, Serialize)]
pub struct WeaponCooldowns {
    pub primary: i32,
    pub secondary: i32,
}

#[derive(Debug, Serialize)]
pub struct EnemyShipView {
    pub position: Vec2,
    pub velocity: Vec2,
    pub heading: f64,
}

#[derive(Debug, Serialize)]
pub struct ProjectileView {
    pub position: Vec2,
    pub velocity: Vec2,
}

#[derive(Debug, Serialize)]
pub struct ArenaView {
    pub width: f64,
    pub height: f64,
}

// -- Types received FROM the AI --

#[derive(Debug, Deserialize, Clone)]
pub struct ShipCommand {
    #[serde(default)]
    pub thrust: f64,
    #[serde(default)]
    pub turn: f64,
    #[serde(default)]
    pub fire_primary: bool,
    #[serde(default)]
    pub fire_secondary: bool,
    #[serde(default)]
    pub shield: bool,
}

impl Default for ShipCommand {
    fn default() -> Self {
        Self {
            thrust: 0.0,
            turn: 0.0,
            fire_primary: false,
            fire_secondary: false,
            shield: false,
        }
    }
}

// -- Conversion functions --

pub fn build_game_state(
    turn: i32,
    observer_idx: usize,
    ships: &[Ship; 2],
    projectiles: &[Projectile],
    arena_width: f64,
    arena_height: f64,
) -> GameStateMessage {
    let observer = &ships[observer_idx];
    let opponent = &ships[1 - observer_idx];

    let detected_by_enemy = fog::is_visible(opponent, observer);

    let enemy = if fog::is_visible(observer, opponent) {
        Some(EnemyShipView {
            position: opponent.position,
            velocity: opponent.velocity,
            heading: opponent.heading,
        })
    } else {
        None
    };

    let visible_projs = fog::visible_projectiles(observer, projectiles);
    let detected_projectiles = visible_projs
        .iter()
        .map(|p| ProjectileView {
            position: p.position,
            velocity: p.velocity,
        })
        .collect();

    GameStateMessage {
        turn,
        self_ship: SelfShipView {
            position: observer.position,
            velocity: observer.velocity,
            heading: observer.heading,
            health: observer.health,
            energy: observer.energy,
            shield_active: observer.shield_active,
            weapon_cooldowns: WeaponCooldowns {
                primary: observer.primary_cooldown,
                secondary: observer.secondary_cooldown,
            },
        },
        enemy,
        detected_projectiles,
        arena: ArenaView {
            width: arena_width,
            height: arena_height,
        },
        sensor_range: crate::game::ship::SENSOR_RANGE,
        detected_by_enemy,
    }
}

/// Parse the AI response text, extracting JSON from possible markdown code blocks.
pub fn parse_command(response: &str) -> Result<ShipCommand, String> {
    // Try to extract JSON from markdown code block first
    let json_str = if let Some(start) = response.find("```") {
        let after_ticks = &response[start + 3..];
        // Skip optional language tag (e.g., "json")
        let content_start = after_ticks.find('\n').unwrap_or(0) + 1;
        let content = &after_ticks[content_start..];
        if let Some(end) = content.find("```") {
            content[..end].trim()
        } else {
            response.trim()
        }
    } else if let Some(start) = response.find('{') {
        let end = response.rfind('}').unwrap_or(response.len());
        &response[start..=end]
    } else {
        response.trim()
    };

    serde_json::from_str(json_str).map_err(|e| format!("Failed to parse command: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_raw_json() {
        let input = r#"{"thrust": 0.5, "turn": 10.0, "fire_primary": true, "fire_secondary": false, "shield": false}"#;
        let cmd = parse_command(input).unwrap();
        assert!((cmd.thrust - 0.5).abs() < 1e-10);
        assert!(cmd.fire_primary);
    }

    #[test]
    fn parse_json_in_code_block() {
        let input = "Here's my move:\n```json\n{\"thrust\": 1.0, \"turn\": -15.0, \"fire_primary\": false, \"fire_secondary\": true, \"shield\": true}\n```";
        let cmd = parse_command(input).unwrap();
        assert!((cmd.thrust - 1.0).abs() < 1e-10);
        assert!(cmd.fire_secondary);
        assert!(cmd.shield);
    }

    #[test]
    fn parse_missing_fields_uses_defaults() {
        let input = r#"{"thrust": 0.8}"#;
        let cmd = parse_command(input).unwrap();
        assert!((cmd.thrust - 0.8).abs() < 1e-10);
        assert!(!cmd.fire_primary);
        assert!((cmd.turn - 0.0).abs() < 1e-10);
    }

    #[test]
    fn game_state_hides_enemy_out_of_range() {
        let ships = [
            Ship::new(Vec2::new(0.0, 0.0), 0.0),
            Ship::new(Vec2::new(500.0, 500.0), 0.0),
        ];
        let state = build_game_state(1, 0, &ships, &[], 800.0, 400.0);
        assert!(state.enemy.is_none());
        assert!(!state.detected_by_enemy);
    }

    #[test]
    fn game_state_shows_enemy_in_range() {
        let ships = [
            Ship::new(Vec2::new(100.0, 100.0), 0.0),
            Ship::new(Vec2::new(200.0, 100.0), 90.0),
        ];
        let state = build_game_state(1, 0, &ships, &[], 800.0, 400.0);
        assert!(state.enemy.is_some());
        assert!(state.detected_by_enemy);
        let enemy = state.enemy.unwrap();
        assert!((enemy.heading - 90.0).abs() < 1e-10);
    }

    #[test]
    fn game_state_serializes_to_expected_json() {
        let ships = [
            Ship::new(Vec2::new(100.0, 100.0), 45.0),
            Ship::new(Vec2::new(500.0, 500.0), 0.0),
        ];
        let state = build_game_state(1, 0, &ships, &[], 800.0, 400.0);
        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(json["turn"], 1);
        assert!(json.get("self").is_some());
        assert!(json["enemy"].is_null());
        assert_eq!(json["detected_by_enemy"], false);
    }

    #[test]
    fn detected_by_enemy_true_when_in_range() {
        let ships = [
            Ship::new(Vec2::new(100.0, 100.0), 0.0),
            Ship::new(Vec2::new(200.0, 100.0), 0.0),
        ];
        let state0 = build_game_state(1, 0, &ships, &[], 800.0, 400.0);
        let state1 = build_game_state(1, 1, &ships, &[], 800.0, 400.0);
        assert!(state0.detected_by_enemy);
        assert!(state1.detected_by_enemy);
    }

    #[test]
    fn detected_by_enemy_false_when_far_apart() {
        let ships = [
            Ship::new(Vec2::new(0.0, 0.0), 0.0),
            Ship::new(Vec2::new(500.0, 500.0), 0.0),
        ];
        let state0 = build_game_state(1, 0, &ships, &[], 800.0, 400.0);
        let state1 = build_game_state(1, 1, &ships, &[], 800.0, 400.0);
        assert!(!state0.detected_by_enemy);
        assert!(!state1.detected_by_enemy);
    }
}
