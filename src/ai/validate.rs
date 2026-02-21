use mlua::{Lua, Function, LuaSerdeExt, Value};
use mlua::SerializeOptions;
use super::protocol::{
    GameStateMessage, SelfShipView, EnemyShipView, WeaponCooldowns, ArenaView,
};
use crate::game::Vec2;

pub struct ValidationResult {
    pub checks: Vec<Check>,
}

pub struct Check {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
    pub command_output: Option<CommandOutput>,
}

#[derive(Debug)]
pub struct CommandOutput {
    pub thrust: f64,
    pub turn: f64,
    pub fire_primary: bool,
    pub fire_secondary: bool,
    pub shield: bool,
}

impl ValidationResult {
    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }

    pub fn passed_count(&self) -> usize {
        self.checks.iter().filter(|c| c.passed).count()
    }

    pub fn total_count(&self) -> usize {
        self.checks.len()
    }
}

fn mock_state(with_enemy: bool) -> GameStateMessage {
    GameStateMessage {
        turn: 5,
        self_ship: SelfShipView {
            position: Vec2::new(100.0, 200.0),
            velocity: Vec2::new(5.0, -3.0),
            heading: 45.0,
            health: 85,
            energy: 70,
            shield_active: false,
            weapon_cooldowns: WeaponCooldowns { primary: 0, secondary: 3 },
        },
        enemy: if with_enemy {
            Some(EnemyShipView {
                position: Vec2::new(300.0, 150.0),
                velocity: Vec2::new(-2.0, 1.0),
                heading: 180.0,
                distance: 120.5,
                bearing: 30.0,
                turn_to_aim: 15.0,
                lead_turn_to_aim: 18.5,
                closing_speed: 7.0,
            })
        } else {
            None
        },
        detected_projectiles: vec![],
        arena: ArenaView { width: 800.0, height: 600.0 },
        sensor_range: 150.0,
        detected_by_enemy: false,
    }
}

fn extract_command_output(_lua: &Lua, result: &Value) -> Result<CommandOutput, String> {
    match result {
        Value::Table(table) => {
            let thrust: f64 = table.get("thrust").unwrap_or(0.0);
            let turn: f64 = table.get("turn").unwrap_or(0.0);
            let fire_primary: bool = table.get("fire_primary").unwrap_or(false);
            let fire_secondary: bool = table.get("fire_secondary").unwrap_or(false);
            let shield: bool = table.get("shield").unwrap_or(false);
            Ok(CommandOutput {
                thrust,
                turn,
                fire_primary,
                fire_secondary,
                shield,
            })
        }
        _ => Err(format!("think() returned {:?} instead of a table", result)),
    }
}

fn call_think(lua: &Lua, state: &GameStateMessage) -> Result<(Value, CommandOutput), String> {
    let think: Function = lua
        .globals()
        .get("think")
        .map_err(|e| format!("{e}"))?;

    let options = SerializeOptions::new().serialize_none_to_null(false);
    let state_value = lua
        .to_value_with(state, options)
        .map_err(|e| format!("failed to serialize state: {e}"))?;

    let result: Value = think
        .call(state_value)
        .map_err(|e| format!("{e}"))?;

    let output = extract_command_output(lua, &result)?;
    Ok((result, output))
}

pub fn validate_source(source: &str) -> ValidationResult {
    let mut checks: Vec<Check> = Vec::new();
    let lua = Lua::new();

    // Serialize None as nil (not null userdata)
    // No sandbox restrictions for validation

    // Check 1: Lua compiles
    match lua.load(source).exec() {
        Ok(_) => {
            checks.push(Check {
                name: "Lua compiles".to_string(),
                passed: true,
                error: None,
                command_output: None,
            });
        }
        Err(e) => {
            checks.push(Check {
                name: "Lua compiles".to_string(),
                passed: false,
                error: Some(format!("{e}")),
                command_output: None,
            });
            return ValidationResult { checks };
        }
    }

    // Check 2: think() function exists
    match lua.globals().get::<Function>("think") {
        Ok(_) => {
            checks.push(Check {
                name: "think() function exists".to_string(),
                passed: true,
                error: None,
                command_output: None,
            });
        }
        Err(e) => {
            checks.push(Check {
                name: "think() function exists".to_string(),
                passed: false,
                error: Some(format!("{e}")),
                command_output: None,
            });
            return ValidationResult { checks };
        }
    }

    // Check 3: think() runs with enemy visible
    let check3_output = match call_think(&lua, &mock_state(true)) {
        Ok((_val, output)) => {
            checks.push(Check {
                name: "think() runs with enemy visible".to_string(),
                passed: true,
                error: None,
                command_output: Some(output),
            });
            // Re-call to get a fresh output for check 4 validation
            // Actually, let's just grab the values from the check we just pushed
            let last = checks.last().unwrap();
            last.command_output.as_ref().map(|o| (o.thrust, o.turn))
        }
        Err(e) => {
            checks.push(Check {
                name: "think() runs with enemy visible".to_string(),
                passed: false,
                error: Some(e),
                command_output: None,
            });
            None
        }
    };

    // Check 4: Return value is valid ShipCommand (thrust in 0..=1, turn in -30..=30)
    match check3_output {
        Some((thrust, turn)) => {
            let mut errors = Vec::new();
            if thrust < 0.0 || thrust > 1.0 {
                errors.push(format!("thrust {} is outside 0.0..=1.0", thrust));
            }
            if turn < -30.0 || turn > 30.0 {
                errors.push(format!("turn {} is outside -30.0..=30.0", turn));
            }
            if errors.is_empty() {
                checks.push(Check {
                    name: "Return value is valid ShipCommand".to_string(),
                    passed: true,
                    error: None,
                    command_output: None,
                });
            } else {
                checks.push(Check {
                    name: "Return value is valid ShipCommand".to_string(),
                    passed: false,
                    error: Some(errors.join("; ")),
                    command_output: None,
                });
            }
        }
        None => {
            checks.push(Check {
                name: "Return value is valid ShipCommand".to_string(),
                passed: false,
                error: Some("skipped: think() did not return a valid result".to_string()),
                command_output: None,
            });
        }
    }

    // Check 5: think() runs with enemy nil (fog of war)
    match call_think(&lua, &mock_state(false)) {
        Ok((_val, output)) => {
            checks.push(Check {
                name: "think() runs with enemy nil (fog of war)".to_string(),
                passed: true,
                error: None,
                command_output: Some(output),
            });
        }
        Err(e) => {
            checks.push(Check {
                name: "think() runs with enemy nil (fog of war)".to_string(),
                passed: false,
                error: Some(e),
                command_output: None,
            });
        }
    }

    ValidationResult { checks }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_script_passes_all_checks() {
        let source = r#"
            function think(state, memory)
                if state.enemy then
                    return {
                        thrust = 1.0,
                        turn = state.enemy.turn_to_aim,
                        fire_primary = true,
                        fire_secondary = false,
                        shield = false
                    }
                else
                    return {
                        thrust = 0.5,
                        turn = 0.0,
                        fire_primary = false,
                        fire_secondary = false,
                        shield = true
                    }
                end
            end
        "#;
        let result = validate_source(source);
        assert!(result.all_passed(), "Expected all checks to pass, but {}/{} passed. Failures: {:?}",
            result.passed_count(), result.total_count(),
            result.checks.iter()
                .filter(|c| !c.passed)
                .map(|c| format!("{}: {}", c.name, c.error.as_deref().unwrap_or("?")))
                .collect::<Vec<_>>()
        );
        assert_eq!(result.total_count(), 5);
        assert_eq!(result.passed_count(), 5);
    }

    #[test]
    fn syntax_error_fails_compile_check() {
        let source = r#"function think(state this is broken"#;
        let result = validate_source(source);
        assert_eq!(result.passed_count(), 0);
        assert_eq!(result.total_count(), 1); // early return after compile failure
        assert!(!result.checks[0].passed);
        assert!(result.checks[0].error.is_some());
    }

    #[test]
    fn missing_think_fails_function_check() {
        let source = r#"
            function not_think(state, memory)
                return { thrust = 1.0 }
            end
        "#;
        let result = validate_source(source);
        assert_eq!(result.total_count(), 2);
        assert!(result.checks[0].passed, "Compile check should pass");
        assert!(!result.checks[1].passed, "think() exists check should fail");
        assert_eq!(result.passed_count(), 1);
    }

    #[test]
    fn nil_enemy_crash_fails_fog_check() {
        // Script accesses state.enemy.distance directly without nil check
        let source = r#"
            function think(state, memory)
                return {
                    thrust = 0.5,
                    turn = state.enemy.distance * 0.1,
                    fire_primary = true,
                    fire_secondary = false,
                    shield = false
                }
            end
        "#;
        let result = validate_source(source);
        assert_eq!(result.total_count(), 5);
        assert!(result.checks[0].passed, "Compile check should pass");
        assert!(result.checks[1].passed, "think() exists check should pass");
        assert!(result.checks[2].passed, "think() with enemy should pass (enemy is present)");
        // Check 4 may or may not pass depending on the turn value: distance=120.5, turn=12.05 which is in range
        assert!(!result.checks[4].passed, "think() with nil enemy should fail");
        assert!(result.checks[4].error.is_some());
    }

    #[test]
    fn out_of_range_values_fail_type_check() {
        let source = r#"
            function think(state, memory)
                return {
                    thrust = 5.0,
                    turn = 100.0,
                    fire_primary = false,
                    fire_secondary = false,
                    shield = false
                }
            end
        "#;
        let result = validate_source(source);
        assert_eq!(result.total_count(), 5);
        assert!(result.checks[0].passed, "Compile check should pass");
        assert!(result.checks[1].passed, "think() exists check should pass");
        assert!(result.checks[2].passed, "think() with enemy should pass");
        assert!(!result.checks[3].passed, "Type check should fail for out-of-range values");
        assert!(result.checks[3].error.as_ref().unwrap().contains("thrust"));
        assert!(result.checks[3].error.as_ref().unwrap().contains("turn"));
    }
}
