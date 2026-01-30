#![allow(clippy::multiple_crate_versions)]

mod app;
mod handlers;
mod state;
mod ui;

use anyhow::{Context, Result};
use app::App;
use beeconfig::BeeConfig;
use beeminder::BeeminderClient;
use crossterm::cursor::Show;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use handlers::handle_key;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use state::{StatusKind, TICK_RATE};
use std::io::{self, Stdout};
use tokio::runtime::Runtime;
use ui::render_app;

fn main() -> Result<()> {
    let config = BeeConfig::load_or_onboard().with_context(|| "Failed to load beeminder config")?;
    let api_key = config
        .api_key()
        .with_context(|| "Missing api_key in beeminder config")?;

    let client = if let Some(user) = config.default_user.as_ref() {
        BeeminderClient::new(api_key).with_username(user)
    } else {
        BeeminderClient::new(api_key)
    };

    let runtime = Runtime::new().context("Failed to start tokio runtime")?;
    let mut app = App::new(config, client);

    let (mut terminal, _guard) = init_terminal()?;

    if app.config.tui.refresh_on_start {
        if let Err(err) = app.refresh_goals(&runtime) {
            app.set_status(StatusKind::Error, err.to_string());
        }
    } else {
        app.set_status(StatusKind::Info, "Press r to load goals".to_string());
    }

    run_app(&mut terminal, &mut app, &runtime)
}

fn init_terminal() -> Result<(Terminal<CrosstermBackend<Stdout>>, TerminalGuard)> {
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    Ok((terminal, TerminalGuard))
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    runtime: &Runtime,
) -> Result<()> {
    loop {
        app.clear_expired_status();
        terminal.draw(|f| render_app(f, app))?;

        if event::poll(TICK_RATE)? {
            if let Event::Key(key) = event::read()? {
                if handle_key(app, key, runtime) {
                    return Ok(());
                }
            }
        }
    }
}
