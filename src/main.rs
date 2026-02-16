mod cli;
mod config;
mod display;
mod docker;
mod error;
mod tui;

use std::time::Duration;

use clap::Parser as _;
use crossterm::event::{KeyCode, KeyModifiers};

use crate::cli::{Cli, Command, ConfigAction};
use crate::config::{default_config_path, load_config, resolve_host};
use crate::docker::client;
use crate::tui::app::{App, InputMode};
use crate::tui::event::{Event, EventHandler};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "warn" };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let config = load_config(cli.config.as_deref())?;
    let host = resolve_host(cli.host.as_deref(), &config);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    match cli.command {
        Some(Command::Status { json, watch, name }) => {
            runtime.block_on(run_status(&host, &config, json, watch, name.as_deref()))?;
        }
        Some(Command::Config { action }) => {
            run_config_command(&action);
        }
        None => {
            runtime.block_on(run_tui(host, config))?;
        }
    }

    Ok(())
}

async fn run_status(
    host: &str,
    config: &config::Config,
    json: bool,
    watch: bool,
    name: Option<&str>,
) -> anyhow::Result<()> {
    let docker = client::connect(host)?;
    client::ping(&docker).await?;

    if let Some(name) = name {
        let container = docker::fetch::fetch_one(&docker, name).await?;
        if json {
            display::json::print_single_json(&container);
        } else {
            display::table::print_single(&container, config);
        }
        return Ok(());
    }

    if watch {
        run_watch(&docker, config, json).await?;
    } else {
        let containers = docker::fetch::fetch_all(&docker, config.display.show_stopped).await?;
        if json {
            display::json::print_json(&containers);
        } else {
            display::table::print_table(&containers, config);
        }
    }

    Ok(())
}

async fn run_watch(
    docker: &bollard::Docker,
    config: &config::Config,
    json: bool,
) -> anyhow::Result<()> {
    let interval = Duration::from_millis(config.display.refresh_interval);

    loop {
        print!("\x1B[2J\x1B[H");

        let containers = docker::fetch::fetch_all(docker, config.display.show_stopped).await?;
        if json {
            display::json::print_json(&containers);
        } else {
            display::table::print_table(&containers, config);
        }

        tokio::time::sleep(interval).await;

        if crossterm::event::poll(Duration::ZERO)?
            && let crossterm::event::Event::Key(key) = crossterm::event::read()?
            && (key.code == KeyCode::Char('q')
                || (key.code == KeyCode::Char('c')
                    && key.modifiers.contains(KeyModifiers::CONTROL)))
        {
            break;
        }
    }

    Ok(())
}

fn run_config_command(action: &ConfigAction) {
    match action {
        ConfigAction::Show => {
            let config = load_config(None).unwrap_or_default();
            println!("{config:#?}");
        }
        ConfigAction::Path => {
            println!("{}", default_config_path().display());
        }
    }
}

async fn run_tui(host: String, config: config::Config) -> anyhow::Result<()> {
    let docker = client::connect(&host)?;
    client::ping(&docker).await?;

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(config.display.refresh_interval);
    let show_stopped = config.display.show_stopped;
    let mut app = App::new(config, host);
    let mut events = EventHandler::new(tick_rate);

    match docker::fetch::fetch_all(&docker, show_stopped).await {
        Ok(containers) => app.update_containers(containers),
        Err(e) => app.last_error = Some(format!("{e}")),
    }

    while app.running {
        terminal.draw(|f| tui::ui::render(f, &app))?;

        match events.next().await {
            Some(Event::Key(key)) => handle_key(&mut app, key),
            Some(Event::Tick) => {
                match docker::fetch::fetch_all(&docker, app.config.display.show_stopped).await {
                    Ok(containers) => {
                        app.last_error = None;
                        app.update_containers(containers);
                    }
                    Err(e) => app.last_error = Some(format!("{e}")),
                }
            }
            None => app.running = false,
        }
    }

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn handle_key(app: &mut App, key: crossterm::event::KeyEvent) {
    match app.input_mode {
        InputMode::Normal => handle_normal_key(app, key),
        InputMode::Filter => handle_filter_key(app, key),
    }
}

#[allow(
    clippy::wildcard_enum_match_arm,
    reason = "KeyCode has many irrelevant variants"
)]
fn handle_normal_key(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.running = false,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.running = false;
        }
        KeyCode::Char('j') | KeyCode::Down => app.select_next(),
        KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
        KeyCode::Char('/') => app.start_filter(),
        KeyCode::Char('?') => app.toggle_help(),
        KeyCode::Esc => {
            if app.show_help {
                app.show_help = false;
            }
        }
        _ => {}
    }
}

#[allow(
    clippy::wildcard_enum_match_arm,
    reason = "KeyCode has many irrelevant variants"
)]
fn handle_filter_key(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Enter => app.confirm_filter(),
        KeyCode::Esc => app.cancel_filter(),
        KeyCode::Backspace => app.filter_pop(),
        KeyCode::Char(c) => app.filter_push(c),
        _ => {}
    }
}
