use mlua::prelude::*;
use mlua::{LuaSerdeExt, SerializeOptions};
use super::protocol::{GameStateMessage, ShipCommand};

pub struct ScriptAgent {
    lua: Lua,
}

impl ScriptAgent {
    /// Load a Lua script from a file path.
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let source = std::fs::read_to_string(path)?;
        Self::from_source(&source)
    }

    /// Load a Lua script from a source string.
    pub fn from_source(source: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let lua = Lua::new();

        // Sandbox: remove dangerous globals
        lua.globals().set("io", LuaValue::Nil)?;
        lua.globals().set("os", LuaValue::Nil)?;
        lua.globals().set("debug", LuaValue::Nil)?;
        lua.globals().set("loadfile", LuaValue::Nil)?;
        lua.globals().set("dofile", LuaValue::Nil)?;
        lua.globals().set("require", LuaValue::Nil)?;

        // Set instruction limit to prevent infinite loops (~100,000 instructions)
        lua.set_hook(
            mlua::HookTriggers::new().every_nth_instruction(100_000),
            |_lua, _debug| {
                Err(mlua::Error::RuntimeError(
                    "instruction limit exceeded".to_string(),
                ))
            },
        );

        // Create persistent memory table
        lua.globals().set("__memory", lua.create_table()?)?;

        // Load the user script
        lua.load(source).exec()?;

        Ok(Self { lua })
    }

    /// Call the Lua `think(state, memory)` function and return a ShipCommand.
    /// On any error, returns `ShipCommand::default()`.
    pub fn get_command(&self, state: &GameStateMessage) -> ShipCommand {
        self.try_get_command(state).unwrap_or_default()
    }

    fn try_get_command(&self, state: &GameStateMessage) -> Result<ShipCommand, Box<dyn std::error::Error>> {
        let think: LuaFunction = self.lua.globals().get("think")?;

        // Serialize GameStateMessage to a Lua value via mlua's serde support.
        // Use serialize_none_to_null=false so Option::None becomes Lua nil (not a userdata).
        let options = SerializeOptions::new().serialize_none_to_null(false);
        let state_value = self.lua.to_value_with(state, options)?;

        // Get the persistent memory table
        let memory: LuaTable = self.lua.globals().get("__memory")?;

        // Call think(state, memory)
        let result: LuaValue = think.call((state_value, memory))?;

        // Parse the returned table into a ShipCommand
        let command = self.parse_command_table(result)?;
        Ok(command)
    }

    fn parse_command_table(&self, value: LuaValue) -> Result<ShipCommand, Box<dyn std::error::Error>> {
        match value {
            LuaValue::Table(table) => {
                let thrust: f64 = table.get("thrust").unwrap_or(0.0);
                let turn: f64 = table.get("turn").unwrap_or(0.0);
                let fire_primary: bool = table.get("fire_primary").unwrap_or(false);
                let fire_secondary: bool = table.get("fire_secondary").unwrap_or(false);
                let shield: bool = table.get("shield").unwrap_or(false);

                Ok(ShipCommand {
                    thrust,
                    turn,
                    fire_primary,
                    fire_secondary,
                    shield,
                })
            }
            _ => Ok(ShipCommand::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Vec2;
    use super::super::protocol::{
        SelfShipView, EnemyShipView, WeaponCooldowns, ArenaView,
    };

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

    #[test]
    fn basic_think_returns_command() {
        let agent = ScriptAgent::from_source(r#"
            function think(state, memory)
                return {
                    thrust = 1.0,
                    turn = 15.5,
                    fire_primary = true,
                    fire_secondary = false,
                    shield = true
                }
            end
        "#).unwrap();

        let cmd = agent.get_command(&mock_state(true));
        assert!((cmd.thrust - 1.0).abs() < 1e-10);
        assert!((cmd.turn - 15.5).abs() < 1e-10);
        assert!(cmd.fire_primary);
        assert!(!cmd.fire_secondary);
        assert!(cmd.shield);
    }

    #[test]
    fn missing_fields_default_to_zero() {
        let agent = ScriptAgent::from_source(r#"
            function think(state, memory)
                return { thrust = 0.7, fire_primary = true }
            end
        "#).unwrap();

        let cmd = agent.get_command(&mock_state(true));
        assert!((cmd.thrust - 0.7).abs() < 1e-10);
        assert!((cmd.turn - 0.0).abs() < 1e-10);
        assert!(cmd.fire_primary);
        assert!(!cmd.fire_secondary);
        assert!(!cmd.shield);
    }

    #[test]
    fn empty_return_gives_drift() {
        let agent = ScriptAgent::from_source(r#"
            function think(state, memory)
                return {}
            end
        "#).unwrap();

        let cmd = agent.get_command(&mock_state(false));
        assert!((cmd.thrust - 0.0).abs() < 1e-10);
        assert!((cmd.turn - 0.0).abs() < 1e-10);
        assert!(!cmd.fire_primary);
        assert!(!cmd.fire_secondary);
        assert!(!cmd.shield);
    }

    #[test]
    fn memory_persists_across_calls() {
        let agent = ScriptAgent::from_source(r#"
            function think(state, memory)
                memory.count = (memory.count or 0) + 1
                return { thrust = memory.count }
            end
        "#).unwrap();

        let state = mock_state(false);
        let cmd1 = agent.get_command(&state);
        let cmd2 = agent.get_command(&state);
        let cmd3 = agent.get_command(&state);

        assert!((cmd1.thrust - 1.0).abs() < 1e-10);
        assert!((cmd2.thrust - 2.0).abs() < 1e-10);
        assert!((cmd3.thrust - 3.0).abs() < 1e-10);
    }

    #[test]
    fn handles_nil_enemy() {
        let agent = ScriptAgent::from_source(r#"
            function think(state, memory)
                if state.enemy then
                    return { thrust = 1.0, turn = state.enemy.bearing }
                else
                    return { thrust = 0.5 }
                end
            end
        "#).unwrap();

        let cmd = agent.get_command(&mock_state(false));
        assert!((cmd.thrust - 0.5).abs() < 1e-10);
        assert!((cmd.turn - 0.0).abs() < 1e-10);

        let cmd_with_enemy = agent.get_command(&mock_state(true));
        assert!((cmd_with_enemy.thrust - 1.0).abs() < 1e-10);
        assert!((cmd_with_enemy.turn - 30.0).abs() < 1e-10);
    }

    #[test]
    fn reads_state_fields_correctly() {
        // self_ship is now serialized as "self_ship" so Lua can use dot access
        let agent = ScriptAgent::from_source(r#"
            function think(state, memory)
                local me = state.self_ship
                return {
                    thrust = me.energy / 100.0,
                    turn = me.heading
                }
            end
        "#).unwrap();

        let cmd = agent.get_command(&mock_state(true));
        // energy is 70, so thrust = 70/100 = 0.7
        assert!((cmd.thrust - 0.7).abs() < 1e-10);
        // heading is 45.0
        assert!((cmd.turn - 45.0).abs() < 1e-10);
    }

    #[test]
    fn sandbox_blocks_io() {
        let agent = ScriptAgent::from_source(r#"
            function think(state, memory)
                io.write("hacked!")
                return { thrust = 1.0 }
            end
        "#).unwrap();

        let cmd = agent.get_command(&mock_state(false));
        // io.write should error, so we get default command
        assert!((cmd.thrust - 0.0).abs() < 1e-10);
        assert!((cmd.turn - 0.0).abs() < 1e-10);
        assert!(!cmd.fire_primary);
        assert!(!cmd.fire_secondary);
        assert!(!cmd.shield);
    }

    #[test]
    fn error_in_think_returns_default() {
        let agent = ScriptAgent::from_source(r#"
            function think(state, memory)
                error("something went wrong!")
                return { thrust = 1.0 }
            end
        "#).unwrap();

        let cmd = agent.get_command(&mock_state(false));
        assert!((cmd.thrust - 0.0).abs() < 1e-10);
        assert!((cmd.turn - 0.0).abs() < 1e-10);
        assert!(!cmd.fire_primary);
        assert!(!cmd.fire_secondary);
        assert!(!cmd.shield);
    }
}
