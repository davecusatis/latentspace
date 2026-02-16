# UI Module

Ratatui widgets for the terminal interface. No game logic — display only.

## Files

- **layout.rs** — `AppLayout::compute(area)` divides the screen: arena (75% width, 97% height), sidebar (25% width split into ship1_hud/ship2_hud/match_info), marquee (bottom 3 rows).
- **hud.rs** — `ShipHud` widget shows HP bar (green/yellow/red), energy bar (blue), shield status, weapon cooldowns. `MatchInfo` shows turn counter.
- **marquee.rs** — `EventLog` collects `GameEvent`s as colored strings. `MarqueeWidget` renders recent events in a scrolling line separated by " · ". Max 50 events.
- **debug_overlay.rs** — `DebugOverlay` shows ship positions, velocities, headings, HP, energy, AI commands, distance/bearing. Toggled with `d` key. Two-column layout, bordered panel.

## Layout

```
+-----------------------------+--------+
|                             | Ship 1 |
|        Arena (75%)          +--------+
|                             | Ship 2 |
|                             +--------+
|                             | Match  |
+-----------------------------+--------+
|       Marquee (3 rows)               |
+--------------------------------------+
```

## Conventions

- Ship 1 = Cyan, Ship 2 = Magenta (consistent across all widgets)
- HP bar color: green (>50%), yellow (>25%), red (<=25%)
- Event colors: ship actions use ship color, hits/destroyed use Red, boundaries use DarkGray
- Widgets degrade gracefully with small terminal sizes
