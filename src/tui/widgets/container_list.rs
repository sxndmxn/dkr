use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::docker::models::{ContainerStatus, HealthStatus};
use crate::tui::app::App;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.config.theme;
    let healthy_color = parse_hex_color(&theme.healthy);
    let warning_color = parse_hex_color(&theme.warning);
    let critical_color = parse_hex_color(&theme.critical);
    let accent_color = parse_hex_color(&theme.accent);

    let mut lines: Vec<Line<'_>> = Vec::new();
    let mut current_idx: usize = 0;
    let max_name_width = usize::from(area.width.saturating_sub(16));
    let capped = max_name_width.min(20);

    for group in &app.groups {
        if app.config.display.group_by_project && !group.name.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("[{}]", group.name),
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            )));
        }

        for c in &group.containers {
            let is_selected = current_idx == app.selected_index;
            let indicator = container_indicator(&c.status, &c.health);
            let color = container_color(
                &c.status,
                &c.health,
                healthy_color,
                warning_color,
                critical_color,
            );

            let name = if c.name.len() > capped {
                format!("{}...", &c.name[..capped.saturating_sub(3)])
            } else {
                c.name.clone()
            };

            let prefix = if is_selected { ">" } else { " " };
            let status_text = c.status_display();

            let mut style = Style::default().fg(color);
            if is_selected {
                style = style.add_modifier(Modifier::BOLD);
            }

            let prefix_style = if is_selected {
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            lines.push(Line::from(vec![
                Span::styled(format!("{prefix} "), prefix_style),
                Span::styled(format!("{name:<capped$}"), style),
                Span::styled(
                    format!(" {status_text:<10}"),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!(" {indicator}"), style),
            ]));

            current_idx += 1;
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No containers match filter",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let list = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(list, area);
}

const fn container_indicator(status: &ContainerStatus, health: &HealthStatus) -> &'static str {
    match (status, health) {
        (
            ContainerStatus::Running,
            HealthStatus::Healthy | HealthStatus::None | HealthStatus::Unhealthy,
        ) => "\u{25cf}",
        (ContainerStatus::Running, HealthStatus::Starting) => "\u{25d0}",
        (ContainerStatus::Restarting, _) => "\u{21bb}",
        (ContainerStatus::Paused, _) => "\u{23f8}",
        (
            ContainerStatus::Exited(_)
            | ContainerStatus::Dead
            | ContainerStatus::Created
            | ContainerStatus::Removing
            | ContainerStatus::Unknown(_),
            _,
        ) => "\u{25cb}",
    }
}

const fn container_color(
    status: &ContainerStatus,
    health: &HealthStatus,
    healthy: Color,
    warning: Color,
    critical: Color,
) -> Color {
    match (status, health) {
        (ContainerStatus::Running, HealthStatus::Healthy | HealthStatus::None) => healthy,
        (ContainerStatus::Running, HealthStatus::Starting)
        | (ContainerStatus::Restarting | ContainerStatus::Paused, _) => warning,
        (ContainerStatus::Running, HealthStatus::Unhealthy)
        | (ContainerStatus::Exited(_) | ContainerStatus::Dead, _) => critical,
        (ContainerStatus::Created | ContainerStatus::Removing | ContainerStatus::Unknown(_), _) => {
            Color::DarkGray
        }
    }
}

fn parse_hex_color(hex: &str) -> Color {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return Color::White;
    }
    let Ok(r) = u8::from_str_radix(&hex[0..2], 16) else {
        return Color::White;
    };
    let Ok(g) = u8::from_str_radix(&hex[2..4], 16) else {
        return Color::White;
    };
    let Ok(b) = u8::from_str_radix(&hex[4..6], 16) else {
        return Color::White;
    };
    Color::Rgb(r, g, b)
}
