use crate::app::{App, InputMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(10),    // Table
            Constraint::Length(3),  // Input/Message
            Constraint::Length(4),  // Help
        ])
        .split(f.area());

    draw_title(f, chunks[0], app);
    draw_table(f, app, chunks[1]);
    draw_input_or_message(f, app, chunks[2]);
    draw_help(f, app, chunks[3]);
}

fn draw_title(f: &mut Frame, area: Rect, app: &App) {
    let backend = app.storage.get_backend_name();
    let title = Paragraph::new(format!("Cron Manager [Backend: {}]", backend))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn draw_table(f: &mut Frame, app: &App, area: Rect) {
    let header_cells = ["Status", "Name", "Schedule", "Command"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = app.entries.iter().enumerate().map(|(i, entry)| {
        let status_symbol = if entry.enabled { "✓" } else { "✗" };
        let status_color = if entry.enabled { Color::Green } else { Color::Red };

        let cells = vec![
            Cell::from(status_symbol).style(Style::default().fg(status_color)),
            Cell::from(entry.name.clone()),
            Cell::from(entry.schedule.clone()),
            Cell::from(entry.command.clone()),
        ];

        let style = if i == app.selected_index {
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        Row::new(cells).style(style).height(1)
    }).collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Percentage(20),
        Constraint::Percentage(30),
        Constraint::Percentage(50),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Cron Entries ({}) ", app.entries.len()))
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        );

    f.render_widget(table, area);
}

fn draw_input_or_message(f: &mut Frame, app: &App, area: Rect) {
    let text = if app.input_mode != InputMode::Normal {
        let prompt = app.message.as_ref().map(|s| s.as_str()).unwrap_or("");
        format!("{} {}", prompt, app.input_buffer)
    } else if let Some(msg) = &app.message {
        msg.clone()
    } else {
        "Ready".to_string()
    };

    let style = if app.input_mode != InputMode::Normal {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };

    let paragraph = Paragraph::new(text)
        .style(style)
        .block(Block::default().borders(Borders::ALL).title(" Status "));
    f.render_widget(paragraph, area);
}

fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let help_text = if app.input_mode != InputMode::Normal {
        vec![
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(": Confirm | "),
                Span::styled("Esc", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::raw(": Cancel"),
            ])
        ]
    } else {
        vec![
            Line::from(vec![
                Span::styled("↑/↓", Style::default().fg(Color::Cyan)),
                Span::raw(": Navigate | "),
                Span::styled("a", Style::default().fg(Color::Green)),
                Span::raw(": Add | "),
                Span::styled("d", Style::default().fg(Color::Red)),
                Span::raw(": Delete | "),
                Span::styled("Space", Style::default().fg(Color::Yellow)),
                Span::raw(": Toggle Enable/Disable"),
            ]),
            Line::from(vec![
                Span::styled("n", Style::default().fg(Color::Cyan)),
                Span::raw(": Edit Name | "),
                Span::styled("s", Style::default().fg(Color::Cyan)),
                Span::raw(": Edit Schedule | "),
                Span::styled("c", Style::default().fg(Color::Cyan)),
                Span::raw(": Edit Command | "),
                Span::styled("q", Style::default().fg(Color::Red)),
                Span::raw(": Quit"),
            ]),
        ]
    };

    let paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title(" Controls "));
    f.render_widget(paragraph, area);
}
