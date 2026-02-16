use latentspace::ai::protocol::{parse_command, ShipCommand};
use latentspace::game::simulation::{GameState, MatchResult};

#[test]
fn full_match_with_scripted_commands() {
    let mut game = GameState::new(800.0, 400.0, 50);

    for _ in 0..50 {
        let cmd0 = ShipCommand {
            thrust: 1.0,
            turn: 0.0,
            fire_primary: true,
            fire_secondary: false,
            shield: false,
        };
        let cmd1 = ShipCommand::default();
        game.advance([cmd0, cmd1]);

        if game.result() != MatchResult::InProgress {
            break;
        }
    }

    let result = game.result();
    assert!(result != MatchResult::InProgress, "Match should have ended");
}

#[test]
fn parse_command_handles_various_formats() {
    let cmd = parse_command(r#"{"thrust":1.0,"turn":5.0,"fire_primary":true,"fire_secondary":false,"shield":false}"#).unwrap();
    assert!((cmd.thrust - 1.0).abs() < 1e-10);

    let cmd = parse_command("```json\n{\"thrust\":0.5}\n```").unwrap();
    assert!((cmd.thrust - 0.5).abs() < 1e-10);

    let cmd = parse_command("Here is my command: {\"thrust\":0.3} hope that works").unwrap();
    assert!((cmd.thrust - 0.3).abs() < 1e-10);
}
