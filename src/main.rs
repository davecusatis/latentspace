pub mod ai;
pub mod canvas;
pub mod game;
pub mod ui;

use std::io;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use ai::client::AiAgent;
use ai::protocol::{self, ShipCommand};
use canvas::renderer::PixelCanvas;
use canvas::sprites::{self, Viewport};
use game::simulation::{GameState, MatchResult};
use ui::hud::{MatchInfo, ShipHud};
use ui::layout::AppLayout;
use ui::marquee::EventLog;

#[derive(Parser)]
#[command(name = "latentspace")]
#[command(about = "AI Spaceship Deathmatch Arena")]
struct Cli {
    /// Path to Ship 1's prompt file
    #[arg(long)]
    ship1: String,

    /// Path to Ship 2's prompt file
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

    /// Anthropic API key (or set ANTHROPIC_API_KEY env var)
    #[arg(long, env = "ANTHROPIC_API_KEY")]
    api_key: String,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let prompt1 = std::fs::read_to_string(&cli.ship1)?;
    let prompt2 = std::fs::read_to_string(&cli.ship2)?;
    let (arena_w, arena_h) = parse_arena_size(&cli.arena);
    let interp_dur = interpolation_duration(&cli.speed);

    let mut agent1 = AiAgent::new(cli.api_key.clone(), prompt1);
    let mut agent2 = AiAgent::new(cli.api_key.clone(), prompt2);

    let mut game = GameState::new(arena_w, arena_h, cli.turns);
    let mut event_log = EventLog::new(50);

    let ship1_name = std::path::Path::new(&cli.ship1)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Ship 1".to_string());
    let ship2_name = std::path::Path::new(&cli.ship2)
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
        &mut agent1,
        &mut agent2,
        &mut event_log,
        &ship1_name,
        &ship2_name,
        interp_dur,
    )
    .await;

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
async fn run_game(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    game: &mut GameState,
    agent1: &mut AiAgent,
    agent2: &mut AiAgent,
    event_log: &mut EventLog,
    ship1_name: &str,
    ship2_name: &str,
    interp_dur: Duration,
) -> Result<MatchResult, Box<dyn std::error::Error>> {
    loop {
        // Snapshot previous state for interpolation
        let prev_ships = game.ships.clone();
        let prev_projectiles = game.projectiles.clone();

        // Build game state JSON for each AI
        let state1_json = serde_json::to_string(&protocol::build_game_state(
            game.turn,
            0,
            &game.ships,
            &game.projectiles,
            game.arena.width,
            game.arena.height,
        ))?;
        let state2_json = serde_json::to_string(&protocol::build_game_state(
            game.turn,
            1,
            &game.ships,
            &game.projectiles,
            game.arena.width,
            game.arena.height,
        ))?;

        // ===== PHASE 1: Render continuously while waiting for AI responses =====
        let ai_start = Instant::now();

        let cmd1_future = agent1.get_command(&state1_json);
        let cmd2_future = agent2.get_command(&state2_json);
        tokio::pin!(cmd1_future);
        tokio::pin!(cmd2_future);

        let mut cmd1_result: Option<ShipCommand> = None;
        let mut cmd2_result: Option<ShipCommand> = None;

        while cmd1_result.is_none() || cmd2_result.is_none() {
            // Poll AI futures with a short timeout for frame rendering
            tokio::select! {
                biased;
                cmd = &mut cmd1_future, if cmd1_result.is_none() => {
                    cmd1_result = Some(cmd);
                }
                cmd = &mut cmd2_future, if cmd2_result.is_none() => {
                    cmd2_result = Some(cmd);
                }
                _ = tokio::time::sleep(Duration::from_millis(16)) => {}
            }

            // Render frame with projectile extrapolation
            let elapsed = ai_start.elapsed().as_secs_f64();

            // Check for Ctrl+C
            if event::poll(Duration::from_millis(0))? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        return Ok(game.result());
                    }
                }
            }

            terminal.draw(|frame| {
                let layout = AppLayout::compute(frame.area());

                let pixel_w = layout.arena.width as usize;
                let pixel_h = (layout.arena.height as usize) * 2;
                let mut canvas = PixelCanvas::new(pixel_w, pixel_h);
                let vp =
                    Viewport::new(game.arena.width, game.arena.height, pixel_w, pixel_h);

                sprites::draw_arena_border(&mut canvas);

                // Ships: stay at current position (no new commands yet)
                for (i, ship) in game.ships.iter().enumerate() {
                    sprites::draw_sensor_range(&mut canvas, ship, &vp);
                    sprites::draw_ship(&mut canvas, ship, i, &vp);
                    sprites::draw_shield(&mut canvas, ship, i, &vp);
                }

                // Projectiles: extrapolate forward using velocity.
                // velocity is in game-units per turn; one turn corresponds
                // to interp_dur of real time, so elapsed / interp_dur gives
                // the fraction of a turn to advance.
                let proj_t = elapsed / interp_dur.as_secs_f64();
                for proj in &prev_projectiles {
                    let visual_pos = proj.position + proj.velocity * proj_t;
                    // Only draw if still within arena bounds
                    if visual_pos.x >= 0.0
                        && visual_pos.x <= game.arena.width
                        && visual_pos.y >= 0.0
                        && visual_pos.y <= game.arena.height
                    {
                        let mut visual_proj = proj.clone();
                        visual_proj.position = visual_pos;
                        sprites::draw_projectile(&mut canvas, &visual_proj, &vp);
                    }
                }

                frame.render_widget(&canvas, layout.arena);

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
        }

        let cmd1 = cmd1_result.unwrap();
        let cmd2 = cmd2_result.unwrap();

        // Apply Phase 1 elapsed movement to actual projectile positions so
        // they continue from where they were visually, not snap back.
        let phase1_t = ai_start.elapsed().as_secs_f64() / interp_dur.as_secs_f64();
        for proj in &mut game.projectiles {
            proj.position = proj.position + proj.velocity * phase1_t;
        }
        let (aw, ah) = (game.arena.width, game.arena.height);
        game.projectiles.retain(|p| p.is_in_bounds(aw, ah));

        // ===== Advance simulation =====
        game.advance([cmd1, cmd2]);
        event_log.push_game_events(&game.events);

        // ===== PHASE 2: Ship interpolation after advance =====
        let start = Instant::now();
        while start.elapsed() < interp_dur {
            let t = start.elapsed().as_secs_f64() / interp_dur.as_secs_f64();
            let t = t.min(1.0);

            // Check for Ctrl+C
            if event::poll(Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        return Ok(game.result());
                    }
                }
            }

            terminal.draw(|frame| {
                let layout = AppLayout::compute(frame.area());

                let pixel_w = layout.arena.width as usize;
                let pixel_h = (layout.arena.height as usize) * 2;
                let mut canvas = PixelCanvas::new(pixel_w, pixel_h);
                let vp =
                    Viewport::new(game.arena.width, game.arena.height, pixel_w, pixel_h);

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

                frame.render_widget(&canvas, layout.arena);

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
        }

        // Check end condition
        let result = game.result();
        if result != MatchResult::InProgress {
            return Ok(result);
        }
    }
}
