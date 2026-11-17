use std::path::Path;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table, Wrap,
};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::app::{App, InputMode, PreviewData};
use crate::fs_entry::{format_permissions, format_size, format_time, FileEntry};
use crate::icons::{get_icon, IconCategory};
use crate::theme::Theme;

/// Renders the entire `lsi` interface for the current frame.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let theme = app.theme();
    let area = frame.area();
    let compact = area.height < 10;

    // Paint background across the full frame.
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.bg).fg(theme.fg)),
        area,
    );

    let (header_h, footer_h) = if compact { (1, 1) } else { (3, 2) };
    let root_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_h),
            Constraint::Min(1),
            Constraint::Length(footer_h),
        ])
        .split(area);

    render_header(frame, app, &theme, root_chunks[0], compact);

    if app.show_preview && area.width >= 60 {
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(root_chunks[1]);
        render_table(frame, app, &theme, body_chunks[0], compact);
        render_preview(frame, app, &theme, body_chunks[1], compact);
    } else {
        render_table(frame, app, &theme, root_chunks[1], compact);
    }

    render_footer(frame, app, &theme, root_chunks[2], compact);

    if app.input_mode == InputMode::Help {
        render_help_modal(frame, &theme, area);
    }
}

fn render_header(frame: &mut Frame, app: &App, theme: &Theme, area: Rect, compact: bool) {
    let (dir_count, file_count) = app.visible_counts();
    let display_path = format_display_path(&app.current_dir);

    let mut spans = Vec::new();
    if app.show_icons {
        spans.push(Span::styled(
            "\u{f07c} ",
            Style::default().fg(theme.dir_fg).add_modifier(Modifier::BOLD),
        ));
    }
    spans.push(Span::styled(
        display_path,
        Style::default()
            .fg(theme.header_fg)
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(
        format!("  ({dir_count} dirs, {file_count} files)"),
        Style::default().fg(theme.muted),
    ));

    if app.show_hidden {
        spans.push(Span::styled("  [hidden]", Style::default().fg(theme.accent)));
    }

    let arrow = if app.sort_reverse { "↓" } else { "↑" };
    spans.push(Span::styled(
        format!("  [sort: {} {arrow}]", app.sort_field.as_str()),
        Style::default().fg(theme.muted),
    ));

    spans.push(Span::styled(
        format!("  [theme: {}]", app.theme_name.as_str()),
        Style::default().fg(theme.accent),
    ));

    if !app.search_query.is_empty() {
        spans.push(Span::styled(
            format!("  [filter: \"{}\"]", app.search_query),
            Style::default()
                .fg(theme.code_fg)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let header_block = if compact {
        Block::default().style(Style::default().bg(theme.header_bg).fg(theme.header_fg))
    } else {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.header_bg).fg(theme.header_fg))
    };
    let header = Paragraph::new(Line::from(spans)).block(header_block);
    frame.render_widget(header, area);
}

fn render_table(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, compact: bool) {
    let mut header_cells = Vec::new();
    let mut widths = Vec::new();

    if app.show_permissions {
        header_cells.push(Cell::from("Permissions"));
        widths.push(Constraint::Length(11));
    }
    if app.show_size {
        header_cells.push(Cell::from(Line::from("Size").right_aligned()));
        let max_size_w = app
            .visible_entries
            .iter()
            .map(|e| {
                let (num, unit) = format_size(e.size, e.is_dir);
                num.len() + unit.len()
            })
            .max()
            .unwrap_or(4)
            .max(4) as u16;
        widths.push(Constraint::Length(max_size_w));
    }
    if app.show_ownership {
        header_cells.push(Cell::from("User"));
        let max_user_w = app
            .visible_entries
            .iter()
            .map(|e| e.owner.width())
            .max()
            .unwrap_or(4)
            .clamp(4, 16) as u16;
        widths.push(Constraint::Length(max_user_w));

        header_cells.push(Cell::from("Group"));
        let max_group_w = app
            .visible_entries
            .iter()
            .map(|e| e.group.width())
            .max()
            .unwrap_or(5)
            .clamp(5, 16) as u16;
        widths.push(Constraint::Length(max_group_w));
    }
    if app.show_time {
        header_cells.push(Cell::from("Date Modified"));
        widths.push(Constraint::Length(16));
    }

    header_cells.push(Cell::from("Name"));
    widths.push(Constraint::Min(10));

    let header_row = Row::new(header_cells)
        .style(
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
        .height(1);

    let rows: Vec<Row> = app
        .visible_entries
        .iter()
        .map(|entry| build_entry_row(entry, app, theme))
        .collect();

    let empty_hint = if app.visible_entries.is_empty() {
        if app.search_query.is_empty() {
            " (empty directory) "
        } else {
            " (no matching entries) "
        }
    } else {
        ""
    };

    if compact && app.visible_entries.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            empty_hint,
            Style::default().fg(theme.muted),
        )))
        .style(Style::default().bg(theme.bg).fg(theme.fg));
        frame.render_widget(empty, area);
        return;
    }

    let block = if compact {
        Block::default().style(Style::default().bg(theme.bg).fg(theme.fg))
    } else {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(empty_hint)
            .style(Style::default().bg(theme.bg).fg(theme.fg))
    };

    let table = Table::new(rows, widths)
        .header(header_row)
        .block(block)
        .column_spacing(1)
        .row_highlight_style(
            Style::default()
                .bg(theme.selected_bg)
                .fg(theme.selected_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("❯ ");

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn build_entry_row<'a>(entry: &FileEntry, app: &App, theme: &Theme) -> Row<'a> {
    let mut cells = Vec::new();

    if app.show_permissions {
        cells.push(Cell::from(permission_line(entry, theme)));
    }
    if app.show_size {
        cells.push(Cell::from(size_line(entry, theme).right_aligned()));
    }
    if app.show_ownership {
        let user_color = if entry.uid == app.current_uid {
            theme.owner_self
        } else {
            theme.owner_other
        };
        cells.push(Cell::from(Span::styled(
            entry.owner.clone(),
            Style::default().fg(user_color),
        )));
        cells.push(Cell::from(Span::styled(
            entry.group.clone(),
            Style::default().fg(theme.group_fg),
        )));
    }
    if app.show_time {
        cells.push(Cell::from(Span::styled(
            format_time(entry.modified),
            Style::default().fg(theme.date_fg),
        )));
    }

    cells.push(Cell::from(name_line(entry, app.show_icons, theme)));
    Row::new(cells)
}

fn permission_line<'a>(entry: &FileEntry, theme: &Theme) -> Line<'a> {
    let perm = format_permissions(entry.mode, entry.is_dir, entry.is_symlink);
    let spans: Vec<Span<'a>> = perm
        .chars()
        .enumerate()
        .map(|(idx, ch)| {
            let color = match (idx, ch) {
                (0, 'd' | 'l' | 's' | 'p' | 'b' | 'c') => theme.perm_dir,
                (_, 'r') => theme.perm_read,
                (_, 'w') => theme.perm_write,
                (_, 'x') => theme.perm_exec,
                (_, 's' | 'S' | 't' | 'T') => theme.perm_special,
                _ => theme.perm_none,
            };
            Span::styled(ch.to_string(), Style::default().fg(color))
        })
        .collect();
    Line::from(spans)
}

fn size_line<'a>(entry: &FileEntry, theme: &Theme) -> Line<'a> {
    let (num, unit) = format_size(entry.size, entry.is_dir);
    if entry.is_dir {
        Line::from(Span::styled(num, Style::default().fg(theme.muted)))
    } else {
        Line::from(vec![
            Span::styled(num, Style::default().fg(theme.size_number)),
            Span::styled(unit, Style::default().fg(theme.size_unit)),
        ])
    }
}

fn name_line<'a>(entry: &FileEntry, show_icons: bool, theme: &Theme) -> Line<'a> {
    let icon = get_icon(entry, show_icons);
    let color = category_color(entry, icon.category, theme);

    let mut name_style = Style::default().fg(color);
    if entry.is_dir || entry.is_executable {
        name_style = name_style.add_modifier(Modifier::BOLD);
    }

    let mut spans = Vec::new();
    if show_icons && !icon.glyph.is_empty() {
        spans.push(Span::styled(
            format!("{} ", icon.glyph),
            Style::default().fg(color),
        ));
    }

    spans.push(Span::styled(entry.name.clone(), name_style));

    if entry.is_symlink {
        if let Some(target) = &entry.symlink_target {
            let arrow_color = if entry.symlink_broken {
                theme.broken_symlink_fg
            } else {
                theme.muted
            };
            let target_color = if entry.symlink_broken {
                theme.broken_symlink_fg
            } else {
                theme.symlink_fg
            };
            spans.push(Span::styled(" -> ", Style::default().fg(arrow_color)));
            spans.push(Span::styled(
                target.display().to_string(),
                Style::default().fg(target_color),
            ));
        }
    }

    Line::from(spans)
}

fn category_color(entry: &FileEntry, category: IconCategory, theme: &Theme) -> ratatui::style::Color {
    if entry.symlink_broken {
        return theme.broken_symlink_fg;
    }
    if entry.is_hidden && !entry.is_dir {
        return theme.hidden_fg;
    }
    match category {
        IconCategory::Directory => theme.dir_fg,
        IconCategory::Symlink => theme.symlink_fg,
        IconCategory::BrokenSymlink => theme.broken_symlink_fg,
        IconCategory::Executable => theme.exec_fg,
        IconCategory::Code => theme.code_fg,
        IconCategory::Config => theme.config_fg,
        IconCategory::Document => theme.doc_fg,
        IconCategory::Archive => theme.archive_fg,
        IconCategory::Media => theme.media_fg,
        IconCategory::Regular => theme.file_fg,
    }
}

fn render_preview(frame: &mut Frame, app: &App, theme: &Theme, area: Rect, compact: bool) {
    let title = match app.selected_entry() {
        Some(entry) => format!(" Preview: {} ", entry.name),
        None => " Preview ".to_string(),
    };

    let block = if compact {
        Block::default()
            .title(title)
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.bg).fg(theme.fg))
    } else {
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.bg).fg(theme.fg))
    };

    let lines: Vec<Line> = match &app.preview_data {
        PreviewData::Empty => vec![Line::from(Span::styled(
            "No item selected",
            Style::default().fg(theme.muted),
        ))],
        PreviewData::Directory {
            entries,
            total_count,
        } => {
            if entries.is_empty() {
                vec![Line::from(Span::styled(
                    "(empty directory)",
                    Style::default().fg(theme.muted),
                ))]
            } else {
                let mut out: Vec<Line> = entries
                    .iter()
                    .map(|child| name_line(child, app.show_icons, theme))
                    .collect();
                if *total_count > entries.len() {
                    out.push(Line::from(Span::styled(
                        format!("… and {} more items", total_count - entries.len()),
                        Style::default().fg(theme.muted),
                    )));
                }
                out
            }
        }
        PreviewData::Text { lines, truncated } => {
            if lines.is_empty() {
                vec![Line::from(Span::styled(
                    "(empty file)",
                    Style::default().fg(theme.muted),
                ))]
            } else {
                let mut out: Vec<Line> = lines
                    .iter()
                    .enumerate()
                    .map(|(idx, line)| {
                        Line::from(vec![
                            Span::styled(
                                format!("{:>3} ", idx + 1),
                                Style::default().fg(theme.muted),
                            ),
                            Span::styled(line.clone(), Style::default().fg(theme.fg)),
                        ])
                    })
                    .collect();
                if *truncated {
                    out.push(Line::from(Span::styled(
                        "… (preview truncated)",
                        Style::default().fg(theme.accent),
                    )));
                }
                out
            }
        }
        PreviewData::Binary { size } => {
            let (num, unit) = format_size(*size, false);
            vec![
                Line::from(Span::styled(
                    "Binary file",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("Size: {num}{unit} ({size} bytes)"),
                    Style::default().fg(theme.muted),
                )),
            ]
        }
        PreviewData::Error(msg) => vec![Line::from(Span::styled(
            msg.clone(),
            Style::default().fg(theme.broken_symlink_fg),
        ))],
    };

    let preview = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(preview, area);
}

fn render_footer(frame: &mut Frame, app: &App, theme: &Theme, area: Rect, compact: bool) {
    let status_line = if app.input_mode == InputMode::Search {
        Line::from(vec![
            Span::styled(
                " / ",
                Style::default()
                    .bg(theme.accent)
                    .fg(theme.selected_fg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}▌", app.search_query),
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "   (Enter: lock filter, Esc: clear)",
                Style::default().fg(theme.muted),
            ),
        ])
    } else {
        let total = app.visible_entries.len();
        let pos = match app.table_state.selected() {
            Some(idx) if total > 0 => format!("[{}/{}]", idx + 1, total),
            _ => "[0/0]".to_string(),
        };

        let mut spans = vec![Span::styled(
            format!(" {pos} "),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )];

        if let Some(msg) = &app.status_message {
            spans.push(Span::styled(msg.clone(), Style::default().fg(theme.code_fg)));
        } else if let Some(entry) = app.selected_entry() {
            let perm = format_permissions(entry.mode, entry.is_dir, entry.is_symlink);
            let (num, unit) = format_size(entry.size, entry.is_dir);
            let size_str = if entry.is_dir {
                "-".to_string()
            } else {
                format!("{num}{unit}")
            };
            let modified = format_time(entry.modified);
            spans.push(Span::styled(
                format!(
                    "{perm}  {size_str}  {}:{}  {modified}",
                    entry.owner, entry.group
                ),
                Style::default().fg(theme.footer_fg),
            ));
        }

        if compact {
            spans.push(Span::styled(
                "   q:Quit  ?:Help",
                Style::default().fg(theme.muted),
            ));
        }

        Line::from(spans)
    };

    if compact {
        let status_widget = Paragraph::new(status_line)
            .style(Style::default().bg(theme.footer_bg).fg(theme.footer_fg));
        frame.render_widget(status_widget, area);
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let status_widget =
        Paragraph::new(status_line).style(Style::default().bg(theme.footer_bg).fg(theme.footer_fg));
    frame.render_widget(status_widget, rows[0]);

    let hints = Paragraph::new(Line::from(Span::styled(
        " q:Quit  hjkl/Arrows:Nav  -/+:History  .:Hidden  p:Perm  o:Owner  t:Time  T:Theme  v:Preview  /:Filter  ?:Help",
        Style::default().fg(theme.muted),
    )))
    .style(Style::default().bg(theme.footer_bg));
    frame.render_widget(hints, rows[1]);
}

fn render_help_modal(frame: &mut Frame, theme: &Theme, area: Rect) {
    let modal_width = 72_u16
        .min(area.width.saturating_sub(4))
        .max(area.width.min(20));
    let modal_height = 22_u16
        .min(area.height.saturating_sub(2))
        .max(area.height.min(6));
    let popup_area = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, popup_area);

    let keybinds = [
        ("j / k / Down / Up", "Scroll down / up"),
        ("g / G / Home / End", "Jump to top / bottom"),
        ("Ctrl+d / Ctrl+u / PgDn / PgUp", "Page down / up"),
        ("l / Right / Enter / O", "Open folder or file"),
        ("e", "Open file in $EDITOR"),
        ("h / Left / Backspace", "Go to parent directory"),
        ("- / [ / b   and   + / ] / w", "History back / forward"),
        ("~   and   R / F5", "Jump to home / refresh"),
        (". / a", "Toggle hidden files & folders"),
        ("p / o / t / S / i", "Toggle perms / owner / time / size / icons"),
        ("L", "Toggle all long metadata columns"),
        ("T / c", "Cycle color themes"),
        ("s / r / d", "Cycle sort / reverse / dirs-first"),
        ("v / Tab", "Toggle preview pane"),
        ("/   and   Esc", "Filter entries / clear filter"),
        ("?   and   q", "Toggle help / exit lsi"),
    ];

    let lines: Vec<Line> = keybinds
        .into_iter()
        .map(|(keys, desc)| {
            Line::from(vec![
                Span::styled(
                    format!(" {keys:<30} "),
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(desc, Style::default().fg(theme.fg)),
            ])
        })
        .collect();

    let modal = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" lsi Keybinds (press ? or Esc to close, q to quit) ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent))
                .style(Style::default().bg(theme.header_bg).fg(theme.fg)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(modal, popup_area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}

fn format_display_path(path: &Path) -> String {
    if let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
        if let Ok(rest) = path.strip_prefix(&home) {
            if rest.as_os_str().is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", rest.display());
        }
    }
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use clap::Parser;
    use crossterm::event::{KeyCode, KeyEvent};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::fs;

    #[test]
    fn renders_table_preview_and_help_modal_without_panicking() {
        let base = std::env::temp_dir().join(format!("lsi_ui_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("src")).unwrap();
        fs::write(base.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
        fs::write(base.join("src/main.rs"), "fn main() { println!(\"hi\"); }\n").unwrap();

        let args = Args::try_parse_from([
            "lsi",
            "-l",
            "--preview",
            "--theme",
            "catppuccin",
            base.to_str().unwrap(),
        ])
        .unwrap();
        let mut app = App::new(args).unwrap();

        let backend = TestBackend::new(120, 35);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let buf_str = buffer_to_string(&terminal);
        assert!(buf_str.contains("Permissions"));
        assert!(buf_str.contains("Date Modified"));
        assert!(buf_str.contains("Cargo.toml"));
        assert!(buf_str.contains("Preview:"));

        app.handle_key(KeyEvent::from(KeyCode::Char('?')));
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let help_str = buffer_to_string(&terminal);
        assert!(help_str.contains("lsi Keybinds"));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn renders_inline_viewport_standard_and_compact_heights() {
        use ratatui::backend::Backend;
        use ratatui::layout::Position;
        use ratatui::{TerminalOptions, Viewport};

        let base = std::env::temp_dir().join(format!("lsi_inline_ui_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("src")).unwrap();
        fs::write(base.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();

        // Standard inline viewport (height >= 10) anchored at non-zero cursor row y = 5
        let args = Args::try_parse_from(["lsi", "-n", "12", base.to_str().unwrap()]).unwrap();
        let mut app = App::new(args).unwrap();
        let height = app.desired_inline_height();
        assert_eq!(height, 10);

        let mut backend = TestBackend::new(100, 30);
        backend.set_cursor_position(Position::new(0, 5)).unwrap();
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(height),
            },
        )
        .unwrap();
        assert_eq!(terminal.get_frame().area(), Rect::new(0, 5, 100, 10));

        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let out = buffer_to_string(&terminal);
        assert!(out.contains("Cargo.toml"));
        assert!(out.contains("src"));

        // Compact inline viewport (height < 10) anchored at non-zero cursor row y = 4
        let compact_args =
            Args::try_parse_from(["lsi", "-n", "7", base.to_str().unwrap()]).unwrap();
        let mut compact_app = App::new(compact_args).unwrap();
        let compact_h = compact_app.desired_inline_height();
        assert_eq!(compact_h, 6);

        let mut compact_backend = TestBackend::new(100, 20);
        compact_backend
            .set_cursor_position(Position::new(0, 4))
            .unwrap();
        let mut compact_terminal = Terminal::with_options(
            compact_backend,
            TerminalOptions {
                viewport: Viewport::Inline(compact_h),
            },
        )
        .unwrap();
        assert_eq!(
            compact_terminal.get_frame().area(),
            Rect::new(0, 4, 100, 6)
        );

        compact_terminal
            .draw(|frame| draw(frame, &mut compact_app))
            .unwrap();
        let compact_out = buffer_to_string(&compact_terminal);
        assert!(compact_out.contains("Cargo.toml"));
        assert!(compact_out.contains("?:Help"));

        let _ = fs::remove_dir_all(&base);
    }

    fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
        let buf = terminal.backend().buffer();
        let mut s = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }
}
