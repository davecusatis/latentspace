use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Widget};

use crate::game::ship::{Ship, MAX_ENERGY, MAX_HEALTH};

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
        let shield_text = if self.ship.shield_active {
            "ON"
        } else {
            "OFF"
        };
        let shield_color = if self.ship.shield_active {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        let shield_line = Line::from(vec![
            Span::raw("Shield: "),
            Span::styled(shield_text, Style::default().fg(shield_color)),
        ]);
        buf.set_line(inner.x, inner.y + 2, &shield_line, inner.width);

        // Cooldowns
        if inner.height >= 5 {
            let pri = if self.ship.primary_cooldown == 0 {
                "RDY".to_string()
            } else {
                format!("{}", self.ship.primary_cooldown)
            };
            let sec = if self.ship.secondary_cooldown == 0 {
                "RDY".to_string()
            } else {
                format!("{}", self.ship.secondary_cooldown)
            };
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
