use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::ai::protocol::ShipCommand;
use crate::game::ship::Ship;
use crate::game::simulation::GameState;

pub struct DebugOverlay<'a> {
    pub game: &'a GameState,
    pub commands: &'a [ShipCommand; 2],
    pub ship_names: [&'a str; 2],
}

impl Widget for DebugOverlay<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let s1 = &self.game.ships[0];
        let s2 = &self.game.ships[1];
        let c1 = &self.commands[0];
        let c2 = &self.commands[1];

        let dist = s1.position.distance_to(s2.position);
        let bearing = (s2.position.y - s1.position.y)
            .atan2(s2.position.x - s1.position.x)
            .to_degrees()
            .rem_euclid(360.0);

        let col_width = 36usize;
        let lines = build_lines(
            s1,
            s2,
            c1,
            c2,
            self.ship_names[0],
            self.ship_names[1],
            dist,
            bearing,
            self.game.projectiles.len(),
            self.game.turn,
            self.game.max_turns,
            col_width,
        );

        let panel_w = (col_width as u16 * 2 + 3).min(area.width);
        let panel_h = (lines.len() as u16 + 2).min(area.height);
        let panel_x = area.x + 1;
        let panel_y = area.y + 1;

        let bg = Style::default().fg(Color::White).bg(Color::Black);
        let dim = Style::default().fg(Color::DarkGray).bg(Color::Black);

        // Draw background
        for y in panel_y..panel_y + panel_h {
            for x in panel_x..panel_x + panel_w {
                if x < area.x + area.width && y < area.y + area.height {
                    buf[(x, y)].set_char(' ').set_style(bg);
                }
            }
        }

        // Draw border
        let border_style = dim;
        let right = panel_x + panel_w - 1;
        let bottom = panel_y + panel_h - 1;
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

        // Title
        let title = " DEBUG [d] ";
        let title_x = panel_x + 2;
        if title_x + title.len() as u16 <= area.x + area.width && panel_y < area.y + area.height {
            let title_line = Line::from(Span::styled(
                title,
                Style::default().fg(Color::Yellow).bg(Color::Black),
            ));
            buf.set_line(title_x, panel_y, &title_line, panel_w - 3);
        }

        // Content lines
        let content_x = panel_x + 1;
        let content_y = panel_y + 1;
        let max_w = panel_w.saturating_sub(2);

        for (i, line) in lines.iter().enumerate() {
            let y = content_y + i as u16;
            if y >= bottom || y >= area.y + area.height {
                break;
            }
            buf.set_line(content_x, y, line, max_w);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_lines(
    s1: &Ship,
    s2: &Ship,
    c1: &ShipCommand,
    c2: &ShipCommand,
    name1: &str,
    name2: &str,
    dist: f64,
    bearing: f64,
    proj_count: usize,
    turn: i32,
    max_turns: i32,
    col_w: usize,
) -> Vec<Line<'static>> {
    let cyan = Style::default().fg(Color::Cyan).bg(Color::Black);
    let magenta = Style::default().fg(Color::Magenta).bg(Color::Black);
    let white = Style::default().fg(Color::White).bg(Color::Black);
    let yellow = Style::default().fg(Color::Yellow).bg(Color::Black);
    let gray = Style::default().fg(Color::DarkGray).bg(Color::Black);

    let mut lines = Vec::new();

    // Header
    let h1 = format!("{:^col_w$}", name1, col_w = col_w);
    let h2 = format!("{:^col_w$}", name2, col_w = col_w);
    lines.push(Line::from(vec![
        Span::styled(h1, cyan),
        Span::styled(" ", gray),
        Span::styled(h2, magenta),
    ]));

    // Position
    let p1 = format!(
        "pos: ({:>7.1}, {:>7.1})       ",
        s1.position.x, s1.position.y
    );
    let p2 = format!(
        "pos: ({:>7.1}, {:>7.1})       ",
        s2.position.x, s2.position.y
    );
    lines.push(two_col(&p1, &p2, col_w, white, gray));

    // Velocity + speed
    let spd1 = s1.velocity.magnitude();
    let spd2 = s2.velocity.magnitude();
    let v1 = format!(
        "vel: ({:>6.1},{:>6.1}) spd:{:>5.1}",
        s1.velocity.x, s1.velocity.y, spd1
    );
    let v2 = format!(
        "vel: ({:>6.1},{:>6.1}) spd:{:>5.1}",
        s2.velocity.x, s2.velocity.y, spd2
    );
    lines.push(two_col(&v1, &v2, col_w, white, gray));

    // Heading
    let hd1 = format!("hdg: {:>6.1}               ", s1.heading);
    let hd2 = format!("hdg: {:>6.1}               ", s2.heading);
    lines.push(two_col(&hd1, &hd2, col_w, white, gray));

    // HP / Energy / Shield
    let shld = |s: &Ship| if s.shield_active { "ON" } else { "OFF" };
    let he1 = format!(
        "hp:{:>4} nrg:{:>4} shld:{:<3}   ",
        s1.health, s1.energy, shld(s1)
    );
    let he2 = format!(
        "hp:{:>4} nrg:{:>4} shld:{:<3}   ",
        s2.health, s2.energy, shld(s2)
    );
    lines.push(two_col(&he1, &he2, col_w, white, gray));

    // Cooldowns
    let cd = |s: &Ship| {
        let pri = if s.primary_cooldown == 0 {
            "RDY".to_string()
        } else {
            format!("{}t", s.primary_cooldown)
        };
        let sec = if s.secondary_cooldown == 0 {
            "RDY".to_string()
        } else {
            format!("{}t", s.secondary_cooldown)
        };
        format!("pri:{:<4} sec:{:<4}           ", pri, sec)
    };
    lines.push(two_col(&cd(s1), &cd(s2), col_w, white, gray));

    // AI commands
    let cmd_str = |c: &ShipCommand| {
        let f1 = if c.fire_primary { "F1" } else { "--" };
        let f2 = if c.fire_secondary { "F2" } else { "--" };
        let sh = if c.shield { "S" } else { "-" };
        format!(
            "cmd: T={:<4.2} R={:>6.1} {} {} {}",
            c.thrust, c.turn, f1, f2, sh
        )
    };
    let cm1 = cmd_str(c1);
    let cm2 = cmd_str(c2);
    lines.push(two_col(&cm1, &cm2, col_w, yellow, gray));

    // Shared info line
    let info = format!(
        "dist:{:>7.1}  bearing:{:>6.1}  projs:{}  turn:{}/{}",
        dist, bearing, proj_count, turn, max_turns
    );
    lines.push(Line::from(Span::styled(info, white)));

    lines
}

fn two_col(left: &str, right: &str, col_w: usize, style: Style, sep: Style) -> Line<'static> {
    let l = if left.len() > col_w {
        left[..col_w].to_string()
    } else {
        format!("{:<col_w$}", left, col_w = col_w)
    };
    let r = if right.len() > col_w {
        right[..col_w].to_string()
    } else {
        format!("{:<col_w$}", right, col_w = col_w)
    };
    Line::from(vec![
        Span::styled(l, style),
        Span::styled(" ", sep),
        Span::styled(r, style),
    ])
}
