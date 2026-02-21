pub mod ai;
pub mod canvas;
pub mod game;
pub mod ui;

use std::io;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use ai::protocol::{self, ShipCommand};
use ai::script::ScriptAgent;
use canvas::renderer::PixelCanvas;
use canvas::sprites::{self, Explosion, Viewport};
use game::combat::GameEvent;
use game::simulation::{GameState, MatchResult};
use ui::debug_overlay::DebugOverlay;
use ui::hud::{MatchInfo, ShipHud};
use ui::layout::AppLayout;
use ui::marquee::EventLog;
use ui::startup_overlay::StartupOverlay;

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

        /// Animation speed
        #[arg(long, default_value = "normal")]
        speed: String,
    },
    /// Validate a Lua script without running a match
    Validate {
        /// Path to the Lua script to validate
        script: String,
    },
}

fn parse_arena_size(s: &str) -> (f64, f64) {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() == 2 {
        let w = parts[0].parse().unwrap_or(800.0);
        let h = parts[1].parse().unwrap_or(400.0);
        (w, h)
    } else {
        (800.0, 400.0)
    }
}

fn interpolation_duration(speed: &str) -> Duration {
    match speed {
        "fast" => Duration::from_millis(150),
        "slow" => Duration::from_millis(1000),
        _ => Duration::from_millis(400),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate { script } => run_validate(&script),
        Command::Play {
            ship1,
            ship2,
            turns,
            arena,
            speed,
        } => run_play(&ship1, &ship2, turns, &arena, &speed),
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

fn run_play(
    ship1_path: &str,
    ship2_path: &str,
    turns: i32,
    arena_str: &str,
    speed: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (arena_w, arena_h) = parse_arena_size(arena_str);
    let interp_dur = interpolation_duration(speed);

    let agent1 = ScriptAgent::from_file(ship1_path)?;
    let agent2 = ScriptAgent::from_file(ship2_path)?;

    let mut game = GameState::new(arena_w, arena_h, turns);
    let mut event_log = EventLog::new(50);

    let ship1_name = std::path::Path::new(ship1_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Ship 1".to_string());
    let ship2_name = std::path::Path::new(ship2_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Ship 2".to_string());

    // Setup terminal
    terminal::enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run_game(
        &mut terminal,
        &mut game,
        &agent1,
        &agent2,
        &mut event_log,
        &ship1_name,
        &ship2_name,
        interp_dur,
    );

    // Restore terminal
    terminal::disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    match result {
        Ok(match_result) => {
            match match_result {
                MatchResult::Winner(i) => {
                    let name = if i == 0 { &ship1_name } else { &ship2_name };
                    println!("\n=== {} WINS! ===\n", name.to_uppercase());
                }
                MatchResult::Draw => println!("\n=== DRAW! ===\n"),
                MatchResult::InProgress => unreachable!(),
            }
            println!(
                "Ship 1 ({}): {} HP | {} shots | {} hits | {} damage dealt",
                ship1_name,
                game.ships[0].health,
                game.ships[0].shots_fired,
                game.ships[0].shots_hit,
                game.ships[0].damage_dealt
            );
            println!(
                "Ship 2 ({}): {} HP | {} shots | {} hits | {} damage dealt",
                ship2_name,
                game.ships[1].health,
                game.ships[1].shots_fired,
                game.ships[1].shots_hit,
                game.ships[1].damage_dealt
            );
        }
        Err(e) => eprintln!("Error: {e}"),
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
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
    let startup_overlay_start: Option<Instant> = Some(Instant::now());

    loop {
        // Snapshot previous state for interpolation
        let prev_ships = game.ships.clone();

        // Build game state for each ship
        let state1 = protocol::build_game_state(
            game.turn,
            0,
            &game.ships,
            &game.projectiles,
            game.arena.width,
            game.arena.height,
        );
        let state2 = protocol::build_game_state(
            game.turn,
            1,
            &game.ships,
            &game.projectiles,
            game.arena.width,
            game.arena.height,
        );

        // Call scripts (instant)
        let cmd1 = agent1.get_command(&state1);
        let cmd2 = agent2.get_command(&state2);
        last_commands = [cmd1.clone(), cmd2.clone()];

        // ===== Advance simulation =====
        game.advance([cmd1, cmd2]);
        event_log.push_game_events(&game.events);

        // Spawn explosions for hit/destroyed/ram events
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

        // ===== PHASE 2: Ship interpolation after advance =====
        let start = Instant::now();
        while start.elapsed() < interp_dur {
            let t = start.elapsed().as_secs_f64() / interp_dur.as_secs_f64();
            let t = t.min(1.0);

            // Check for Ctrl+C or debug toggle
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
                let layout = AppLayout::compute(frame.area());

                let pixel_w = (layout.arena.width as usize) * 2;
                let pixel_h = (layout.arena.height as usize) * 4;
                let mut canvas = PixelCanvas::new(pixel_w, pixel_h);
                let vp =
                    Viewport::new(game.arena.width, game.arena.height, pixel_w, pixel_h);

                sprites::draw_starfield(&mut canvas);
                sprites::draw_arena_border(&mut canvas);

                // Interpolate ships from prev to current positions
                for (i, prev_ship) in prev_ships.iter().enumerate() {
                    let mut interp_ship = game.ships[i].clone();
                    interp_ship.position =
                        prev_ship.position.lerp(game.ships[i].position, t);
                    sprites::draw_sensor_range(&mut canvas, &interp_ship, &vp);
                    sprites::draw_ship(&mut canvas, &interp_ship, i, &vp);
                    sprites::draw_shield(&mut canvas, &interp_ship, i, &vp);
                }

                // Projectiles at their post-advance positions
                for proj in &game.projectiles {
                    sprites::draw_projectile(&mut canvas, proj, &vp);
                }

                // Active explosions
                for explosion in &explosions {
                    sprites::draw_explosion(&mut canvas, explosion, &vp);
                }

                frame.render_widget(&canvas, layout.arena);

                // Startup overlay
                if startup_overlay_start.is_some() {
                    frame.render_widget(
                        StartupOverlay { progress: 0.0 },
                        layout.arena,
                    );
                }

                if debug_visible {
                    frame.render_widget(
                        DebugOverlay {
                            game,
                            commands: &last_commands,
                            ship_names: [ship1_name, ship2_name],
                        },
                        layout.arena,
                    );
                }

                // HUD
                frame.render_widget(
                    ShipHud {
                        ship: &game.ships[0],
                        name: ship1_name,
                        color: ratatui::style::Color::Cyan,
                    },
                    layout.ship1_hud,
                );
                frame.render_widget(
                    ShipHud {
                        ship: &game.ships[1],
                        name: ship2_name,
                        color: ratatui::style::Color::Magenta,
                    },
                    layout.ship2_hud,
                );
                frame.render_widget(
                    MatchInfo {
                        turn: game.turn,
                        max_turns: game.max_turns,
                    },
                    layout.match_info,
                );

                // Marquee
                frame.render_widget(event_log.widget(), layout.marquee);
            })?;

            // Tick explosions during Phase 2
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
