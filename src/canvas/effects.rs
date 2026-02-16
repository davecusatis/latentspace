//! Visual effects for combat events using tachyonfx.
//!
//! These effect compositions are applied post-render on the ratatui buffer
//! to provide visual feedback for game events such as weapon fire, impacts,
//! ship destruction, and low-health warnings.

use ratatui::style::Color;
use tachyonfx::fx::{self, Direction};
use tachyonfx::{Effect, Interpolation};

/// Bright flash effect for weapon fire.
///
/// A quick white-to-black fade simulating a muzzle flash. The effect uses
/// a fast linear fade-out over 150ms.
pub fn muzzle_flash() -> Effect {
    fx::fade_from_fg(Color::White, (150_u32, Interpolation::Linear))
}

/// Red flash effect for projectile impacts.
///
/// A two-phase effect: first a red flash fading out over 300ms with quadratic
/// easing, then a dissolve over 200ms to scatter the impact area.
pub fn hit_impact() -> Effect {
    fx::sequence(&[
        fx::fade_from_fg(Color::Red, (300_u32, Interpolation::QuadOut)),
        fx::dissolve((200_u32, Interpolation::Linear)),
    ])
}

/// Dramatic explosion effect for ship destruction.
///
/// A multi-phase sequence: a yellow-to-red color sweep over 400ms, followed
/// by a dissolve over 600ms with quadratic easing to break apart the ship.
pub fn ship_destroyed() -> Effect {
    fx::sequence(&[
        fx::fade_from(Color::Yellow, Color::Red, (400_u32, Interpolation::Linear)),
        fx::dissolve((600_u32, Interpolation::QuadOut)),
    ])
}

/// Pulsing red warning for low-health ships.
///
/// A ping-pong fade effect that oscillates between the current color and
/// dark red, creating a breathing/pulsing warning. Runs as a single
/// ping-pong cycle over 500ms with sinusoidal easing.
pub fn low_health_pulse() -> Effect {
    fx::ping_pong(
        fx::fade_to_fg(Color::Red, (500_u32, Interpolation::SineInOut)),
    )
}

/// Sweep-in effect for newly spawned entities.
///
/// Sweeps content in from left to right with a short gradient, useful
/// for revealing new ships or UI elements.
pub fn spawn_sweep() -> Effect {
    fx::sweep_in(
        Direction::LeftToRight,
        5,  // gradient length
        2,  // randomness
        Color::Black,
        (400_u32, Interpolation::CubicOut),
    )
}
