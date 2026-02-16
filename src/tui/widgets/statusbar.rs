use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::app::{App, InputMode};

pub fn render(f: &mut Frame, area: Rect, app: &App, accent: Color) {
    let content = match app.input_mode {
        InputMode::Normal => {
            let bindings = vec![
                Span::styled(
                    " q",
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" quit", Style::default().fg(Color::DarkGray)),
                Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "/",
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" filter", Style::default().fg(Color::DarkGray)),
                Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "?",
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" help", Style::default().fg(Color::DarkGray)),
            ];

            if let Some(err) = &app.last_error {
                let mut spans = vec![Span::styled(
                    format!(" Error: {err} "),
                    Style::default().fg(Color::Red),
                )];
                spans.extend(bindings);
                Line::from(spans)
            } else {
                Line::from(bindings)
            }
        }
        InputMode::Filter => {
            let spans = vec![
                Span::styled(" /", Style::default().fg(accent)),
                Span::styled(&app.filter, Style::default().fg(Color::White)),
                Span::styled(
                    "▎",
                    Style::default()
                        .fg(accent)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
                Span::styled(
                    " (Enter confirm, Esc cancel)",
                    Style::default().fg(Color::DarkGray),
                ),
            ];
            Line::from(spans)
        }
    };

    let bar = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(bar, area);
}
