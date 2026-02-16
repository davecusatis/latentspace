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
        GameEvent::ShipFiredSecondary(i) => {
            (format!("{} heavy shot!", SHIP_NAMES[*i]), SHIP_COLORS[*i])
        }
        GameEvent::ShipHit { target, damage } => (
            format!("{} hit! -{} HP", SHIP_NAMES[*target], damage),
            Color::Red,
        ),
        GameEvent::ShipDestroyed(i) => {
            (format!("{} DESTROYED!", SHIP_NAMES[*i]), Color::Red)
        }
        GameEvent::ShieldActivated(i) => {
            (format!("{} shield UP", SHIP_NAMES[*i]), Color::Blue)
        }
        GameEvent::ShieldDeactivated(i) => (
            format!("{} shield DOWN", SHIP_NAMES[*i]),
            SHIP_COLORS[*i],
        ),
        GameEvent::BoundaryHit(i) => (
            format!("{} hit boundary", SHIP_NAMES[*i]),
            Color::DarkGray,
        ),
        GameEvent::RamDamage { ship, damage } => (
            format!("{} RAMMED! -{} HP", SHIP_NAMES[*ship], damage),
            Color::Rgb(255, 165, 0), // orange
        ),
    }
}

pub struct MarqueeWidget<'a> {
    log: &'a EventLog,
}

impl Widget for MarqueeWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .style(Style::default().bg(Color::Black));
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || self.log.events.is_empty() {
            return;
        }

        let visible_count = inner.width as usize / 15;
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
