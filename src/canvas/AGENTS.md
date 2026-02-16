# Canvas Module

Terminal rendering using Unicode braille characters for 4x resolution (2 wide x 4 tall pixels per terminal cell).

## Files

- **renderer.rs** — `PixelCanvas` stores a boolean dot grid + per-cell colors. Implements `Widget` to render braille characters (U+2800–U+28FF). Drawing primitives: `set_pixel`, `draw_circle` (filled), `draw_line` (Bresenham), `draw_ring` (outline).
- **sprites.rs** — Game object drawing functions + `Viewport` (game-to-pixel coordinate mapping) + `Explosion` struct/animation.

## Key Types

```rust
PixelCanvas { width, height, dots: Vec<bool>, cell_colors: Vec<Color> }
Viewport { game_width, game_height, pixel_width, pixel_height }
Explosion { position: Vec2, age: f64, duration: f64, radius: f64 }
```

## Drawing Functions

- `draw_ship` — circle (r=6) + heading line (len=10), colored Cyan/Magenta by index
- `draw_sensor_range` — dim blue ring at SENSOR_RANGE
- `draw_projectile` — small yellow circle (r=2)
- `draw_shield` — blue ring (r=10) when active
- `draw_arena_border` — gray rectangle outline
- `draw_explosion` — expanding ring (Yellow->Red->DarkRed) + 8 deterministic debris particles

## Explosion Parameters

- Hit: duration 0.4s, radius 20 game units
- Destroyed: duration 0.8s, radius 35 game units
- Progress `t = age/duration` controls ring size and color phase

## Conventions

- Colors: last-write-wins per cell (all dots in a 2x4 block share one color)
- Out-of-bounds `set_pixel` calls are silently ignored
- Canvas dimensions auto-round (width to even, height to multiple of 4)
- Viewport must be recreated each frame (terminal may resize)
