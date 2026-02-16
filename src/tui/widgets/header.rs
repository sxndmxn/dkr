use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::app::App;

pub fn render(f: &mut Frame, area: Rect, app: &App, accent: Color) {
    let title_spans = vec![
        Span::styled(
            " dkr",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" — ", Style::default().fg(Color::DarkGray)),
        Span::styled(&app.host, Style::default().fg(Color::White)),
    ];

    let count_text = format!("{} containers ", app.container_count());
    let count_span = Span::styled(count_text, Style::default().fg(Color::DarkGray));

    let header = Paragraph::new(Line::from(title_spans)).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray))
            .title_bottom(Line::from(count_span).right_aligned()),
    );

    f.render_widget(header, area);
}
