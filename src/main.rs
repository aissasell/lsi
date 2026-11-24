mod app;
mod cli;
mod config;
mod fs_entry;
mod fs_ops;
mod icons;
mod keybinds;
mod preview;
mod theme;
mod ui;
mod video_preview;

use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::cursor::Show;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Position;
use ratatui::{Terminal, TerminalOptions, Viewport};

use crate::app::{App, AppAction};
use crate::cli::Args;
use crate::preview::clear_terminal_graphics;

type InlineTerminal = Terminal<CrosstermBackend<io::Stdout>>;

fn main() -> Result<()> {
    let args = Args::parse();
    let cwd_file = args.cwd_file.clone();
    let mut app = App::new(args)?;

    install_panic_hook();
    enable_terminal_modes()?;

    let run_result = run_event_loop(&mut app);

    if let Err(err) = restore_terminal_modes() {
        eprintln!("Failed to restore terminal: {err}");
    }

    match run_result {
        Ok(change_dir) => {
            println!();
            app.write_cwd_file(cwd_file.as_deref(), change_dir)?;
            Ok(())
        }
        Err(err) => Err(err),
    }
}

fn run_event_loop(app: &mut App) -> Result<bool> {
    let mut current_height = app.desired_inline_height();
    let mut terminal = init_inline_terminal(current_height)?;

    loop {
        app.ensure_image_picker_ready();
        let _ = app.poll_preview_results();
        if app.take_pending_graphics_clear() {
            clear_terminal_graphics(&mut io::stdout(), app.uses_kitty_graphics())?;
        }

        let target_height = app.desired_inline_height();
        if target_height != current_height {
            if app.uses_kitty_graphics() {
                clear_terminal_graphics(&mut io::stdout(), true)?;
                app.handle_graphics_cleared();
            }
            let origin = terminal.get_frame().area().as_position();
            terminal.clear()?;
            terminal.set_cursor_position(origin)?;
            terminal = init_inline_terminal(target_height)?;
            current_height = target_height;
        }

        terminal.draw(|frame| ui::draw(frame, app))?;

        if app.preview_in_flight && !event::poll(Duration::from_millis(16))? {
            continue;
        }

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match app.handle_key(key) {
                AppAction::Quit { change_dir } => {
                    if app.uses_kitty_graphics() {
                        clear_terminal_graphics(&mut io::stdout(), true)?;
                    }
                    finalize_inline_exit(&mut terminal)?;
                    return Ok(change_dir);
                }
                AppAction::OpenEditor(path) => {
                    if app.uses_kitty_graphics() {
                        clear_terminal_graphics(&mut io::stdout(), true)?;
                        app.handle_graphics_cleared();
                    }
                    let origin = terminal.get_frame().area().as_position();
                    terminal.clear()?;
                    terminal.set_cursor_position(origin)?;
                    restore_terminal_modes()?;

                    let editor_result = launch_editor(&path);

                    enable_terminal_modes()?;
                    if let Err(err) = editor_result {
                        app.status_message = Some(format!("Editor error: {err}"));
                    } else if let Err(err) = app.load_directory(None) {
                        app.status_message = Some(format!("Reload error: {err}"));
                    } else {
                        app.status_message = Some(format!(
                            "Edited {}",
                            path.file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| path.display().to_string())
                        ));
                    }

                    current_height = app.desired_inline_height();
                    terminal = init_inline_terminal(current_height)?;
                }
                AppAction::None => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => app.move_selection(-1),
                MouseEventKind::ScrollDown => app.move_selection(1),
                _ => {}
            },
            Event::Resize(_, _) => {}
            _ => {}
        }
    }
}

fn init_inline_terminal(height: u16) -> io::Result<InlineTerminal> {
    Terminal::with_options(
        CrosstermBackend::new(io::stdout()),
        TerminalOptions {
            viewport: Viewport::Inline(height),
        },
    )
}

fn enable_terminal_modes() -> io::Result<()> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnableMouseCapture)?;
    Ok(())
}

fn restore_terminal_modes() -> io::Result<()> {
    let mouse_res = execute!(io::stdout(), DisableMouseCapture, Show);
    let raw_res = disable_raw_mode();
    mouse_res.and(raw_res)
}

fn finalize_inline_exit(terminal: &mut InlineTerminal) -> io::Result<()> {
    let area = terminal.get_frame().area();
    let last_row = area.bottom().saturating_sub(1);
    terminal.set_cursor_position(Position::new(0, last_row))?;
    terminal.show_cursor()?;
    terminal.backend_mut().flush()?;
    Ok(())
}

fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal_modes();
        default_hook(info);
    }));
}

fn launch_editor(path: &Path) -> io::Result<()> {
    let configured = std::env::var("VISUAL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("EDITOR").ok().filter(|s| !s.trim().is_empty()));

    if let Some(editor_cmd) = configured {
        let mut parts = editor_cmd.split_whitespace();
        if let Some(program) = parts.next() {
            let status = Command::new(program).args(parts).arg(path).status()?;
            if !status.success() {
                return Err(io::Error::other(format!(
                    "{program} exited with status {status}"
                )));
            }
            return Ok(());
        }
    }

    for fallback in ["vi", "nano"] {
        match Command::new(fallback).arg(path).status() {
            Ok(status) => {
                if !status.success() {
                    return Err(io::Error::other(format!(
                        "{fallback} exited with status {status}"
                    )));
                }
                return Ok(());
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => return Err(err),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "No editor found ($VISUAL, $EDITOR, vi, or nano)",
    ))
}
