use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::tui::app::App;
use crate::tui::widgets::{container_list, detail, header, statusbar};

pub fn render(f: &mut Frame, app: &App) {
    let accent = parse_hex_color(&app.config.theme.accent);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(f.area());

    header::render(f, chunks[0], app, accent);

    if app.show_help {
        render_help(f, chunks[1], accent);
    } else {
        render_main(f, chunks[1], app);
    }

    statusbar::render(f, chunks[2], app, accent);
}

fn render_main(f: &mut Frame, area: Rect, app: &App) {
    // Golden ratio: 61.8% left, 38.2% right
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(area);

    container_list::render(f, main_chunks[0], app);
    detail::render(f, main_chunks[1], app.selected_container(), &app.config);
}

fn render_help(f: &mut Frame, area: Rect, accent: Color) {
    let key_style = Style::default().fg(accent).add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::White);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Keybindings",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        help_line("  j/\u{2193}    ", "Move down", key_style, desc_style),
        help_line("  k/\u{2191}    ", "Move up", key_style, desc_style),
        help_line("  /      ", "Filter containers", key_style, desc_style),
        help_line(
            "  Esc    ",
            "Cancel filter / close help",
            key_style,
            desc_style,
        ),
        help_line("  Enter  ", "Confirm filter", key_style, desc_style),
        help_line("  ?      ", "Toggle help", key_style, desc_style),
        help_line("  q      ", "Quit", key_style, desc_style),
    ];

    let help = Paragraph::new(lines);
    f.render_widget(help, area);
}

fn help_line<'a>(key: &'a str, desc: &'a str, key_style: Style, desc_style: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(key, key_style),
        Span::styled(desc, desc_style),
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
