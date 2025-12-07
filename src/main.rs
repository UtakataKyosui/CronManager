mod app;
mod cron_entry;
mod cron_parser;
mod storage;
mod ui;

use anyhow::Result;
use app::{App, InputMode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use storage::Storage;

fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let storage = if args.len() > 1 && args[1] == "--system" {
        Storage::with_system_crontab()
    } else {
        Storage::new(None)
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new(storage)?;
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => {
                            app.quit();
                            break;
                        }
                        KeyCode::Up | KeyCode::Char('k') => app.move_selection_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.move_selection_down(),
                        KeyCode::Char('a') => app.start_add_entry(),
                        KeyCode::Char('d') => app.delete_entry()?,
                        KeyCode::Char('n') => app.start_edit_name(),
                        KeyCode::Char('s') => app.start_edit_schedule(),
                        KeyCode::Char('c') => app.start_edit_command(),
                        KeyCode::Char(' ') => app.toggle_enabled()?,
                        _ => {}
                    },
                    _ => match key.code {
                        KeyCode::Enter => app.confirm_input()?,
                        KeyCode::Char(c) => app.handle_input_char(c),
                        KeyCode::Backspace => app.handle_input_backspace(),
                        KeyCode::Esc => app.cancel_input(),
                        _ => {}
                    },
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
