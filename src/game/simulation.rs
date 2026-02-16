use super::arena::Arena;
use super::combat::{self, GameEvent};
use super::projectile::Projectile;
use super::ship::Ship;
use crate::ai::protocol::ShipCommand;

/// Number of sub-steps per turn for projectile movement and collision
/// detection. Projectile velocity is scaled by 1/PROJECTILE_SUBSTEPS per
/// sub-step so total displacement per turn is preserved.
pub const PROJECTILE_SUBSTEPS: u32 = 4;

#[derive(Debug, Clone)]
pub struct GameState {
    pub ships: [Ship; 2],
    pub projectiles: Vec<Projectile>,
    pub arena: Arena,
    pub turn: i32,
    pub max_turns: i32,
    pub events: Vec<GameEvent>,
}

#[derive(Debug, PartialEq)]
pub enum MatchResult {
    InProgress,
    Winner(usize),
    Draw,
}

impl GameState {
    pub fn new(arena_width: f64, arena_height: f64, max_turns: i32) -> Self {
        let arena = Arena::new(arena_width, arena_height);
        let ship1 = Ship::new(
            super::Vec2::new(arena_width * 0.2, arena_height / 2.0),
            0.0,
        );
        let ship2 = Ship::new(
            super::Vec2::new(arena_width * 0.8, arena_height / 2.0),
            180.0,
        );
        Self {
            ships: [ship1, ship2],
            projectiles: Vec::new(),
            arena,
            turn: 0,
            max_turns,
            events: Vec::new(),
        }
    }

    pub fn advance(&mut self, commands: [ShipCommand; 2]) {
        self.turn += 1;
        self.events.clear();

        for (i, cmd) in commands.iter().enumerate() {
            let ship = &mut self.ships[i];

            if cmd.shield && !ship.shield_active {
                ship.shield_active = true;
                self.events.push(GameEvent::ShieldActivated(i));
            } else if !cmd.shield && ship.shield_active {
                ship.shield_active = false;
                self.events.push(GameEvent::ShieldDeactivated(i));
            }

            ship.apply_thrust(cmd.thrust);
            ship.apply_turn(cmd.turn);

            if cmd.fire_primary && ship.can_fire_primary() {
                ship.fire_primary();
                self.projectiles.push(Projectile::spawn_primary(ship, i));
                self.events.push(GameEvent::ShipFiredPrimary(i));
            }
            if cmd.fire_secondary && ship.can_fire_secondary() {
                ship.fire_secondary();
                self.projectiles.push(Projectile::spawn_secondary(ship, i));
                self.events.push(GameEvent::ShipFiredSecondary(i));
            }
        }

        // Ship movement: once per turn (unchanged)
        for ship in &mut self.ships {
            ship.update_position();
        }

        // Ram damage and knockback: resolve after ships move, before projectiles
        let ram_events = combat::resolve_ram_damage(&mut self.ships);
        self.events.extend(ram_events);
        combat::apply_proximity_knockback(&mut self.ships);

        // Projectile sub-stepping: move projectiles in smaller increments for
        // smoother visuals and more accurate collision detection.
        let dt = 1.0 / PROJECTILE_SUBSTEPS as f64;
        let (w, h) = (self.arena.width, self.arena.height);

        for _ in 0..PROJECTILE_SUBSTEPS {
            for proj in &mut self.projectiles {
                proj.update_substep(dt);
            }

            let hit_events =
                combat::resolve_projectile_hits(&mut self.ships, &mut self.projectiles);
            self.events.extend(hit_events);

            self.projectiles.retain(|p| p.is_in_bounds(w, h));
        }

        let boundary_events = combat::resolve_boundaries(&mut self.ships, &self.arena);
        self.events.extend(boundary_events);

        for ship in &mut self.ships {
            ship.tick_cooldowns();
            ship.regen_energy();
        }
    }

    pub fn result(&self) -> MatchResult {
        let alive: Vec<usize> = (0..2).filter(|&i| self.ships[i].is_alive()).collect();
        match alive.len() {
            0 => MatchResult::Draw,
            1 => MatchResult::Winner(alive[0]),
            _ => {
                if self.turn >= self.max_turns {
                    if self.ships[0].health > self.ships[1].health {
                        MatchResult::Winner(0)
                    } else if self.ships[1].health > self.ships[0].health {
                        MatchResult::Winner(1)
                    } else {
                        MatchResult::Draw
                    }
                } else {
                    MatchResult::InProgress
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_commands() -> [ShipCommand; 2] {
        [ShipCommand::default(), ShipCommand::default()]
    }

    #[test]
    fn game_starts_in_progress() {
        let state = GameState::new(800.0, 400.0, 200);
        assert_eq!(state.result(), MatchResult::InProgress);
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn turn_advances() {
        let mut state = GameState::new(800.0, 400.0, 200);
        state.advance(default_commands());
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn thrust_moves_ship() {
        let mut state = GameState::new(800.0, 400.0, 200);
        let old_x = state.ships[0].position.x;
        let cmds = [
            ShipCommand { thrust: 1.0, ..ShipCommand::default() },
            ShipCommand::default(),
        ];
        state.advance(cmds);
        assert!(state.ships[0].position.x > old_x);
    }

    #[test]
    fn firing_creates_projectile() {
        let mut state = GameState::new(800.0, 400.0, 200);
        let cmds = [
            ShipCommand { fire_primary: true, ..ShipCommand::default() },
            ShipCommand::default(),
        ];
        state.advance(cmds);
        assert!(!state.projectiles.is_empty());
    }

    #[test]
    fn timeout_gives_hp_winner() {
        let mut state = GameState::new(800.0, 400.0, 5);
        state.ships[0].health = 80;
        state.ships[1].health = 90;
        for _ in 0..5 {
            state.advance(default_commands());
        }
        assert_eq!(state.result(), MatchResult::Winner(1));
    }

    #[test]
    fn destruction_gives_winner() {
        let mut state = GameState::new(800.0, 400.0, 200);
        state.ships[1].health = 0;
        assert_eq!(state.result(), MatchResult::Winner(0));
    }

    #[test]
    fn projectile_substep_preserves_total_distance() {
        use crate::game::projectile::Projectile;
        use crate::game::Vec2;

        let vel = Vec2::new(20.0, 0.0);
        let start = Vec2::new(100.0, 100.0);

        // Single full-step update
        let mut p_full = Projectile {
            position: start,
            velocity: vel,
            damage: 10,
            owner: 0,
            distance_traveled: 0.0,
        };
        p_full.update();
        let full_end = p_full.position;

        // Sub-stepped update
        let mut p_sub = Projectile {
            position: start,
            velocity: vel,
            damage: 10,
            owner: 0,
            distance_traveled: 0.0,
        };
        let dt = 1.0 / PROJECTILE_SUBSTEPS as f64;
        for _ in 0..PROJECTILE_SUBSTEPS {
            p_sub.update_substep(dt);
        }
        let sub_end = p_sub.position;

        assert!(
            (full_end.x - sub_end.x).abs() < 1e-10,
            "x: full={} sub={}",
            full_end.x,
            sub_end.x
        );
        assert!(
            (full_end.y - sub_end.y).abs() < 1e-10,
            "y: full={} sub={}",
            full_end.y,
            sub_end.y
        );
    }
}
