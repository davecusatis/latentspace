use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

pub struct StartupOverlay {
    /// Progress from 0.0 (just appeared) to 1.0 (about to disappear).
    pub progress: f64,
}

impl Widget for StartupOverlay {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = "LATENTSPACE";
        let subtitle = "AI Spaceship Deathmatch Arena";

        let panel_w: u16 = (subtitle.len() as u16 + 4).min(area.width);
        let panel_h: u16 = 5u16.min(area.height);

        // Center the panel
        let panel_x = area.x + area.width.saturating_sub(panel_w) / 2;
        let panel_y = area.y + area.height.saturating_sub(panel_h) / 2;

        // Fade effect: bright cyan initially, dims to gray after 70% progress
        let text_color = if self.progress > 0.7 { Color::DarkGray } else { Color::Cyan };
        let border_color = Color::DarkGray;

        let text_style = Style::default().fg(text_color).bg(Color::Black);
        let border_style = Style::default().fg(border_color).bg(Color::Black);
        let bg = Style::default().fg(Color::White).bg(Color::Black);

        let right = panel_x + panel_w - 1;
        let bottom = panel_y + panel_h - 1;

        // Draw background
        for y in panel_y..=bottom {
            for x in panel_x..=right {
                if x < area.x + area.width && y < area.y + area.height {
                    buf[(x, y)].set_char(' ').set_style(bg);
                }
            }
        }

        // Draw border
        for x in panel_x..=right {
            if x < area.x + area.width {
                if panel_y < area.y + area.height {
                    buf[(x, panel_y)].set_char('─').set_style(border_style);
                }
                if bottom < area.y + area.height {
                    buf[(x, bottom)].set_char('─').set_style(border_style);
                }
            }
        }
        for y in panel_y..=bottom {
            if y < area.y + area.height {
                if panel_x < area.x + area.width {
                    buf[(panel_x, y)].set_char('│').set_style(border_style);
                }
                if right < area.x + area.width {
                    buf[(right, y)].set_char('│').set_style(border_style);
                }
            }
        }
        // Corners
        if panel_x < area.x + area.width && panel_y < area.y + area.height {
            buf[(panel_x, panel_y)].set_char('┌').set_style(border_style);
        }
        if right < area.x + area.width && panel_y < area.y + area.height {
            buf[(right, panel_y)].set_char('┐').set_style(border_style);
        }
        if panel_x < area.x + area.width && bottom < area.y + area.height {
            buf[(panel_x, bottom)].set_char('└').set_style(border_style);
        }
        if right < area.x + area.width && bottom < area.y + area.height {
            buf[(right, bottom)].set_char('┘').set_style(border_style);
        }

        // Title line (centered in panel)
        let content_w = panel_w.saturating_sub(2);
        let title_x = panel_x + 1 + content_w.saturating_sub(title.len() as u16) / 2;
        let title_y = panel_y + 1;
        if title_y < area.y + area.height {
            let title_line = Line::from(Span::styled(title, text_style));
            buf.set_line(title_x, title_y, &title_line, content_w);
        }

        // Blank separator line (panel_y + 2) is already background

        // Subtitle line (centered in panel)
        let sub_x = panel_x + 1 + content_w.saturating_sub(subtitle.len() as u16) / 2;
        let sub_y = panel_y + 3;
        if sub_y < area.y + area.height {
            let sub_style = Style::default()
                .fg(if self.progress > 0.7 { Color::DarkGray } else { Color::Gray })
                .bg(Color::Black);
            let sub_line = Line::from(Span::styled(subtitle, sub_style));
            buf.set_line(sub_x, sub_y, &sub_line, content_w);
        }
    }
}
