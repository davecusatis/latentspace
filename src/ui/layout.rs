use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub arena: Rect,
    pub sidebar: Rect,
    pub marquee: Rect,
    pub ship1_hud: Rect,
    pub ship2_hud: Rect,
    pub match_info: Rect,
}

impl AppLayout {
    pub fn compute(area: Rect) -> Self {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(area);

        let top = vertical[0];
        let marquee = vertical[1];

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(75),
                Constraint::Percentage(25),
            ])
            .split(top);

        let arena = horizontal[0];
        let sidebar = horizontal[1];

        let sidebar_sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(38),
                Constraint::Percentage(38),
                Constraint::Percentage(24),
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
