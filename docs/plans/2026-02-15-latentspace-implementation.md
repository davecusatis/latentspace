# LatentSpace Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a terminal-based 1v1 AI spaceship deathmatch arena where Claude-powered agents compete, rendered with half-block pixels and tachyonfx effects.

**Architecture:** Monolithic game loop in a single Rust binary. Turn-based simulation with interpolated rendering to appear real-time. AI agents communicate via Anthropic API. Modules: `game/` (simulation), `ai/` (LLM integration), `canvas/` (half-block rendering), `ui/` (layout/HUD/marquee).

**Tech Stack:** Rust, ratatui + crossterm, tachyonfx, tokio, reqwest, serde, clap

**Reference:** See `docs/plans/2026-02-15-latentspace-game-design.md` for full design.

---

### Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/game/mod.rs`
- Create: `src/ai/mod.rs`
- Create: `src/canvas/mod.rs`
- Create: `src/ui/mod.rs`

**Step 1: Initialize the Cargo project**

Run: `cargo init --name latentspace`

**Step 2: Set up Cargo.toml with all dependencies**

```toml
[package]
name = "latentspace"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.29"
crossterm = "0.28"
tachyonfx = "0.7"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
```

**Step 3: Create module structure**

Create `src/game/mod.rs`, `src/ai/mod.rs`, `src/canvas/mod.rs`, `src/ui/mod.rs` — each as empty modules.

Update `src/main.rs`:

```rust
mod ai;
mod canvas;
mod game;
mod ui;

fn main() {
    println!("LatentSpace — AI Spaceship Deathmatch Arena");
}
```

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully with no errors.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: scaffold project with dependencies and module structure"
```

---

### Task 2: Core Game Types — Vec2, Ship, Projectile

**Files:**
- Create: `src/game/ship.rs`
- Create: `src/game/projectile.rs`
- Modify: `src/game/mod.rs`

**Step 1: Write tests for Vec2 math**

Add to bottom of `src/game/mod.rs`:

```rust
pub mod ship;
pub mod projectile;

/// 2D vector used for positions and velocities.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    pub fn distance_to(self, other: Vec2) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    pub fn magnitude(self) -> f64 {
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }

    pub fn lerp(self, other: Vec2, t: f64) -> Vec2 {
        Vec2 {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl std::ops::Mul<f64> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self { x: self.x * rhs, y: self.y * rhs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec2_distance() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(3.0, 4.0);
        assert!((a.distance_to(b) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn vec2_lerp() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(10.0, 20.0);
        let mid = a.lerp(b, 0.5);
        assert!((mid.x - 5.0).abs() < 1e-10);
        assert!((mid.y - 10.0).abs() < 1e-10);
    }

    #[test]
    fn vec2_add() {
        let a = Vec2::new(1.0, 2.0);
        let b = Vec2::new(3.0, 4.0);
        let c = a + b;
        assert!((c.x - 4.0).abs() < 1e-10);
        assert!((c.y - 6.0).abs() < 1e-10);
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p latentspace game::tests`
Expected: 3 tests pass.

**Step 3: Write Ship struct**

In `src/game/ship.rs`:

```rust
use serde::{Deserialize, Serialize};
use super::Vec2;

// -- Constants --

pub const MAX_HEALTH: i32 = 100;
pub const MAX_ENERGY: i32 = 100;
pub const ENERGY_REGEN_PER_TURN: i32 = 3;
pub const SHIELD_ENERGY_COST: i32 = 5;
pub const PRIMARY_DAMAGE: i32 = 10;
pub const PRIMARY_COOLDOWN: i32 = 2;
pub const PRIMARY_ENERGY_COST: i32 = 5;
pub const SECONDARY_DAMAGE: i32 = 25;
pub const SECONDARY_COOLDOWN: i32 = 8;
pub const SECONDARY_ENERGY_COST: i32 = 15;
pub const MAX_SPEED: f64 = 15.0;
pub const THRUST_FORCE: f64 = 3.0;
pub const DRAG: f64 = 0.95;
pub const MAX_TURN_RATE: f64 = 30.0;
pub const SENSOR_RANGE: f64 = 150.0;
pub const SHIELD_DAMAGE_REDUCTION: f64 = 0.5;
pub const PRIMARY_PROJECTILE_SPEED: f64 = 20.0;
pub const SECONDARY_PROJECTILE_SPEED: f64 = 12.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ship {
    pub position: Vec2,
    pub velocity: Vec2,
    pub heading: f64,
    pub health: i32,
    pub energy: i32,
    pub shield_active: bool,
    pub primary_cooldown: i32,
    pub secondary_cooldown: i32,
    // Stats tracking
    pub shots_fired: i32,
    pub shots_hit: i32,
    pub damage_dealt: i32,
}

impl Ship {
    pub fn new(position: Vec2, heading: f64) -> Self {
        Self {
            position,
            velocity: Vec2::zero(),
            heading,
            health: MAX_HEALTH,
            energy: MAX_ENERGY,
            shield_active: false,
            primary_cooldown: 0,
            secondary_cooldown: 0,
            shots_fired: 0,
            shots_hit: 0,
            damage_dealt: 0,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0
    }

    pub fn apply_thrust(&mut self, thrust: f64) {
        let thrust = thrust.clamp(0.0, 1.0);
        let rad = self.heading.to_radians();
        let accel = Vec2::new(rad.cos(), rad.sin()) * (THRUST_FORCE * thrust);
        self.velocity = self.velocity + accel;
        let speed = self.velocity.magnitude();
        if speed > MAX_SPEED {
            self.velocity = self.velocity * (MAX_SPEED / speed);
        }
    }

    pub fn apply_turn(&mut self, turn_degrees: f64) {
        let turn = turn_degrees.clamp(-MAX_TURN_RATE, MAX_TURN_RATE);
        self.heading = (self.heading + turn) % 360.0;
        if self.heading < 0.0 {
            self.heading += 360.0;
        }
    }

    pub fn update_position(&mut self) {
        self.position = self.position + self.velocity;
        self.velocity = self.velocity * DRAG;
    }

    pub fn tick_cooldowns(&mut self) {
        self.primary_cooldown = (self.primary_cooldown - 1).max(0);
        self.secondary_cooldown = (self.secondary_cooldown - 1).max(0);
    }

    pub fn regen_energy(&mut self) {
        if self.shield_active {
            self.energy = (self.energy - SHIELD_ENERGY_COST).max(0);
            if self.energy == 0 {
                self.shield_active = false;
            }
        } else {
            self.energy = (self.energy + ENERGY_REGEN_PER_TURN).min(MAX_ENERGY);
        }
    }

    pub fn can_fire_primary(&self) -> bool {
        self.primary_cooldown == 0 && self.energy >= PRIMARY_ENERGY_COST
    }

    pub fn can_fire_secondary(&self) -> bool {
        self.secondary_cooldown == 0 && self.energy >= SECONDARY_ENERGY_COST
    }

    pub fn fire_primary(&mut self) {
        self.primary_cooldown = PRIMARY_COOLDOWN;
        self.energy -= PRIMARY_ENERGY_COST;
        self.shots_fired += 1;
    }

    pub fn fire_secondary(&mut self) {
        self.secondary_cooldown = SECONDARY_COOLDOWN;
        self.energy -= SECONDARY_ENERGY_COST;
        self.shots_fired += 1;
    }

    pub fn take_damage(&mut self, damage: i32) -> i32 {
        let actual = if self.shield_active {
            (damage as f64 * SHIELD_DAMAGE_REDUCTION) as i32
        } else {
            damage
        };
        self.health = (self.health - actual).max(0);
        actual
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ship_thrust_increases_velocity() {
        let mut ship = Ship::new(Vec2::new(100.0, 100.0), 0.0);
        ship.apply_thrust(1.0);
        assert!(ship.velocity.x > 0.0);
        assert!(ship.velocity.y.abs() < 1e-10);
    }

    #[test]
    fn ship_thrust_clamped() {
        let mut ship = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        ship.apply_thrust(2.0); // over 1.0, should clamp
        let v1 = ship.velocity.magnitude();
        let mut ship2 = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        ship2.apply_thrust(1.0);
        assert!((v1 - ship2.velocity.magnitude()).abs() < 1e-10);
    }

    #[test]
    fn ship_max_speed_cap() {
        let mut ship = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        for _ in 0..100 {
            ship.apply_thrust(1.0);
        }
        assert!(ship.velocity.magnitude() <= MAX_SPEED + 1e-10);
    }

    #[test]
    fn ship_turn_wraps() {
        let mut ship = Ship::new(Vec2::new(0.0, 0.0), 350.0);
        ship.apply_turn(20.0);
        assert!((ship.heading - 10.0).abs() < 1e-10);
    }

    #[test]
    fn ship_shield_reduces_damage() {
        let mut ship = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        ship.shield_active = true;
        let dmg = ship.take_damage(20);
        assert_eq!(dmg, 10);
        assert_eq!(ship.health, 90);
    }

    #[test]
    fn ship_shield_drains_energy() {
        let mut ship = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        ship.shield_active = true;
        ship.regen_energy();
        assert_eq!(ship.energy, MAX_ENERGY - SHIELD_ENERGY_COST);
    }

    #[test]
    fn ship_fire_primary_cooldown() {
        let mut ship = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        assert!(ship.can_fire_primary());
        ship.fire_primary();
        assert!(!ship.can_fire_primary());
        assert_eq!(ship.primary_cooldown, PRIMARY_COOLDOWN);
    }

    #[test]
    fn ship_dead_at_zero_health() {
        let mut ship = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        ship.take_damage(100);
        assert!(!ship.is_alive());
    }
}
```

**Step 4: Write Projectile struct**

In `src/game/projectile.rs`:

```rust
use super::Vec2;
use super::ship;

#[derive(Debug, Clone)]
pub struct Projectile {
    pub position: Vec2,
    pub velocity: Vec2,
    pub damage: i32,
    pub owner: usize, // 0 or 1 — which ship fired it
}

impl Projectile {
    pub fn spawn_primary(ship: &ship::Ship, owner: usize) -> Self {
        let rad = ship.heading.to_radians();
        let direction = Vec2::new(rad.cos(), rad.sin());
        Self {
            position: ship.position + direction * 5.0, // spawn slightly ahead of ship
            velocity: direction * ship::PRIMARY_PROJECTILE_SPEED + ship.velocity,
            damage: ship::PRIMARY_DAMAGE,
            owner,
        }
    }

    pub fn spawn_secondary(ship: &ship::Ship, owner: usize) -> Self {
        let rad = ship.heading.to_radians();
        let direction = Vec2::new(rad.cos(), rad.sin());
        Self {
            position: ship.position + direction * 5.0,
            velocity: direction * ship::SECONDARY_PROJECTILE_SPEED + ship.velocity,
            damage: ship::SECONDARY_DAMAGE,
            owner,
        }
    }

    pub fn update(&mut self) {
        self.position = self.position + self.velocity;
    }

    pub fn is_in_bounds(&self, arena_width: f64, arena_height: f64) -> bool {
        self.position.x >= 0.0
            && self.position.x <= arena_width
            && self.position.y >= 0.0
            && self.position.y <= arena_height
    }

    pub fn hits_ship(&self, ship: &ship::Ship, hit_radius: f64) -> bool {
        self.position.distance_to(ship.position) <= hit_radius
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_spawns_ahead_of_ship() {
        let ship = ship::Ship::new(Vec2::new(100.0, 100.0), 0.0);
        let proj = Projectile::spawn_primary(&ship, 0);
        assert!(proj.position.x > ship.position.x);
    }

    #[test]
    fn projectile_moves_on_update() {
        let ship = ship::Ship::new(Vec2::new(100.0, 100.0), 0.0);
        let mut proj = Projectile::spawn_primary(&ship, 0);
        let old_x = proj.position.x;
        proj.update();
        assert!(proj.position.x > old_x);
    }

    #[test]
    fn projectile_out_of_bounds() {
        let ship = ship::Ship::new(Vec2::new(-10.0, 100.0), 180.0);
        let mut proj = Projectile::spawn_primary(&ship, 0);
        proj.position = Vec2::new(-5.0, 100.0);
        assert!(!proj.is_in_bounds(800.0, 400.0));
    }
}
```

**Step 5: Run all tests**

Run: `cargo test`
Expected: All tests pass (Vec2, Ship, Projectile).

**Step 6: Commit**

```bash
git add src/game/
git commit -m "feat: add core game types — Vec2, Ship, Projectile"
```

---

### Task 3: Arena & Turn Resolution

**Files:**
- Create: `src/game/arena.rs`
- Create: `src/game/combat.rs`
- Create: `src/game/fog.rs`
- Modify: `src/game/mod.rs`

**Step 1: Write Arena struct**

In `src/game/arena.rs`:

```rust
use super::Vec2;
use super::ship::Ship;

pub const HIT_RADIUS: f64 = 8.0;
pub const BOUNDARY_DAMAGE: i32 = 5;

#[derive(Debug, Clone)]
pub struct Arena {
    pub width: f64,
    pub height: f64,
}

impl Arena {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// Clamp ship to arena bounds. Returns true if ship hit boundary.
    pub fn enforce_boundary(&self, ship: &mut Ship) -> bool {
        let mut hit = false;
        if ship.position.x < 0.0 {
            ship.position.x = 0.0;
            ship.velocity.x = ship.velocity.x.abs();
            hit = true;
        } else if ship.position.x > self.width {
            ship.position.x = self.width;
            ship.velocity.x = -ship.velocity.x.abs();
            hit = true;
        }
        if ship.position.y < 0.0 {
            ship.position.y = 0.0;
            ship.velocity.y = ship.velocity.y.abs();
            hit = true;
        } else if ship.position.y > self.height {
            ship.position.y = self.height;
            ship.velocity.y = -ship.velocity.y.abs();
            hit = true;
        }
        hit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_bounces_ship() {
        let arena = Arena::new(800.0, 400.0);
        let mut ship = Ship::new(Vec2::new(-5.0, 200.0), 180.0);
        ship.velocity = Vec2::new(-10.0, 0.0);
        let hit = arena.enforce_boundary(&mut ship);
        assert!(hit);
        assert_eq!(ship.position.x, 0.0);
        assert!(ship.velocity.x > 0.0);
    }

    #[test]
    fn no_bounce_when_inside() {
        let arena = Arena::new(800.0, 400.0);
        let mut ship = Ship::new(Vec2::new(400.0, 200.0), 0.0);
        let hit = arena.enforce_boundary(&mut ship);
        assert!(!hit);
    }
}
```

**Step 2: Write fog of war**

In `src/game/fog.rs`:

```rust
use super::Vec2;
use super::ship::{Ship, SENSOR_RANGE};
use super::projectile::Projectile;

/// Returns true if `target` is within sensor range of `observer`.
pub fn is_visible(observer: &Ship, target: &Ship) -> bool {
    observer.position.distance_to(target.position) <= SENSOR_RANGE
}

/// Filter projectiles to only those within sensor range of the observer.
pub fn visible_projectiles(observer: &Ship, projectiles: &[Projectile]) -> Vec<Projectile> {
    projectiles
        .iter()
        .filter(|p| observer.position.distance_to(p.position) <= SENSOR_RANGE)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ship_visible_within_range() {
        let a = Ship::new(Vec2::new(100.0, 100.0), 0.0);
        let b = Ship::new(Vec2::new(200.0, 100.0), 0.0);
        assert!(is_visible(&a, &b)); // 100 units apart, within 150 range
    }

    #[test]
    fn ship_not_visible_out_of_range() {
        let a = Ship::new(Vec2::new(0.0, 0.0), 0.0);
        let b = Ship::new(Vec2::new(200.0, 200.0), 0.0);
        assert!(!is_visible(&a, &b)); // ~283 units apart, outside 150 range
    }
}
```

**Step 3: Write combat resolution**

In `src/game/combat.rs`:

```rust
use super::arena::{Arena, HIT_RADIUS, BOUNDARY_DAMAGE};
use super::projectile::Projectile;
use super::ship::Ship;

/// Events generated during a turn for the marquee.
#[derive(Debug, Clone)]
pub enum GameEvent {
    ShipFiredPrimary(usize),
    ShipFiredSecondary(usize),
    ShipHit { target: usize, damage: i32 },
    ShipDestroyed(usize),
    ShieldActivated(usize),
    ShieldDeactivated(usize),
    BoundaryHit(usize),
}

/// Resolve one tick of combat. Returns events that occurred.
pub fn resolve_projectile_hits(
    ships: &mut [Ship; 2],
    projectiles: &mut Vec<Projectile>,
) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut to_remove = Vec::new();

    for (i, proj) in projectiles.iter().enumerate() {
        let target = 1 - proj.owner;
        if proj.hits_ship(&ships[target], HIT_RADIUS) {
            let dmg = ships[target].take_damage(proj.damage);
            ships[proj.owner].shots_hit += 1;
            ships[proj.owner].damage_dealt += dmg;
            events.push(GameEvent::ShipHit { target, damage: dmg });
            if !ships[target].is_alive() {
                events.push(GameEvent::ShipDestroyed(target));
            }
            to_remove.push(i);
        }
    }

    // Remove hit projectiles in reverse order to preserve indices
    for i in to_remove.into_iter().rev() {
        projectiles.swap_remove(i);
    }

    events
}

/// Apply boundary rules and return events.
pub fn resolve_boundaries(
    ships: &mut [Ship; 2],
    arena: &Arena,
) -> Vec<GameEvent> {
    let mut events = Vec::new();
    for (i, ship) in ships.iter_mut().enumerate() {
        if arena.enforce_boundary(ship) {
            ship.take_damage(BOUNDARY_DAMAGE);
            events.push(GameEvent::BoundaryHit(i));
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Vec2;

    #[test]
    fn projectile_hits_target() {
        let mut ships = [
            Ship::new(Vec2::new(100.0, 100.0), 0.0),
            Ship::new(Vec2::new(105.0, 100.0), 180.0), // within hit radius
        ];
        let mut projectiles = vec![Projectile {
            position: Vec2::new(105.0, 100.0),
            velocity: Vec2::new(10.0, 0.0),
            damage: 10,
            owner: 0,
        }];
        let events = resolve_projectile_hits(&mut ships, &mut projectiles);
        assert_eq!(ships[1].health, 90);
        assert!(projectiles.is_empty());
        assert!(events.iter().any(|e| matches!(e, GameEvent::ShipHit { target: 1, .. })));
    }

    #[test]
    fn projectile_misses() {
        let mut ships = [
            Ship::new(Vec2::new(100.0, 100.0), 0.0),
            Ship::new(Vec2::new(500.0, 500.0), 180.0), // far away
        ];
        let mut projectiles = vec![Projectile {
            position: Vec2::new(200.0, 200.0),
            velocity: Vec2::new(10.0, 0.0),
            damage: 10,
            owner: 0,
        }];
        let events = resolve_projectile_hits(&mut ships, &mut projectiles);
        assert_eq!(ships[1].health, 100);
        assert_eq!(projectiles.len(), 1);
        assert!(events.is_empty());
    }
}
```

**Step 4: Update `src/game/mod.rs` to export new modules**

Add to `src/game/mod.rs` (after the existing `pub mod ship; pub mod projectile;`):

```rust
pub mod arena;
pub mod combat;
pub mod fog;
```

**Step 5: Run all tests**

Run: `cargo test`
Expected: All tests pass.

**Step 6: Commit**

```bash
git add src/game/
git commit -m "feat: add arena, combat resolution, and fog of war"
```

---

### Task 4: AI Protocol — Serialization & Deserialization

**Files:**
- Create: `src/ai/protocol.rs`
- Modify: `src/ai/mod.rs`

**Step 1: Write the AI protocol types and tests**

In `src/ai/protocol.rs`:

```rust
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
    }

    #[test]
    fn game_state_shows_enemy_in_range() {
        let ships = [
            Ship::new(Vec2::new(100.0, 100.0), 0.0),
            Ship::new(Vec2::new(200.0, 100.0), 90.0),
        ];
        let state = build_game_state(1, 0, &ships, &[], 800.0, 400.0);
        assert!(state.enemy.is_some());
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
    }
}
```

**Step 2: Update `src/ai/mod.rs`**

```rust
pub mod protocol;
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass.

**Step 4: Commit**

```bash
git add src/ai/
git commit -m "feat: add AI protocol types and JSON parsing"
```

---

### Task 5: AI Client — Anthropic API Integration

**Files:**
- Create: `src/ai/client.rs`
- Create: `src/ai/history.rs`
- Modify: `src/ai/mod.rs`

**Step 1: Write the conversation history manager**

In `src/ai/history.rs`:

```rust
use serde::Serialize;

const MAX_HISTORY_TURNS: usize = 20;

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug)]
pub struct ConversationHistory {
    messages: Vec<Message>,
}

impl ConversationHistory {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn add_user(&mut self, content: String) {
        self.messages.push(Message {
            role: "user".to_string(),
            content,
        });
        self.trim();
    }

    pub fn add_assistant(&mut self, content: String) {
        self.messages.push(Message {
            role: "assistant".to_string(),
            content,
        });
        self.trim();
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    fn trim(&mut self) {
        // Keep the last MAX_HISTORY_TURNS * 2 messages (user + assistant pairs)
        let max_messages = MAX_HISTORY_TURNS * 2;
        if self.messages.len() > max_messages {
            let drain_count = self.messages.len() - max_messages;
            self.messages.drain(..drain_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_adds_messages() {
        let mut h = ConversationHistory::new();
        h.add_user("state1".to_string());
        h.add_assistant("cmd1".to_string());
        assert_eq!(h.messages().len(), 2);
    }

    #[test]
    fn history_trims_old_messages() {
        let mut h = ConversationHistory::new();
        for i in 0..50 {
            h.add_user(format!("state{i}"));
            h.add_assistant(format!("cmd{i}"));
        }
        assert_eq!(h.messages().len(), MAX_HISTORY_TURNS * 2);
        // Most recent should be preserved
        assert!(h.messages().last().unwrap().content.contains("cmd49"));
    }
}
```

**Step 2: Write the Anthropic API client**

In `src/ai/client.rs`:

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::history::ConversationHistory;
use super::protocol::{self, ShipCommand};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const MODEL: &str = "claude-sonnet-4-5-20250929";
const AI_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_TOKENS: i32 = 256;

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: i32,
    system: String,
    messages: Vec<super::history::Message>,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

pub struct AiAgent {
    client: Client,
    api_key: String,
    system_prompt: String,
    pub history: ConversationHistory,
}

impl AiAgent {
    pub fn new(api_key: String, system_prompt: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            system_prompt,
            history: ConversationHistory::new(),
        }
    }

    /// Send the game state to the AI and get a command back.
    /// Returns the default (drift) command on timeout or error.
    pub async fn get_command(&mut self, game_state_json: &str) -> ShipCommand {
        self.history.add_user(game_state_json.to_string());

        let request = ApiRequest {
            model: MODEL.to_string(),
            max_tokens: MAX_TOKENS,
            system: self.system_prompt.clone(),
            messages: self.history.messages().to_vec(),
        };

        let result = tokio::time::timeout(AI_TIMEOUT, async {
            self.client
                .post(API_URL)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&request)
                .send()
                .await
        })
        .await;

        match result {
            Ok(Ok(response)) => {
                if let Ok(api_response) = response.json::<ApiResponse>().await {
                    if let Some(text) = api_response.content.first().and_then(|c| c.text.as_ref()) {
                        self.history.add_assistant(text.clone());
                        match protocol::parse_command(text) {
                            Ok(cmd) => return cmd,
                            Err(_) => return ShipCommand::default(),
                        }
                    }
                }
                ShipCommand::default()
            }
            _ => ShipCommand::default(), // Timeout or network error — ship drifts
        }
    }
}
```

**Step 3: Update `src/ai/mod.rs`**

```rust
pub mod client;
pub mod history;
pub mod protocol;
```

**Step 4: Run tests**

Run: `cargo test`
Expected: All tests pass. (Client itself isn't unit-tested since it requires a live API — tested via integration later.)

**Step 5: Commit**

```bash
git add src/ai/
git commit -m "feat: add Anthropic API client and conversation history"
```

---

### Task 6: Game Loop — Turn Simulation

**Files:**
- Create: `src/game/simulation.rs`
- Modify: `src/game/mod.rs`

**Step 1: Write the simulation engine with tests**

In `src/game/simulation.rs`:

```rust
use super::arena::Arena;
use super::combat::{self, GameEvent};
use super::projectile::Projectile;
use super::ship::Ship;
use crate::ai::protocol::ShipCommand;

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
        // Ships start at opposite ends facing each other
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

    /// Apply commands from both AIs and advance the simulation by one turn.
    pub fn advance(&mut self, commands: [ShipCommand; 2]) {
        self.turn += 1;
        self.events.clear();

        // Apply commands
        for (i, cmd) in commands.iter().enumerate() {
            let ship = &mut self.ships[i];

            // Shield toggle
            if cmd.shield && !ship.shield_active {
                ship.shield_active = true;
                self.events.push(GameEvent::ShieldActivated(i));
            } else if !cmd.shield && ship.shield_active {
                ship.shield_active = false;
                self.events.push(GameEvent::ShieldDeactivated(i));
            }

            // Movement
            ship.apply_thrust(cmd.thrust);
            ship.apply_turn(cmd.turn);

            // Firing
            if cmd.fire_primary && ship.can_fire_primary() {
                ship.fire_primary();
                self.projectiles
                    .push(Projectile::spawn_primary(ship, i));
                self.events.push(GameEvent::ShipFiredPrimary(i));
            }
            if cmd.fire_secondary && ship.can_fire_secondary() {
                ship.fire_secondary();
                self.projectiles
                    .push(Projectile::spawn_secondary(ship, i));
                self.events.push(GameEvent::ShipFiredSecondary(i));
            }
        }

        // Update positions
        for ship in &mut self.ships {
            ship.update_position();
        }
        for proj in &mut self.projectiles {
            proj.update();
        }

        // Resolve combat
        let hit_events = combat::resolve_projectile_hits(&mut self.ships, &mut self.projectiles);
        self.events.extend(hit_events);

        // Boundary enforcement
        let boundary_events = combat::resolve_boundaries(&mut self.ships, &self.arena);
        self.events.extend(boundary_events);

        // Remove out-of-bounds projectiles
        let (w, h) = (self.arena.width, self.arena.height);
        self.projectiles.retain(|p| p.is_in_bounds(w, h));

        // Cooldowns and energy
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
}
```

**Step 2: Update `src/game/mod.rs`**

Add:
```rust
pub mod simulation;
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass.

**Step 4: Commit**

```bash
git add src/game/
git commit -m "feat: add game simulation engine with turn resolution"
```

---

### Task 7: Canvas — Half-Block Pixel Renderer

**Files:**
- Create: `src/canvas/renderer.rs`
- Modify: `src/canvas/mod.rs`

**Step 1: Write the half-block pixel buffer**

In `src/canvas/renderer.rs`:

```rust
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

/// A pixel buffer that renders using half-block characters.
/// Each terminal cell represents 2 vertical pixels using ▀ with
/// foreground = top pixel, background = bottom pixel.
pub struct PixelCanvas {
    width: usize,
    height: usize, // must be even — represents pixel rows
    pixels: Vec<Color>,
}

impl PixelCanvas {
    /// Create a canvas. `pixel_height` will be rounded up to even.
    pub fn new(width: usize, pixel_height: usize) -> Self {
        let height = if pixel_height % 2 == 0 {
            pixel_height
        } else {
            pixel_height + 1
        };
        Self {
            width,
            height,
            pixels: vec![Color::Black; width * height],
        }
    }

    pub fn pixel_width(&self) -> usize {
        self.width
    }

    pub fn pixel_height(&self) -> usize {
        self.height
    }

    /// Terminal rows needed = pixel_height / 2
    pub fn cell_height(&self) -> usize {
        self.height / 2
    }

    pub fn clear(&mut self) {
        self.pixels.fill(Color::Black);
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = color;
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Color {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x]
        } else {
            Color::Black
        }
    }

    /// Draw a filled circle.
    pub fn draw_circle(&mut self, cx: f64, cy: f64, radius: f64, color: Color) {
        let r2 = radius * radius;
        let min_x = ((cx - radius).floor() as isize).max(0) as usize;
        let max_x = ((cx + radius).ceil() as isize).min(self.width as isize - 1) as usize;
        let min_y = ((cy - radius).floor() as isize).max(0) as usize;
        let max_y = ((cy + radius).ceil() as isize).min(self.height as isize - 1) as usize;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = px as f64 - cx;
                let dy = py as f64 - cy;
                if dx * dx + dy * dy <= r2 {
                    self.set_pixel(px, py, color);
                }
            }
        }
    }

    /// Draw a line using Bresenham's algorithm.
    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut x = x0;
        let mut y = y0;

        loop {
            if x >= 0 && y >= 0 {
                self.set_pixel(x as usize, y as usize, color);
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Draw a ring (unfilled circle outline).
    pub fn draw_ring(&mut self, cx: f64, cy: f64, radius: f64, thickness: f64, color: Color) {
        let outer_r2 = radius * radius;
        let inner_r2 = (radius - thickness).max(0.0).powi(2);
        let min_x = ((cx - radius).floor() as isize).max(0) as usize;
        let max_x = ((cx + radius).ceil() as isize).min(self.width as isize - 1) as usize;
        let min_y = ((cy - radius).floor() as isize).max(0) as usize;
        let max_y = ((cy + radius).ceil() as isize).min(self.height as isize - 1) as usize;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = px as f64 - cx;
                let dy = py as f64 - cy;
                let dist2 = dx * dx + dy * dy;
                if dist2 <= outer_r2 && dist2 >= inner_r2 {
                    self.set_pixel(px, py, color);
                }
            }
        }
    }
}

/// Widget implementation — renders the pixel buffer into a ratatui area.
impl Widget for &PixelCanvas {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rows = self.cell_height().min(area.height as usize);
        let cols = self.width.min(area.width as usize);

        for row in 0..rows {
            for col in 0..cols {
                let top = self.get_pixel(col, row * 2);
                let bottom = self.get_pixel(col, row * 2 + 1);
                let cell = &mut buf[(area.x + col as u16, area.y + row as u16)];
                cell.set_char('▀');
                cell.set_fg(top);
                cell.set_bg(bottom);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_dimensions() {
        let c = PixelCanvas::new(80, 48);
        assert_eq!(c.pixel_width(), 80);
        assert_eq!(c.pixel_height(), 48);
        assert_eq!(c.cell_height(), 24);
    }

    #[test]
    fn canvas_odd_height_rounds_up() {
        let c = PixelCanvas::new(80, 47);
        assert_eq!(c.pixel_height(), 48);
    }

    #[test]
    fn set_and_get_pixel() {
        let mut c = PixelCanvas::new(10, 10);
        c.set_pixel(5, 5, Color::Red);
        assert_eq!(c.get_pixel(5, 5), Color::Red);
        assert_eq!(c.get_pixel(0, 0), Color::Black);
    }

    #[test]
    fn out_of_bounds_ignored() {
        let mut c = PixelCanvas::new(10, 10);
        c.set_pixel(100, 100, Color::Red); // should not panic
        assert_eq!(c.get_pixel(100, 100), Color::Black);
    }

    #[test]
    fn clear_resets_all() {
        let mut c = PixelCanvas::new(10, 10);
        c.set_pixel(5, 5, Color::Red);
        c.clear();
        assert_eq!(c.get_pixel(5, 5), Color::Black);
    }

    #[test]
    fn circle_draws_pixels() {
        let mut c = PixelCanvas::new(20, 20);
        c.draw_circle(10.0, 10.0, 3.0, Color::Cyan);
        assert_eq!(c.get_pixel(10, 10), Color::Cyan); // center
        assert_eq!(c.get_pixel(0, 0), Color::Black); // outside
    }
}
```

**Step 2: Update `src/canvas/mod.rs`**

```rust
pub mod renderer;
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass.

**Step 4: Commit**

```bash
git add src/canvas/
git commit -m "feat: add half-block pixel canvas renderer"
```

---

### Task 8: Sprites — Ship & Projectile Visuals

**Files:**
- Create: `src/canvas/sprites.rs`
- Modify: `src/canvas/mod.rs`

**Step 1: Write sprite rendering for ships and projectiles**

In `src/canvas/sprites.rs`:

```rust
use ratatui::style::Color;

use super::renderer::PixelCanvas;
use crate::game::ship::Ship;
use crate::game::projectile::Projectile;
use crate::game::ship::SENSOR_RANGE;

/// Map game coordinates to pixel coordinates on the canvas.
pub struct Viewport {
    pub game_width: f64,
    pub game_height: f64,
    pub pixel_width: usize,
    pub pixel_height: usize,
}

impl Viewport {
    pub fn new(game_width: f64, game_height: f64, pixel_width: usize, pixel_height: usize) -> Self {
        Self { game_width, game_height, pixel_width, pixel_height }
    }

    pub fn to_pixel_x(&self, game_x: f64) -> f64 {
        (game_x / self.game_width) * self.pixel_width as f64
    }

    pub fn to_pixel_y(&self, game_y: f64) -> f64 {
        (game_y / self.game_height) * self.pixel_height as f64
    }

    pub fn to_pixel_radius(&self, game_radius: f64) -> f64 {
        (game_radius / self.game_width) * self.pixel_width as f64
    }
}

const SHIP_COLORS: [Color; 2] = [Color::Cyan, Color::Magenta];
const PROJECTILE_COLOR: Color = Color::Yellow;
const SENSOR_COLOR: Color = Color::Rgb(30, 30, 60);

/// Draw a ship as a small triangle-like shape pointing in its heading direction.
pub fn draw_ship(canvas: &mut PixelCanvas, ship: &Ship, ship_idx: usize, vp: &Viewport) {
    let cx = vp.to_pixel_x(ship.position.x);
    let cy = vp.to_pixel_y(ship.position.y);
    let color = SHIP_COLORS[ship_idx];

    // Draw ship body as a filled circle (simple but visible)
    canvas.draw_circle(cx, cy, 3.0, color);

    // Draw nose in heading direction
    let rad = ship.heading.to_radians();
    let nose_x = cx + rad.cos() * 5.0;
    let nose_y = cy + rad.sin() * 5.0;
    canvas.draw_line(cx as i32, cy as i32, nose_x as i32, nose_y as i32, color);
}

/// Draw sensor range ring around a ship.
pub fn draw_sensor_range(canvas: &mut PixelCanvas, ship: &Ship, vp: &Viewport) {
    let cx = vp.to_pixel_x(ship.position.x);
    let cy = vp.to_pixel_y(ship.position.y);
    let radius = vp.to_pixel_radius(SENSOR_RANGE);
    canvas.draw_ring(cx, cy, radius, 1.0, SENSOR_COLOR);
}

/// Draw a projectile as a bright dot.
pub fn draw_projectile(canvas: &mut PixelCanvas, proj: &Projectile, vp: &Viewport) {
    let px = vp.to_pixel_x(proj.position.x);
    let py = vp.to_pixel_y(proj.position.y);
    canvas.draw_circle(px, py, 1.0, PROJECTILE_COLOR);
}

/// Draw the arena boundary as a dim border.
pub fn draw_arena_border(canvas: &mut PixelCanvas) {
    let w = canvas.pixel_width();
    let h = canvas.pixel_height();
    let color = Color::DarkGray;
    canvas.draw_line(0, 0, w as i32 - 1, 0, color);
    canvas.draw_line(0, 0, 0, h as i32 - 1, color);
    canvas.draw_line(w as i32 - 1, 0, w as i32 - 1, h as i32 - 1, color);
    canvas.draw_line(0, h as i32 - 1, w as i32 - 1, h as i32 - 1, color);
}

/// Draw shield glow around a ship when active.
pub fn draw_shield(canvas: &mut PixelCanvas, ship: &Ship, ship_idx: usize, vp: &Viewport) {
    if ship.shield_active {
        let cx = vp.to_pixel_x(ship.position.x);
        let cy = vp.to_pixel_y(ship.position.y);
        let color = Color::Rgb(80, 80, 255);
        canvas.draw_ring(cx, cy, 5.0, 1.0, color);
    }
    let _ = ship_idx; // reserved for per-ship shield coloring later
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Vec2;

    #[test]
    fn viewport_maps_coordinates() {
        let vp = Viewport::new(800.0, 400.0, 160, 80);
        assert!((vp.to_pixel_x(400.0) - 80.0).abs() < 1e-10);
        assert!((vp.to_pixel_y(200.0) - 40.0).abs() < 1e-10);
    }

    #[test]
    fn draw_ship_does_not_panic() {
        let mut canvas = PixelCanvas::new(80, 40);
        let ship = Ship::new(Vec2::new(400.0, 200.0), 45.0);
        let vp = Viewport::new(800.0, 400.0, 80, 40);
        draw_ship(&mut canvas, &ship, 0, &vp);
        // Just verify it doesn't panic and draws something
        assert_eq!(canvas.get_pixel(40, 20), Color::Cyan);
    }
}
```

**Step 2: Update `src/canvas/mod.rs`**

```rust
pub mod renderer;
pub mod sprites;
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass.

**Step 4: Commit**

```bash
git add src/canvas/
git commit -m "feat: add ship and projectile sprite rendering"
```

---

### Task 9: UI Layout — Sidebar, HUD, Marquee

**Files:**
- Create: `src/ui/layout.rs`
- Create: `src/ui/hud.rs`
- Create: `src/ui/marquee.rs`
- Modify: `src/ui/mod.rs`

**Step 1: Write the layout splitter**

In `src/ui/layout.rs`:

```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub arena: Rect,
    pub sidebar: Rect,
    pub marquee: Rect,
    // Sidebar sub-areas
    pub ship1_hud: Rect,
    pub ship2_hud: Rect,
    pub match_info: Rect,
}

impl AppLayout {
    pub fn compute(area: Rect) -> Self {
        // Split: top (arena + sidebar) and bottom (marquee)
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),     // arena + sidebar
                Constraint::Length(3),    // marquee
            ])
            .split(area);

        let top = vertical[0];
        let marquee = vertical[1];

        // Split top: left (arena ~75%) and right (sidebar ~25%)
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(75),
                Constraint::Percentage(25),
            ])
            .split(top);

        let arena = horizontal[0];
        let sidebar = horizontal[1];

        // Split sidebar into 3 sections
        let sidebar_sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(38), // ship 1 HUD
                Constraint::Percentage(38), // ship 2 HUD
                Constraint::Percentage(24), // match info
            ])
            .split(sidebar);

        Self {
            arena,
            sidebar,
            marquee,
            ship1_hud: sidebar_sections[0],
            ship2_hud: sidebar_sections[1],
            match_info: sidebar_sections[2],
        }
    }
}
```

**Step 2: Write the HUD widget**

In `src/ui/hud.rs`:

```rust
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Widget};

use crate::game::ship::{Ship, MAX_HEALTH, MAX_ENERGY};

pub struct ShipHud<'a> {
    pub ship: &'a Ship,
    pub name: &'a str,
    pub color: Color,
}

impl Widget for ShipHud<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(Span::styled(
                format!(" {} ", self.name),
                Style::default().fg(self.color),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.color));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 4 {
            return;
        }

        // HP bar
        let hp_ratio = self.ship.health as f64 / MAX_HEALTH as f64;
        let hp_color = if hp_ratio > 0.5 {
            Color::Green
        } else if hp_ratio > 0.25 {
            Color::Yellow
        } else {
            Color::Red
        };
        let hp_area = Rect::new(inner.x, inner.y, inner.width, 1);
        Gauge::default()
            .label(format!("HP: {}", self.ship.health))
            .ratio(hp_ratio)
            .gauge_style(Style::default().fg(hp_color).bg(Color::DarkGray))
            .render(hp_area, buf);

        // Energy bar
        let en_ratio = self.ship.energy as f64 / MAX_ENERGY as f64;
        let en_area = Rect::new(inner.x, inner.y + 1, inner.width, 1);
        Gauge::default()
            .label(format!("EN: {}", self.ship.energy))
            .ratio(en_ratio)
            .gauge_style(Style::default().fg(Color::Blue).bg(Color::DarkGray))
            .render(en_area, buf);

        // Shield status
        let shield_text = if self.ship.shield_active { "ON" } else { "OFF" };
        let shield_color = if self.ship.shield_active { Color::Cyan } else { Color::DarkGray };
        let shield_line = Line::from(vec![
            Span::raw("Shield: "),
            Span::styled(shield_text, Style::default().fg(shield_color)),
        ]);
        buf.set_line(inner.x, inner.y + 2, &shield_line, inner.width);

        // Cooldowns
        if inner.height >= 5 {
            let pri = if self.ship.primary_cooldown == 0 { "RDY" } else { &format!("{}", self.ship.primary_cooldown) };
            let sec = if self.ship.secondary_cooldown == 0 { "RDY" } else { &format!("{}", self.ship.secondary_cooldown) };
            let cd_line = Line::from(format!("CD: {}/{}", pri, sec));
            buf.set_line(inner.x, inner.y + 3, &cd_line, inner.width);
        }
    }
}

pub struct MatchInfo {
    pub turn: i32,
    pub max_turns: i32,
}

impl Widget for MatchInfo {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Match ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height >= 1 {
            let line = Line::from(format!("Turn: {}/{}", self.turn, self.max_turns));
            buf.set_line(inner.x, inner.y, &line, inner.width);
        }
    }
}
```

**Step 3: Write the marquee widget**

In `src/ui/marquee.rs`:

```rust
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Widget};

use crate::game::combat::GameEvent;

const SHIP_COLORS: [Color; 2] = [Color::Cyan, Color::Magenta];
const SHIP_NAMES: [&str; 2] = ["Ship 1", "Ship 2"];

pub struct EventLog {
    events: Vec<(String, Color)>,
    max_events: usize,
}

impl EventLog {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Vec::new(),
            max_events,
        }
    }

    pub fn push_game_events(&mut self, events: &[GameEvent]) {
        for event in events {
            let (text, color) = format_event(event);
            self.events.push((text, color));
        }
        if self.events.len() > self.max_events {
            let drain = self.events.len() - self.max_events;
            self.events.drain(..drain);
        }
    }

    pub fn widget(&self) -> MarqueeWidget<'_> {
        MarqueeWidget { log: self }
    }
}

fn format_event(event: &GameEvent) -> (String, Color) {
    match event {
        GameEvent::ShipFiredPrimary(i) => (format!("{} fires", SHIP_NAMES[*i]), SHIP_COLORS[*i]),
        GameEvent::ShipFiredSecondary(i) => (format!("{} heavy shot!", SHIP_NAMES[*i]), SHIP_COLORS[*i]),
        GameEvent::ShipHit { target, damage } => (format!("{} hit! -{} HP", SHIP_NAMES[*target], damage), Color::Red),
        GameEvent::ShipDestroyed(i) => (format!("{} DESTROYED!", SHIP_NAMES[*i]), Color::Red),
        GameEvent::ShieldActivated(i) => (format!("{} shield UP", SHIP_NAMES[*i]), Color::Blue),
        GameEvent::ShieldDeactivated(i) => (format!("{} shield DOWN", SHIP_NAMES[*i]), SHIP_COLORS[*i]),
        GameEvent::BoundaryHit(i) => (format!("{} hit boundary", SHIP_NAMES[*i]), Color::DarkGray),
    }
}

pub struct MarqueeWidget<'a> {
    log: &'a EventLog,
}

impl Widget for MarqueeWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || self.log.events.is_empty() {
            return;
        }

        // Show last N events that fit in the area, joined with " · "
        let visible_count = inner.width as usize / 15; // rough estimate
        let start = self.log.events.len().saturating_sub(visible_count.max(3));
        let spans: Vec<Span> = self.log.events[start..]
            .iter()
            .enumerate()
            .flat_map(|(i, (text, color))| {
                let mut v = Vec::new();
                if i > 0 {
                    v.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
                }
                v.push(Span::styled(text.clone(), Style::default().fg(*color)));
                v
            })
            .collect();

        let line = Line::from(spans);
        buf.set_line(inner.x, inner.y, &line, inner.width);
    }
}
```

**Step 4: Update `src/ui/mod.rs`**

```rust
pub mod hud;
pub mod layout;
pub mod marquee;
```

**Step 5: Run tests**

Run: `cargo test`
Expected: All tests pass (UI modules have no unit tests — tested via integration).

**Step 6: Commit**

```bash
git add src/ui/
git commit -m "feat: add UI layout, HUD widgets, and event marquee"
```

---

### Task 10: Main Loop — CLI, Rendering, Game Integration

**Files:**
- Modify: `src/main.rs`
- Create: `src/canvas/effects.rs`

**Step 1: Write tachyonfx effect wrappers**

In `src/canvas/effects.rs`:

```rust
// Placeholder for tachyonfx effect compositions.
// Effects are applied post-render on the ratatui buffer.
// We'll add specific effects (muzzle flash, hit dissolve, etc.) as we iterate.
```

Update `src/canvas/mod.rs`:
```rust
pub mod effects;
pub mod renderer;
pub mod sprites;
```

**Step 2: Write the full main.rs**

```rust
mod ai;
mod canvas;
mod game;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use ai::client::AiAgent;
use ai::protocol;
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
    #[arg(long, default_value_t = 200)]
    turns: i32,

    /// Arena dimensions (WxH)
    #[arg(long, default_value = "800x400")]
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
        "fast" => Duration::from_millis(300),
        "slow" => Duration::from_millis(1500),
        _ => Duration::from_millis(750),
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
            println!("Ship 1 ({}): {} HP | {} shots | {} hits | {} damage dealt",
                ship1_name, game.ships[0].health, game.ships[0].shots_fired,
                game.ships[0].shots_hit, game.ships[0].damage_dealt);
            println!("Ship 2 ({}): {} HP | {} shots | {} hits | {} damage dealt",
                ship2_name, game.ships[1].health, game.ships[1].shots_fired,
                game.ships[1].shots_hit, game.ships[1].damage_dealt);
        }
        Err(e) => eprintln!("Error: {e}"),
    }

    Ok(())
}

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
            game.turn, 0, &game.ships, &game.projectiles, game.arena.width, game.arena.height,
        ))?;
        let state2_json = serde_json::to_string(&protocol::build_game_state(
            game.turn, 1, &game.ships, &game.projectiles, game.arena.width, game.arena.height,
        ))?;

        // Request commands from both AIs in parallel
        let (cmd1, cmd2) = tokio::join!(
            agent1.get_command(&state1_json),
            agent2.get_command(&state2_json),
        );

        // Advance simulation
        game.advance([cmd1, cmd2]);
        event_log.push_game_events(&game.events);

        // Interpolated rendering
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

                // Canvas
                let pixel_w = layout.arena.width as usize;
                let pixel_h = (layout.arena.height as usize) * 2; // half-block doubles vertical
                let mut canvas = PixelCanvas::new(pixel_w, pixel_h);
                let vp = Viewport::new(game.arena.width, game.arena.height, pixel_w, pixel_h);

                sprites::draw_arena_border(&mut canvas);

                // Interpolate ship positions
                for i in 0..2 {
                    let mut interp_ship = game.ships[i].clone();
                    interp_ship.position = prev_ships[i].position.lerp(game.ships[i].position, t);
                    sprites::draw_sensor_range(&mut canvas, &interp_ship, &vp);
                    sprites::draw_ship(&mut canvas, &interp_ship, i, &vp);
                    sprites::draw_shield(&mut canvas, &interp_ship, i, &vp);
                }

                // Interpolate projectile positions
                for (j, proj) in game.projectiles.iter().enumerate() {
                    let mut interp_proj = proj.clone();
                    if j < prev_projectiles.len() {
                        interp_proj.position = prev_projectiles[j].position.lerp(proj.position, t);
                    }
                    sprites::draw_projectile(&mut canvas, &interp_proj, &vp);
                }

                frame.render_widget(&canvas, layout.arena);

                // HUD
                frame.render_widget(
                    ShipHud { ship: &game.ships[0], name: ship1_name, color: ratatui::style::Color::Cyan },
                    layout.ship1_hud,
                );
                frame.render_widget(
                    ShipHud { ship: &game.ships[1], name: ship2_name, color: ratatui::style::Color::Magenta },
                    layout.ship2_hud,
                );
                frame.render_widget(
                    MatchInfo { turn: game.turn, max_turns: game.max_turns },
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
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

**Step 4: Commit**

```bash
git add src/
git commit -m "feat: integrate main game loop with CLI, rendering, and AI"
```

---

### Task 11: Example Prompts

**Files:**
- Create: `prompts/aggressive.txt`
- Create: `prompts/defensive.txt`

**Step 1: Write aggressive prompt**

In `prompts/aggressive.txt`:

```
You are piloting a spaceship in a 1v1 deathmatch arena. Your goal is to destroy the enemy ship.

Strategy: AGGRESSIVE
- Always thrust toward the enemy when detected
- Fire primary weapons whenever possible
- Use secondary weapon when close range
- Never use shields — offense is the best defense
- When enemy is not visible, thrust forward and sweep to find them
- Aim by turning to face the enemy before firing

Respond with ONLY a JSON object:
{"thrust": 0.0-1.0, "turn": -30 to 30, "fire_primary": bool, "fire_secondary": bool, "shield": bool}
```

**Step 2: Write defensive prompt**

In `prompts/defensive.txt`:

```
You are piloting a spaceship in a 1v1 deathmatch arena. Your goal is to destroy the enemy ship.

Strategy: DEFENSIVE
- Keep distance from the enemy — stay at medium range
- Use shields when taking fire or when enemy is close
- Fire primary weapons at range, conserve secondary for good opportunities
- Manage energy carefully — don't let it drop below 30
- When enemy is not visible, move slowly and conserve energy
- Evade incoming projectiles by turning perpendicular to their path

Respond with ONLY a JSON object:
{"thrust": 0.0-1.0, "turn": -30 to 30, "fire_primary": bool, "fire_secondary": bool, "shield": bool}
```

**Step 3: Commit**

```bash
git add prompts/
git commit -m "feat: add example AI strategy prompts"
```

---

### Task 12: Integration Test — Full Match Smoke Test

**Files:**
- Create: `tests/integration.rs`

**Step 1: Write a simulation-only integration test (no AI, no rendering)**

In `tests/integration.rs`:

```rust
use latentspace::ai::protocol::{parse_command, ShipCommand};
use latentspace::game::simulation::{GameState, MatchResult};

#[test]
fn full_match_with_scripted_commands() {
    let mut game = GameState::new(800.0, 400.0, 50);

    // Ship 0 charges forward and fires, Ship 1 sits still
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

    // Either ship 0 won or we hit turn limit
    let result = game.result();
    assert!(result != MatchResult::InProgress, "Match should have ended");
}

#[test]
fn parse_command_handles_various_formats() {
    // Raw JSON
    let cmd = parse_command(r#"{"thrust":1.0,"turn":5.0,"fire_primary":true,"fire_secondary":false,"shield":false}"#).unwrap();
    assert!((cmd.thrust - 1.0).abs() < 1e-10);

    // Code block
    let cmd = parse_command("```json\n{\"thrust\":0.5}\n```").unwrap();
    assert!((cmd.thrust - 0.5).abs() < 1e-10);

    // Surrounded by text
    let cmd = parse_command("Here is my command: {\"thrust\":0.3} hope that works").unwrap();
    assert!((cmd.thrust - 0.3).abs() < 1e-10);
}
```

**Step 2: Make modules public for integration tests**

In `src/main.rs`, ensure the module declarations use `pub mod` instead of `mod`:

```rust
pub mod ai;
pub mod canvas;
pub mod game;
pub mod ui;
```

**Step 3: Run integration tests**

Run: `cargo test --test integration`
Expected: All tests pass.

**Step 4: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

**Step 5: Commit**

```bash
git add tests/ src/main.rs
git commit -m "feat: add integration tests for simulation and protocol"
```

---

### Task 13: Polish — TachyonFX Effects

**Files:**
- Modify: `src/canvas/effects.rs`

**Step 1: Add effect compositions for game events**

In `src/canvas/effects.rs`:

```rust
use std::time::Duration;
use tachyonfx::{fx, CellFilter, Effect, Interpolation::*};
use ratatui::style::Color;

/// Muzzle flash effect — brief bright white burst.
pub fn muzzle_flash() -> Effect {
    fx::sequence(&[
        fx::fade_from(Color::White, Color::Black, (150, LinearOut)),
    ])
}

/// Hit impact — red burst that dissolves.
pub fn hit_impact() -> Effect {
    fx::sequence(&[
        fx::fade_from(Color::Red, Color::Black, (300, QuadOut)),
        fx::dissolve((200, LinearOut)),
    ])
}

/// Ship destruction — dramatic dissolve.
pub fn ship_destroyed() -> Effect {
    fx::sequence(&[
        fx::fade_from(Color::Yellow, Color::Red, (400, LinearOut)),
        fx::dissolve((600, QuadOut)),
    ])
}

/// Low health warning — pulsing red.
pub fn low_health_pulse() -> Effect {
    fx::ping_pong(fx::fade_to(
        Color::Red,
        Color::DarkGray,
        (500, SineInOut),
    ))
}
```

Note: The exact tachyonfx API may require adjustment based on the version's available functions. Adapt the function signatures to match the actual tachyonfx 0.7 API at build time.

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles. If tachyonfx API differs from above, adjust function calls to match actual API.

**Step 3: Commit**

```bash
git add src/canvas/effects.rs
git commit -m "feat: add tachyonfx visual effects for combat events"
```

---

### Task 14: Final Verification

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

**Step 3: Build release binary**

Run: `cargo build --release`
Expected: Compiles successfully.

**Step 4: Verify CLI help**

Run: `cargo run --release -- --help`
Expected: Shows help text with `--ship1`, `--ship2`, `--turns`, `--arena`, `--speed`, `--api-key` flags.

**Step 5: Commit any fixes**

If clippy or compilation required fixes, commit them:
```bash
git add -A
git commit -m "chore: fix clippy warnings and polish"
```
