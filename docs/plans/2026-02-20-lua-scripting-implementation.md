# Lua Scripting Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace Gemini LLM API calls with sandboxed Lua scripts for ship control, including a validation CLI for LLM feedback loops.

**Architecture:** Each ship runs a Lua VM (via `mlua` crate) that calls a `think(state, memory)` function per turn. The game loop becomes fully synchronous — no more async/tokio. A `validate` subcommand runs 5 checks against a script and prints structured pass/fail output.

**Tech Stack:** Rust, mlua (Lua 5.4 with serialize feature), clap subcommands

**Design doc:** `docs/plans/2026-02-20-lua-scripting-design.md`

---

### Task 1: Add mlua dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add mlua to Cargo.toml**

Add the mlua dependency with lua54 and serialize features. Keep all existing dependencies for now (they'll be removed in Task 7).

```toml
mlua = { version = "0.10", features = ["lua54", "serialize"] }
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add mlua dependency for Lua scripting"
```

---

### Task 2: Create ScriptAgent (TDD)

**Files:**
- Create: `src/ai/script.rs`
- Modify: `src/ai/mod.rs` (add `pub mod script;`)

**Step 1: Write failing test — basic think() call**

Create `src/ai/script.rs` with tests only:

```rust
use mlua::Lua;
use super::protocol::{GameStateMessage, ShipCommand, SelfShipView, WeaponCooldowns, ArenaView};
use crate::game::Vec2;

pub struct ScriptAgent {
    lua: Lua,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_state(with_enemy: bool) -> GameStateMessage {
        use super::super::protocol::EnemyShipView;
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
                return { thrust = 1.0, turn = 15.0, fire_primary = true }
            end
        "#).unwrap();
        let cmd = agent.get_command(&mock_state(true));
        assert!((cmd.thrust - 1.0).abs() < 1e-10);
        assert!((cmd.turn - 15.0).abs() < 1e-10);
        assert!(cmd.fire_primary);
        assert!(!cmd.fire_secondary);
        assert!(!cmd.shield);
    }
}
```

Add to `src/ai/mod.rs`:
```rust
pub mod client;
pub mod history;
pub mod protocol;
pub mod script;
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p latentspace script::tests::basic_think_returns_command`
Expected: FAIL — `ScriptAgent::from_source` not implemented

**Step 3: Implement ScriptAgent**

Complete the `ScriptAgent` implementation in `src/ai/script.rs`:

```rust
use mlua::{Lua, Function, LuaSerdeExt, Result as LuaResult, Value};
use super::protocol::{GameStateMessage, ShipCommand};

const INSTRUCTION_LIMIT: u32 = 100_000;

pub struct ScriptAgent {
    lua: Lua,
}

impl ScriptAgent {
    /// Load a script from a file path.
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let source = std::fs::read_to_string(path)?;
        Self::from_source(&source).map_err(|e| e.into())
    }

    /// Load a script from a source string.
    pub fn from_source(source: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let lua = Lua::new();

        // Sandbox: remove dangerous globals
        {
            let globals = lua.globals();
            globals.set("io", Value::Nil)?;
            globals.set("os", Value::Nil)?;
            globals.set("debug", Value::Nil)?;
            globals.set("loadfile", Value::Nil)?;
            globals.set("dofile", Value::Nil)?;
            globals.set("require", Value::Nil)?;
        }

        // Set instruction limit to prevent infinite loops
        lua.set_hook(
            mlua::HookTriggers::every_nth_instruction(INSTRUCTION_LIMIT),
            |_lua, _debug| {
                Err(mlua::Error::RuntimeError(
                    "instruction limit exceeded".to_string(),
                ))
            },
        );

        // Load the script
        lua.load(source).exec()?;

        // Initialize persistent memory table
        lua.load("__memory = {}").exec()?;

        Ok(Self { lua })
    }

    /// Call think() and return a ShipCommand. Returns default (drift) on error.
    pub fn get_command(&self, state: &GameStateMessage) -> ShipCommand {
        self.call_think(state).unwrap_or_default()
    }

    fn call_think(&self, state: &GameStateMessage) -> Result<ShipCommand, Box<dyn std::error::Error>> {
        let think: Function = self.lua.globals().get("think")?;
        let state_value = self.lua.to_value(state)?;
        let memory: Value = self.lua.globals().get("__memory")?;
        let result: mlua::Table = think.call((state_value, memory))?;

        Ok(ShipCommand {
            thrust: result.get("thrust").unwrap_or(0.0),
            turn: result.get("turn").unwrap_or(0.0),
            fire_primary: result.get("fire_primary").unwrap_or(false),
            fire_secondary: result.get("fire_secondary").unwrap_or(false),
            shield: result.get("shield").unwrap_or(false),
        })
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p latentspace script::tests::basic_think_returns_command`
Expected: PASS

**Step 5: Write and run additional tests**

Add these tests to the `mod tests` block:

```rust
    #[test]
    fn missing_fields_default_to_zero() {
        let agent = ScriptAgent::from_source(r#"
            function think(state, memory)
                return { thrust = 0.5 }
            end
        "#).unwrap();
        let cmd = agent.get_command(&mock_state(true));
        assert!((cmd.thrust - 0.5).abs() < 1e-10);
        assert!((cmd.turn).abs() < 1e-10);
        assert!(!cmd.fire_primary);
    }

    #[test]
    fn empty_return_gives_drift() {
        let agent = ScriptAgent::from_source(r#"
            function think(state, memory)
                return {}
            end
        "#).unwrap();
        let cmd = agent.get_command(&mock_state(true));
        assert!((cmd.thrust).abs() < 1e-10);
    }

    #[test]
    fn memory_persists_across_calls() {
        let agent = ScriptAgent::from_source(r#"
            function think(state, memory)
                memory.count = (memory.count or 0) + 1
                return { thrust = memory.count * 0.1 }
            end
        "#).unwrap();
        let state = mock_state(true);
        let cmd1 = agent.get_command(&state);
        let cmd2 = agent.get_command(&state);
        let cmd3 = agent.get_command(&state);
        assert!((cmd1.thrust - 0.1).abs() < 1e-10);
        assert!((cmd2.thrust - 0.2).abs() < 1e-10);
        assert!((cmd3.thrust - 0.3).abs() < 1e-10);
    }

    #[test]
    fn handles_nil_enemy() {
        let agent = ScriptAgent::from_source(r#"
            function think(state, memory)
                if state.enemy then
                    return { thrust = 1.0, turn = state.enemy.lead_turn_to_aim }
                end
                return { thrust = 0.5, turn = 15 }
            end
        "#).unwrap();
        let cmd = agent.get_command(&mock_state(false));
        assert!((cmd.thrust - 0.5).abs() < 1e-10);
        assert!((cmd.turn - 15.0).abs() < 1e-10);
    }

    #[test]
    fn reads_state_fields_correctly() {
        let agent = ScriptAgent::from_source(r#"
            function think(state, memory)
                return {
                    thrust = state.self_ship.energy / 100.0,
                    turn = state.self_ship.heading
                }
            end
        "#).unwrap();
        let cmd = agent.get_command(&mock_state(true));
        assert!((cmd.thrust - 0.7).abs() < 1e-10);  -- energy=70 / 100
        assert!((cmd.turn - 45.0).abs() < 1e-10);    -- heading=45
    }

    #[test]
    fn sandbox_blocks_io() {
        let result = ScriptAgent::from_source(r#"
            function think(state, memory)
                io.write("hack")
                return {}
            end
        "#);
        // Script loads fine (io is nil), but calling io.write will error
        let agent = result.unwrap();
        let cmd = agent.get_command(&mock_state(true));
        // Should return default because think() errored
        assert!((cmd.thrust).abs() < 1e-10);
    }

    #[test]
    fn error_in_think_returns_default() {
        let agent = ScriptAgent::from_source(r#"
            function think(state, memory)
                error("something went wrong")
            end
        "#).unwrap();
        let cmd = agent.get_command(&mock_state(true));
        assert!((cmd.thrust).abs() < 1e-10);
    }

    #[test]
    fn missing_think_function_fails_construction() {
        let result = ScriptAgent::from_source(r#"
            function not_think(state, memory)
                return {}
            end
        "#);
        // from_source should succeed (valid Lua), but we could add a check
        // For now, get_command will fail gracefully
        let agent = result.unwrap();
        let cmd = agent.get_command(&mock_state(true));
        assert!((cmd.thrust).abs() < 1e-10);
    }
```

Run: `cargo test -p latentspace script::tests`
Expected: All PASS

**Step 6: Commit**

```bash
git add src/ai/script.rs src/ai/mod.rs
git commit -m "feat: add ScriptAgent for Lua-based ship control"
```

---

### Task 3: Fix protocol serde rename for Lua compatibility

**Files:**
- Modify: `src/ai/protocol.rs`

The `GameStateMessage.self_ship` field uses `#[serde(rename = "self")]` which produces `state.self` in Lua — but `self` is a Lua keyword. Change it to `self_ship`.

**Step 1: Update the serde attribute**

In `src/ai/protocol.rs`, change:

```rust
    #[serde(rename = "self")]
    pub self_ship: SelfShipView,
```

to:

```rust
    #[serde(rename = "self_ship")]
    pub self_ship: SelfShipView,
```

**Step 2: Update the existing test**

In `src/ai/protocol.rs` test `game_state_serializes_to_expected_json`, change:

```rust
        assert!(json.get("self").is_some());
```

to:

```rust
        assert!(json.get("self_ship").is_some());
```

**Step 3: Verify tests pass**

Run: `cargo test -p latentspace protocol::tests`
Expected: All PASS

**Step 4: Commit**

```bash
git add src/ai/protocol.rs
git commit -m "fix: rename serde field from 'self' to 'self_ship' for Lua compatibility"
```

---

### Task 4: Create validation module (TDD)

**Files:**
- Create: `src/ai/validate.rs`
- Modify: `src/ai/mod.rs` (add `pub mod validate;`)

**Step 1: Write failing test — all checks pass for valid script**

Create `src/ai/validate.rs`:

```rust
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

pub fn validate_source(source: &str) -> ValidationResult {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_script_passes_all_checks() {
        let result = validate_source(r#"
            function think(state, memory)
                if state.enemy then
                    return { thrust = 1.0, turn = state.enemy.lead_turn_to_aim, fire_primary = true }
                end
                return { thrust = 0.5, turn = 15 }
            end
        "#);
        assert!(result.all_passed());
        assert_eq!(result.total_count(), 5);
    }
}
```

Add `pub mod validate;` to `src/ai/mod.rs`.

**Step 2: Run test to verify it fails**

Run: `cargo test -p latentspace validate::tests::valid_script_passes_all_checks`
Expected: FAIL — `todo!()` panics

**Step 3: Implement validate_source**

```rust
use mlua::{Lua, Function, LuaSerdeExt, Value};
use super::protocol::{
    GameStateMessage, SelfShipView, EnemyShipView, WeaponCooldowns,
    ArenaView, ProjectileView,
};
use crate::game::Vec2;

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

fn extract_command(table: &mlua::Table) -> CommandOutput {
    CommandOutput {
        thrust: table.get("thrust").unwrap_or(0.0),
        turn: table.get("turn").unwrap_or(0.0),
        fire_primary: table.get("fire_primary").unwrap_or(false),
        fire_secondary: table.get("fire_secondary").unwrap_or(false),
        shield: table.get("shield").unwrap_or(false),
    }
}

pub fn validate_source(source: &str) -> ValidationResult {
    let mut checks = Vec::new();

    // Check 1: Lua compiles
    let lua = Lua::new();
    match lua.load(source).exec() {
        Ok(()) => checks.push(Check {
            name: "Lua compiles".to_string(),
            passed: true,
            error: None,
            command_output: None,
        }),
        Err(e) => {
            checks.push(Check {
                name: "Lua compiles".to_string(),
                passed: false,
                error: Some(e.to_string()),
                command_output: None,
            });
            return ValidationResult { checks };
        }
    }

    // Check 2: think() function exists
    match lua.globals().get::<Function>("think") {
        Ok(_) => checks.push(Check {
            name: "think() function exists".to_string(),
            passed: true,
            error: None,
            command_output: None,
        }),
        Err(e) => {
            checks.push(Check {
                name: "think() function exists".to_string(),
                passed: false,
                error: Some(e.to_string()),
                command_output: None,
            });
            return ValidationResult { checks };
        }
    }

    // Initialize memory
    lua.load("__memory = {}").exec().unwrap();

    // Check 3: think() runs with enemy visible
    let think: Function = lua.globals().get("think").unwrap();
    let state_visible = lua.to_value(&mock_state(true)).unwrap();
    let memory: Value = lua.globals().get("__memory").unwrap();

    match think.call::<mlua::Table>((state_visible, memory.clone())) {
        Ok(table) => {
            let cmd = extract_command(&table);
            checks.push(Check {
                name: "think() runs with enemy visible".to_string(),
                passed: true,
                error: None,
                command_output: Some(cmd),
            });
        }
        Err(e) => {
            checks.push(Check {
                name: "think() runs with enemy visible".to_string(),
                passed: false,
                error: Some(e.to_string()),
                command_output: None,
            });
            // Still try remaining checks
        }
    }

    // Check 4: Return value has valid types
    // Re-run to check types (reuse last result if check 3 passed)
    let valid_types = if let Some(last) = checks.last() {
        if last.passed {
            if let Some(ref cmd) = last.command_output {
                let thrust_ok = (0.0..=1.0).contains(&cmd.thrust);
                let turn_ok = (-30.0..=30.0).contains(&cmd.turn);
                if !thrust_ok || !turn_ok {
                    let mut errs = Vec::new();
                    if !thrust_ok {
                        errs.push(format!("thrust {} not in 0.0..=1.0", cmd.thrust));
                    }
                    if !turn_ok {
                        errs.push(format!("turn {} not in -30.0..=30.0", cmd.turn));
                    }
                    Some((false, Some(errs.join(", "))))
                } else {
                    Some((true, None))
                }
            } else {
                Some((false, Some("no command output from previous check".to_string())))
            }
        } else {
            Some((false, Some("skipped: think() failed in previous check".to_string())))
        }
    } else {
        Some((false, Some("no previous check result".to_string())))
    };

    if let Some((passed, error)) = valid_types {
        checks.push(Check {
            name: "Return value is valid ShipCommand".to_string(),
            passed,
            error,
            command_output: None,
        });
    }

    // Check 5: think() runs with enemy nil (fog of war)
    // Reset memory for clean test
    lua.load("__memory = {}").exec().unwrap();
    let state_nil = lua.to_value(&mock_state(false)).unwrap();
    let memory: Value = lua.globals().get("__memory").unwrap();
    let think: Function = lua.globals().get("think").unwrap();

    match think.call::<mlua::Table>((state_nil, memory)) {
        Ok(table) => {
            let cmd = extract_command(&table);
            checks.push(Check {
                name: "think() runs with enemy nil (fog of war)".to_string(),
                passed: true,
                error: None,
                command_output: Some(cmd),
            });
        }
        Err(e) => {
            checks.push(Check {
                name: "think() runs with enemy nil (fog of war)".to_string(),
                passed: false,
                error: Some(e.to_string()),
                command_output: None,
            });
        }
    }

    ValidationResult { checks }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p latentspace validate::tests::valid_script_passes_all_checks`
Expected: PASS

**Step 5: Add more tests**

```rust
    #[test]
    fn syntax_error_fails_compile_check() {
        let result = validate_source("function think(");
        assert!(!result.all_passed());
        assert_eq!(result.passed_count(), 0);
        assert!(result.checks[0].error.as_ref().unwrap().contains("syntax"));
    }

    #[test]
    fn missing_think_fails_function_check() {
        let result = validate_source("function not_think() return {} end");
        assert!(!result.all_passed());
        assert!(result.checks[0].passed);  // compiles
        assert!(!result.checks[1].passed); // no think()
    }

    #[test]
    fn nil_enemy_crash_fails_fog_check() {
        let result = validate_source(r#"
            function think(state, memory)
                return { thrust = 1.0, turn = state.enemy.distance }
            end
        "#);
        // Compiles, think exists, but crashes when enemy is nil
        assert!(result.checks[0].passed);  // compiles
        assert!(result.checks[1].passed);  // think exists
        assert!(result.checks[2].passed);  // runs with enemy
        assert!(!result.checks[4].passed); // crashes with nil enemy
    }

    #[test]
    fn out_of_range_values_fail_type_check() {
        let result = validate_source(r#"
            function think(state, memory)
                return { thrust = 5.0, turn = 100 }
            end
        "#);
        assert!(result.checks[0].passed);  // compiles
        assert!(result.checks[1].passed);  // think exists
        assert!(result.checks[2].passed);  // runs
        assert!(!result.checks[3].passed); // values out of range
    }
```

Run: `cargo test -p latentspace validate::tests`
Expected: All PASS

**Step 6: Commit**

```bash
git add src/ai/validate.rs src/ai/mod.rs
git commit -m "feat: add script validation with 5-check harness"
```

---

### Task 5: Create example Lua scripts

**Files:**
- Create: `scripts/aggressive.lua`
- Create: `scripts/defensive.lua`

**Step 1: Create scripts directory and files**

`scripts/aggressive.lua`:
```lua
-- Aggressive Hunter: sprint toward enemy, fire everything
function think(state, memory)
    if state.enemy then
        return {
            thrust = 1.0,
            turn = state.enemy.lead_turn_to_aim,
            fire_primary = state.enemy.distance < 120,
            fire_secondary = state.enemy.distance < 80
        }
    end
    -- No enemy visible: spiral search
    return { thrust = 0.8, turn = 15 }
end
```

`scripts/defensive.lua`:
```lua
-- Defensive Kiter: maintain distance, orbit and fire, evade when close
function think(state, memory)
    memory.turns_since_seen = (memory.turns_since_seen or 0) + 1

    if state.enemy then
        memory.turns_since_seen = 0
        memory.last_enemy_pos = state.enemy.position

        if state.enemy.distance < 50 then
            -- Too close: evade
            return {
                thrust = 1.0,
                turn = state.enemy.turn_to_aim + 180,
                shield = state.self_ship.energy > 30
            }
        elseif state.enemy.distance < 100 then
            -- Ideal range: orbit and fire
            return {
                thrust = 0.6,
                turn = state.enemy.lead_turn_to_aim + 12,
                fire_primary = true,
                fire_secondary = state.self_ship.energy > 40
            }
        else
            -- Close the gap cautiously
            return { thrust = 0.7, turn = state.enemy.lead_turn_to_aim }
        end
    end

    -- Search pattern
    return { thrust = 0.6, turn = 20 }
end
```

**Step 2: Validate both scripts compile and pass**

Run: `cargo test -p latentspace` (integration tests from Task 6 will cover this, for now just check Lua syntax manually or via a quick unit test)

**Step 3: Commit**

```bash
git add scripts/
git commit -m "feat: add aggressive and defensive example Lua scripts"
```

---

### Task 6: Add integration tests for Lua scripts

**Files:**
- Create: `tests/lua_scripts.rs`

**Step 1: Write integration test that validates all scripts in scripts/**

```rust
use std::fs;
use latentspace::ai::validate;

#[test]
fn all_scripts_pass_validation() {
    let entries = fs::read_dir("scripts")
        .expect("scripts/ directory should exist");

    let mut script_count = 0;
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "lua") {
            script_count += 1;
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()));
            let result = validate::validate_source(&source);
            for check in &result.checks {
                assert!(
                    check.passed,
                    "Script {} failed check '{}': {}",
                    path.display(),
                    check.name,
                    check.error.as_deref().unwrap_or("unknown error")
                );
            }
        }
    }

    assert!(script_count >= 2, "Expected at least 2 scripts in scripts/, found {script_count}");
}
```

**Step 2: Run the integration test**

Run: `cargo test --test lua_scripts`
Expected: PASS (both scripts validate)

**Step 3: Commit**

```bash
git add tests/lua_scripts.rs
git commit -m "test: add integration tests validating all Lua scripts"
```

---

### Task 7: Rewrite main.rs — CLI subcommands and synchronous game loop

This is the big switchover. Replace `AiAgent` with `ScriptAgent`, add `validate` subcommand, remove async/tokio from the game loop.

**Files:**
- Modify: `src/main.rs`

**Step 1: Replace CLI struct with subcommands**

Change the `Cli` struct from flat args to subcommands:

```rust
#[derive(Parser)]
#[command(name = "latentspace")]
#[command(about = "AI Spaceship Deathmatch Arena")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Run a match between two Lua scripts
    Play {
        /// Path to Ship 1's Lua script
        #[arg(long)]
        ship1: String,

        /// Path to Ship 2's Lua script
        #[arg(long)]
        ship2: String,

        /// Maximum turns before timeout
        #[arg(long, default_value_t = 100)]
        turns: i32,

        /// Arena dimensions (WxH)
        #[arg(long, default_value = "600x300")]
        arena: String,

        /// Animation speed (fast, normal, slow)
        #[arg(long, default_value = "normal")]
        speed: String,
    },
    /// Validate a Lua script without running a match
    Validate {
        /// Path to the Lua script to validate
        script: String,
    },
}
```

**Step 2: Implement validate subcommand handler**

Add to main():

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Validate { script } => run_validate(&script),
        Command::Play { ship1, ship2, turns, arena, speed } => {
            run_play(&ship1, &ship2, turns, &arena, &speed)
        }
    }
}

fn run_validate(script_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(script_path)?;
    let result = ai::validate::validate_source(&source);

    println!("Validating {}...", script_path);
    for check in &result.checks {
        let icon = if check.passed { "PASS" } else { "FAIL" };
        println!("  [{}] {}", icon, check.name);
        if let Some(ref err) = check.error {
            println!("    Error: {}", err);
        }
        if let Some(ref cmd) = check.command_output {
            println!(
                "    Output: thrust={:.1}  turn={:.1}  fire_primary={}  fire_secondary={}  shield={}",
                cmd.thrust, cmd.turn, cmd.fire_primary, cmd.fire_secondary, cmd.shield
            );
        }
    }
    println!();
    println!(
        "  Result: {}/{} checks passed",
        result.passed_count(),
        result.total_count()
    );

    if result.all_passed() {
        Ok(())
    } else {
        std::process::exit(1);
    }
}
```

**Step 3: Rewrite run_play as synchronous with ScriptAgent**

Replace the entire `run_game` async function with a synchronous version. Key changes:
- `fn main()` instead of `#[tokio::main] async fn main()`
- `ScriptAgent::from_file()` instead of `AiAgent::new()`
- `agent.get_command(&state)` instead of `agent.get_command(&json).await`
- Remove Phase 1 entirely (no async AI polling)
- Keep Phase 2 interpolation loop using `std::thread::sleep` and `crossterm::event::poll`

The new game loop structure:

```rust
fn run_game(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    game: &mut GameState,
    agent1: &ScriptAgent,
    agent2: &ScriptAgent,
    event_log: &mut EventLog,
    ship1_name: &str,
    ship2_name: &str,
    interp_dur: Duration,
) -> Result<MatchResult, Box<dyn std::error::Error>> {
    let mut debug_visible = false;
    let mut last_commands = [ShipCommand::default(), ShipCommand::default()];
    let mut explosions: Vec<Explosion> = Vec::new();

    loop {
        // Snapshot previous state for interpolation
        let prev_ships = game.ships.clone();

        // Build game state for each ship
        let state1 = protocol::build_game_state(
            game.turn, 0, &game.ships, &game.projectiles,
            game.arena.width, game.arena.height,
        );
        let state2 = protocol::build_game_state(
            game.turn, 1, &game.ships, &game.projectiles,
            game.arena.width, game.arena.height,
        );

        // Call scripts (instant)
        let cmd1 = agent1.get_command(&state1);
        let cmd2 = agent2.get_command(&state2);
        last_commands = [cmd1.clone(), cmd2.clone()];

        // Advance simulation
        game.advance([cmd1, cmd2]);
        event_log.push_game_events(&game.events);

        // Spawn explosions
        for event in &game.events {
            match event {
                GameEvent::ShipHit { target, .. } => {
                    explosions.push(Explosion::hit(game.ships[*target].position));
                }
                GameEvent::ShipDestroyed(i) => {
                    explosions.push(Explosion::destroyed(game.ships[*i].position));
                }
                GameEvent::RamDamage { ship, .. } => {
                    explosions.push(Explosion::hit(game.ships[*ship].position));
                }
                _ => {}
            }
        }

        // Interpolated rendering
        let start = Instant::now();
        while start.elapsed() < interp_dur {
            let t = (start.elapsed().as_secs_f64() / interp_dur.as_secs_f64()).min(1.0);

            if event::poll(Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if key.code == KeyCode::Char('c')
                            && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                        {
                            return Ok(game.result());
                        }
                        if key.code == KeyCode::Char('d') {
                            debug_visible = !debug_visible;
                        }
                    }
                }
            }

            terminal.draw(|frame| {
                // ... same rendering code as current Phase 2 ...
                // Interpolate ships, draw projectiles, explosions, HUD, marquee
            })?;

            let frame_dt = 0.016;
            for explosion in &mut explosions {
                explosion.tick(frame_dt);
            }
            explosions.retain(|e| e.is_alive());
        }

        // Check end condition
        let result = game.result();
        if result != MatchResult::InProgress {
            return Ok(result);
        }
    }
}
```

**Step 4: Remove all tokio and reqwest imports**

Remove from `main.rs`:
- `use tokio::...` imports
- `#[tokio::main]` attribute
- All `.await` calls
- `use ai::client::AiAgent`

Replace with:
- `use std::thread`
- `use ai::script::ScriptAgent`

**Step 5: Remove api_key from CLI and AiAgent construction**

The `--api-key` / `GOOGLE_API_KEY` arg is no longer needed.

**Step 6: Verify it compiles and runs**

Run: `cargo build`
Expected: Compiles

Run: `cargo run -- validate scripts/aggressive.lua`
Expected: 5/5 checks passed

Run: `cargo run -- play --ship1 scripts/aggressive.lua --ship2 scripts/defensive.lua --turns 50`
Expected: Game runs with smooth rendering, instant turns

**Step 7: Commit**

```bash
git add src/main.rs
git commit -m "feat: rewrite game loop with Lua scripts, add validate subcommand"
```

---

### Task 8: Remove old AI code and unused dependencies

**Files:**
- Delete: `src/ai/client.rs`
- Delete: `src/ai/history.rs`
- Modify: `src/ai/mod.rs` (remove `pub mod client;` and `pub mod history;`)
- Modify: `Cargo.toml` (remove `reqwest` and `tokio` dependencies)
- Delete: `prompts/` directory (or keep for reference — user's choice)

**Step 1: Remove client.rs and history.rs**

Delete `src/ai/client.rs` and `src/ai/history.rs`.

Update `src/ai/mod.rs`:
```rust
pub mod protocol;
pub mod script;
pub mod validate;
```

**Step 2: Remove unused dependencies from Cargo.toml**

Remove these lines from `[dependencies]`:
```toml
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
```

**Step 3: Verify everything compiles and tests pass**

Run: `cargo test`
Expected: All tests pass, no compilation errors

**Step 4: Commit**

```bash
git add -A
git commit -m "refactor: remove Gemini AI client, history, and unused deps"
```

---

### Task 9: Update documentation

**Files:**
- Modify: `src/ai/AGENTS.md`
- Modify: `AGENTS.md` (top-level)

**Step 1: Update src/ai/AGENTS.md**

Replace the AI module documentation to describe the Lua scripting system instead of the Gemini integration. Cover:
- `script.rs`: ScriptAgent, Lua VM lifecycle, sandbox, memory
- `protocol.rs`: GameStateMessage building (unchanged), ShipCommand parsing (now from Lua tables)
- `validate.rs`: 5-check validation harness

**Step 2: Update top-level AGENTS.md**

Update the AI section to mention Lua scripts instead of Gemini. Update the CLI usage examples.

**Step 3: Commit**

```bash
git add AGENTS.md src/ai/AGENTS.md
git commit -m "docs: update AGENTS.md for Lua scripting architecture"
```
