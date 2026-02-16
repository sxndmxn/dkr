use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::config::Config;
use crate::docker::models::{ContainerInfo, ContainerStatus, HealthStatus, format_bytes};

pub fn render(f: &mut Frame, area: Rect, container: Option<&ContainerInfo>, config: &Config) {
    let Some(c) = container else {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No container selected",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(empty, area);
        return;
    };

    let theme = &config.theme;
    let healthy_color = parse_hex_color(&theme.healthy);
    let warning_color = parse_hex_color(&theme.warning);
    let critical_color = parse_hex_color(&theme.critical);

    let sc = match (&c.status, &c.health) {
        (ContainerStatus::Running, HealthStatus::Healthy | HealthStatus::None) => healthy_color,
        (ContainerStatus::Running, HealthStatus::Starting)
        | (ContainerStatus::Restarting | ContainerStatus::Paused, _) => warning_color,
        (ContainerStatus::Running, HealthStatus::Unhealthy)
        | (ContainerStatus::Exited(_) | ContainerStatus::Dead, _) => critical_color,
        (ContainerStatus::Created | ContainerStatus::Removing | ContainerStatus::Unknown(_), _) => {
            Color::DarkGray
        }
    };

    let mem_color = if c.memory_percent() >= config.alerts.memory_warn_percent {
        warning_color
    } else {
        Color::White
    };

    let cpu_color = if c.cpu_percent >= config.alerts.cpu_warn_percent {
        warning_color
    } else {
        Color::White
    };

    let label_style = Style::default().fg(Color::DarkGray);
    let value_style = Style::default().fg(Color::White);

    let status_str = format!("{} ({})", c.status_display(), c.health_display());
    let cpu_str = format!("{:.1}%", c.cpu_percent);
    let mem_str = format!(
        "{} / {} ({:.0}%)",
        format_bytes(c.memory_usage),
        format_bytes(c.memory_limit),
        c.memory_percent()
    );
    let ports_str = c.ports_display();
    let uptime_str = c.uptime_string();
    let short_id = if c.id.len() > 12 { &c.id[..12] } else { &c.id };

    let mut lines = vec![
        Line::from(Span::styled(
            format!("Container: {}", c.name),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        detail_line("Image", &c.image, label_style, value_style),
        detail_line("Status", &status_str, label_style, Style::default().fg(sc)),
        detail_line("CPU", &cpu_str, label_style, Style::default().fg(cpu_color)),
        detail_line(
            "Memory",
            &mem_str,
            label_style,
            Style::default().fg(mem_color),
        ),
        detail_line("Ports", &ports_str, label_style, value_style),
        detail_line("Uptime", &uptime_str, label_style, value_style),
        detail_line(
            "Health",
            c.health_display(),
            label_style,
            Style::default().fg(sc),
        ),
    ];

    if let Some(project) = &c.compose_project {
        lines.push(detail_line("Project", project, label_style, value_style));
    }

    lines.push(detail_line(
        "ID",
        short_id,
        label_style,
        Style::default().fg(Color::DarkGray),
    ));

    let detail = Paragraph::new(lines);
    f.render_widget(detail, area);
}

fn detail_line<'a>(
    label: &'a str,
    value: &'a str,
    label_style: Style,
    value_style: Style,
) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {label:<9}"), label_style),
        Span::styled(value, value_style),
    ])
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
