use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table, Wrap};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::app::{App, ClipboardMode, InputMode};
use crate::fs_entry::{format_permissions, format_size, format_time, FileEntry};
use crate::icons::{get_icon, IconCategory};
use crate::keybinds::Keybinds;
use crate::preview::PreviewData;
use crate::theme::Theme;

/// Renders the entire `lsi` interface for the current frame.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let theme = app.theme();
    let area = frame.area();
    let compact = area.height < 6;
    let show_borders = app.show_borders && !compact;

    // Paint background across the full frame.
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.bg).fg(theme.fg)),
        area,
    );

    let footer_h = if app.show_footer {
        if compact {
            1
        } else {
            2
        }
    } else if matches!(
        app.input_mode,
        InputMode::Search | InputMode::ConfirmDelete | InputMode::CreateItem | InputMode::RenameItem
    ) {
        1
    } else {
        0
    };

    let (body_area, footer_area) = if footer_h > 0 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(footer_h)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    if app.show_preview && area.width >= 60 {
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(body_area);
        let inner_w = if show_borders {
            body_chunks[1].width.saturating_sub(2)
        } else {
            body_chunks[1].width.saturating_sub(1)
        };
        let inner_h = if show_borders {
            body_chunks[1].height.saturating_sub(2)
        } else {
            body_chunks[1].height
        };
        app.set_preview_area(inner_w, inner_h);
        render_table(frame, app, &theme, body_chunks[0], show_borders);
        render_preview(frame, app, &theme, body_chunks[1], show_borders);
    } else {
        render_table(frame, app, &theme, body_area, show_borders);
    }

    if let Some(footer_rect) = footer_area {
        render_footer(frame, app, &theme, footer_rect, footer_h == 1);
    }

    if app.input_mode == InputMode::Help {
        render_help_modal(frame, &theme, &app.keybinds, area);
    }
}

fn render_table(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    show_borders: bool,
) {
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

    if !show_borders && app.visible_entries.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            empty_hint,
            Style::default().fg(theme.muted),
        )))
        .style(Style::default().bg(theme.bg).fg(theme.fg));
        frame.render_widget(empty, area);
        return;
    }

    let block = if show_borders {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(empty_hint)
            .style(Style::default().bg(theme.bg).fg(theme.fg))
    } else {
        Block::default().style(Style::default().bg(theme.bg).fg(theme.fg))
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

    let is_selected = app.selected_paths.contains(&entry.path);
    let clip_mode = app
        .clipboard
        .as_ref()
        .filter(|clip| clip.paths.iter().any(|p| p == &entry.path))
        .map(|clip| clip.mode);

    cells.push(Cell::from(name_line_marked(
        entry,
        app.show_icons,
        is_selected,
        clip_mode,
        theme,
    )));
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
    name_line_marked(entry, show_icons, false, None, theme)
}

fn name_line_marked<'a>(
    entry: &FileEntry,
    show_icons: bool,
    is_selected: bool,
    clip_mode: Option<ClipboardMode>,
    theme: &Theme,
) -> Line<'a> {
    let icon = get_icon(entry, show_icons);
    let color = category_color(entry, icon.category, theme);

    let mut name_style = Style::default().fg(color);
    if entry.is_dir || entry.is_executable {
        name_style = name_style.add_modifier(Modifier::BOLD);
    }

    let mut spans = Vec::new();
    if is_selected {
        spans.push(Span::styled(
            "● ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
    }
    match clip_mode {
        Some(ClipboardMode::Cut) => {
            spans.push(Span::styled(
                "✂ ",
                Style::default().fg(theme.broken_symlink_fg),
            ));
        }
        Some(ClipboardMode::Copy) => {
            spans.push(Span::styled("+ ", Style::default().fg(theme.exec_fg)));
        }
        None => {}
    }

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

fn category_color(
    entry: &FileEntry,
    category: IconCategory,
    theme: &Theme,
) -> ratatui::style::Color {
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

fn render_preview(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    show_borders: bool,
) {
    let block = if show_borders {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.bg).fg(theme.fg))
    } else {
        Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.bg).fg(theme.fg))
    };

    #[cfg(feature = "image-preview")]
    if let PreviewData::Image { protocol, .. } = &app.preview_data {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        if protocol.needs_placeholder(inner).is_some() {
            let notice = Paragraph::new(Line::from(Span::styled(
                "Resizing image…",
                Style::default().fg(theme.muted),
            )));
            frame.render_widget(notice, inner);
        } else {
            frame.render_widget(
                ratatui_image::Image::new(protocol).allow_clipping(true),
                inner,
            );
        }
        return;
    }

    let lines: Vec<Line> = match &app.preview_data {
        PreviewData::Empty => vec![Line::from(Span::styled(
            if app.should_show_preview_loading() {
                "Loading preview…"
            } else {
                "No item selected"
            },
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
        PreviewData::Text {
            lines, truncated, ..
        } => {
            if lines.is_empty() {
                vec![Line::from(Span::styled(
                    "(empty file)",
                    Style::default().fg(theme.muted),
                ))]
            } else {
                let mut out: Vec<Line> = lines
                    .iter()
                    .enumerate()
                    .map(|(idx, segments)| {
                        let mut spans = Vec::with_capacity(segments.len() + 1);
                        spans.push(Span::styled(
                            format!("{:>3} ", idx + 1),
                            Style::default().fg(theme.muted),
                        ));
                        for seg in segments {
                            let mut style = Style::default().fg(seg.fg.unwrap_or(theme.fg));
                            if !seg.modifiers.is_empty() {
                                style = style.add_modifier(seg.modifiers);
                            }
                            spans.push(Span::styled(seg.text.clone(), style));
                        }
                        Line::from(spans)
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
        #[cfg(feature = "image-preview")]
        PreviewData::Image { .. } => unreachable!("handled cleanly above"),
        PreviewData::Binary { size, detail } => {
            let (num, unit) = format_size(*size, false);
            let mut out = vec![
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
            ];
            if let Some(extra) = detail {
                out.push(Line::from(Span::styled(
                    extra.clone(),
                    Style::default().fg(theme.code_fg),
                )));
            }
            out
        }
        PreviewData::Error(msg) => vec![Line::from(Span::styled(
            msg.clone(),
            Style::default().fg(theme.broken_symlink_fg),
        ))],
    };

    let preview = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.preview_scroll, 0));
    frame.render_widget(preview, area);
}

fn render_footer(frame: &mut Frame, app: &App, theme: &Theme, area: Rect, compact: bool) {
    let kb = &app.keybinds;
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
                format!(
                    "   ({}: lock filter, {}: clear)",
                    Keybinds::primary_short(&kb.search_confirm),
                    Keybinds::primary_short(&kb.search_cancel),
                ),
                Style::default().fg(theme.muted),
            ),
        ])
    } else if app.input_mode == InputMode::CreateItem {
        let kind_hint = if app.create_input.trim_end().ends_with('/') {
            "folder"
        } else {
            "file (end with / for folder)"
        };
        Line::from(vec![
            Span::styled(
                " NEW ",
                Style::default()
                    .bg(theme.accent)
                    .fg(theme.selected_fg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}▌", app.create_input),
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "   [{kind_hint}]  ({}: create, {}: cancel)",
                    Keybinds::primary_short(&kb.create_confirm),
                    Keybinds::primary_short(&kb.create_cancel),
                ),
                Style::default().fg(theme.muted),
            ),
        ])
    } else if app.input_mode == InputMode::RenameItem {
        let (before, after) = app.split_rename_input();
        Line::from(vec![
            Span::styled(
                " RENAME ",
                Style::default()
                    .bg(theme.accent)
                    .fg(theme.selected_fg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {before}"),
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "▌",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                after.to_string(),
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "   ({}: rename, {}: cancel)",
                    Keybinds::primary_short(&kb.rename_confirm),
                    Keybinds::primary_short(&kb.rename_cancel),
                ),
                Style::default().fg(theme.muted),
            ),
        ])
    } else if app.input_mode == InputMode::ConfirmDelete {
        let target_desc = if app.pending_delete.len() == 1 {
            app.pending_delete[0]
                .file_name()
                .map(|n| format!("'{}'", n.to_string_lossy()))
                .unwrap_or_else(|| "1 item".to_string())
        } else {
            format!("{} items", app.pending_delete.len())
        };
        let confirm_key = Keybinds::primary_short(&kb.confirm_delete);
        let cancel_key = Keybinds::primary_short(&kb.cancel_delete);
        let prompt_keys = if confirm_key == "y" && cancel_key == "n" {
            "(y/N)".to_string()
        } else {
            format!("({confirm_key}/{cancel_key})")
        };
        Line::from(vec![
            Span::styled(
                " DELETE ",
                Style::default()
                    .bg(theme.broken_symlink_fg)
                    .fg(theme.selected_fg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" Permanently delete {target_desc}? {prompt_keys}"),
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
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

        if !app.selected_paths.is_empty() {
            spans.push(Span::styled(
                format!("[{} sel] ", app.selected_paths.len()),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        if let Some(clip) = &app.clipboard {
            let (label, color) = match clip.mode {
                ClipboardMode::Copy => (format!("[{} yanked] ", clip.paths.len()), theme.exec_fg),
                ClipboardMode::Cut => (
                    format!("[{} cut] ", clip.paths.len()),
                    theme.broken_symlink_fg,
                ),
            };
            spans.push(Span::styled(label, Style::default().fg(color)));
        }

        if let Some(msg) = &app.status_message {
            spans.push(Span::styled(
                msg.clone(),
                Style::default().fg(theme.code_fg),
            ));
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
                format!(
                    "   {}:Quit(cd)  {}/{}:Quit  {}:Help",
                    Keybinds::primary_short(&kb.quit_cd),
                    Keybinds::primary_short(&kb.quit),
                    Keybinds::primary_short(&kb.cancel),
                    Keybinds::primary_short(&kb.toggle_help),
                ),
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

    let hints_text = format!(
        " {}:Quit(cd)  {}/{}:Quit  {}:New  {}:Rename  {}:Sel  {}/{}/{}:Copy/Cut/Paste  {}:Trash  {}:Perm  {}:Hidden  {}:Theme  {}:Help",
        Keybinds::primary_short(&kb.quit_cd),
        Keybinds::primary_short(&kb.quit),
        Keybinds::primary_short(&kb.cancel),
        Keybinds::primary_short(&kb.create_item),
        Keybinds::primary_short(&kb.rename_item),
        Keybinds::primary_short(&kb.toggle_select),
        Keybinds::primary_short(&kb.yank),
        Keybinds::primary_short(&kb.cut),
        Keybinds::primary_short(&kb.paste),
        Keybinds::primary_short(&kb.trash),
        Keybinds::primary_short(&kb.toggle_permissions),
        Keybinds::primary_short(&kb.toggle_hidden),
        Keybinds::primary_short(&kb.cycle_theme),
        Keybinds::primary_short(&kb.toggle_help),
    );
    let hints = Paragraph::new(Line::from(Span::styled(
        hints_text,
        Style::default().fg(theme.muted),
    )))
    .style(Style::default().bg(theme.footer_bg));
    frame.render_widget(hints, rows[1]);
}

fn render_help_modal(frame: &mut Frame, theme: &Theme, kb: &Keybinds, area: Rect) {
    let modal_width = 72_u16
        .min(area.width.saturating_sub(4))
        .max(area.width.min(20));
    let modal_height = 29_u16
        .min(area.height.saturating_sub(2))
        .max(area.height.min(6));
    let popup_area = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, popup_area);

    let keybinds = [
        (
            format!(
                "{} / {}",
                Keybinds::format_short(&kb.move_down),
                Keybinds::format_short(&kb.move_up),
            ),
            "Scroll down / up",
        ),
        (
            format!(
                "{} / {}",
                Keybinds::format_short(&kb.jump_top),
                Keybinds::format_short(&kb.jump_bottom),
            ),
            "Jump to top / bottom",
        ),
        (
            format!(
                "{} / {}",
                Keybinds::format_short(&kb.page_down),
                Keybinds::format_short(&kb.page_up),
            ),
            "Page down / up",
        ),
        (
            Keybinds::format_short(&kb.open),
            "Open folder or file in $EDITOR",
        ),
        (
            Keybinds::format_short(&kb.open_system),
            "Open file with system opener (xdg-open)",
        ),
        (
            Keybinds::format_short(&kb.open_editor),
            "Open file in $EDITOR",
        ),
        (
            Keybinds::format_short(&kb.go_parent),
            "Go to parent directory",
        ),
        (
            format!(
                "{}   and   {}",
                Keybinds::format_short(&kb.history_back),
                Keybinds::format_short(&kb.history_forward),
            ),
            "History back / forward",
        ),
        (
            format!(
                "{}   and   {}",
                Keybinds::format_short(&kb.jump_home),
                Keybinds::format_short(&kb.refresh),
            ),
            "Jump to home / refresh",
        ),
        (
            Keybinds::format_short(&kb.toggle_select),
            "Toggle selection on current item",
        ),
        (
            format!(
                "{} / {} / {}",
                Keybinds::format_short(&kb.yank),
                Keybinds::format_short(&kb.cut),
                Keybinds::format_short(&kb.paste),
            ),
            "Yank (copy) / cut / paste items",
        ),
        (
            format!(
                "{} / {}",
                Keybinds::format_short(&kb.trash),
                Keybinds::format_short(&kb.permanent_delete),
            ),
            "Move to Trash / permanently delete",
        ),
        (
            Keybinds::format_short(&kb.create_item),
            "Create new file or folder (end with /)",
        ),
        (
            Keybinds::format_short(&kb.rename_item),
            "Rename selected file or folder",
        ),
        (
            Keybinds::format_short(&kb.toggle_hidden),
            "Toggle hidden files & folders",
        ),
        (
            format!(
                "{} / {} / {} / {} / {}",
                Keybinds::primary_short(&kb.toggle_permissions),
                Keybinds::primary_short(&kb.toggle_ownership),
                Keybinds::primary_short(&kb.toggle_time),
                Keybinds::primary_short(&kb.toggle_size),
                Keybinds::primary_short(&kb.toggle_icons),
            ),
            "Toggle perms / owner / time / size / icons",
        ),
        (
            Keybinds::format_short(&kb.toggle_long),
            "Toggle all long metadata columns",
        ),
        (
            format!(
                "{} / {}",
                Keybinds::format_short(&kb.toggle_borders),
                Keybinds::format_short(&kb.toggle_footer),
            ),
            "Toggle borders / footer",
        ),
        (
            Keybinds::format_short(&kb.cycle_theme),
            "Cycle color themes",
        ),
        (
            format!(
                "{} / {} / {}",
                Keybinds::format_short(&kb.cycle_sort),
                Keybinds::format_short(&kb.toggle_reverse_sort),
                Keybinds::format_short(&kb.toggle_dirs_first),
            ),
            "Cycle sort / reverse / dirs-first",
        ),
        (
            Keybinds::format_short(&kb.toggle_preview),
            "Toggle preview pane",
        ),
        (
            format!(
                "{} / {}",
                Keybinds::format_short(&kb.preview_scroll_down),
                Keybinds::format_short(&kb.preview_scroll_up),
            ),
            "Scroll preview down / up",
        ),
        (Keybinds::format_short(&kb.search), "Live filter entries"),
        (
            Keybinds::format_short(&kb.cancel),
            "Cancel selection / clear filter / quit",
        ),
        (
            Keybinds::format_short(&kb.quit_cd),
            "Quit and cd (writes --cwd-file)",
        ),
        (
            Keybinds::format_short(&kb.quit),
            "Quit without changing directory",
        ),
        (
            Keybinds::format_short(&kb.toggle_help),
            "Toggle help modal",
        ),
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
                .title(format!(
                    " lsi Keybinds (press {} or {} to close, {} to quit) ",
                    Keybinds::primary_short(&kb.toggle_help),
                    Keybinds::primary_short(&kb.close_help),
                    Keybinds::primary_short(&kb.quit_cd),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(modal, popup_area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use crate::config::LoadedConfig;
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
        fs::write(
            base.join("src/main.rs"),
            "fn main() { println!(\"hi\"); }\n",
        )
        .unwrap();

        let args = Args::try_parse_from([
            "lsi",
            "-l",
            "--preview",
            "--theme",
            "catppuccin",
            base.to_str().unwrap(),
        ])
        .unwrap();
        let mut app = App::new_with_config(args, LoadedConfig::default()).unwrap();

        let backend = TestBackend::new(120, 35);
        let mut terminal = Terminal::new(backend).unwrap();

        app.wait_for_preview_for_test(std::time::Duration::from_secs(2));
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let buf_str = buffer_to_string(&terminal);
        assert!(buf_str.contains("Permissions"));
        assert!(buf_str.contains("Date Modified"));
        assert!(buf_str.contains("Cargo.toml"));
        assert!(!buf_str.contains("Preview:"));
        assert!(buf_str.contains("main.rs"));

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

        // Standard inline viewport (height >= 6) anchored at non-zero cursor row y = 5
        let args = Args::try_parse_from(["lsi", "-n", "12", base.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, LoadedConfig::default()).unwrap();
        let height = app.desired_inline_height();
        assert_eq!(height, 7);

        let mut backend = TestBackend::new(100, 30);
        backend.set_cursor_position(Position::new(0, 5)).unwrap();
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(height),
            },
        )
        .unwrap();
        assert_eq!(terminal.get_frame().area(), Rect::new(0, 5, 100, 7));

        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let out = buffer_to_string(&terminal);
        assert!(out.contains("Cargo.toml"));
        assert!(out.contains("src"));

        // Compact inline viewport (height < 6) anchored at non-zero cursor row y = 4
        let compact_args =
            Args::try_parse_from(["lsi", "-n", "5", base.to_str().unwrap()]).unwrap();
        let mut compact_app = App::new_with_config(compact_args, LoadedConfig::default()).unwrap();
        let compact_h = compact_app.desired_inline_height();
        assert_eq!(compact_h, 4);

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
        assert_eq!(compact_terminal.get_frame().area(), Rect::new(0, 4, 100, 4));

        compact_terminal
            .draw(|frame| draw(frame, &mut compact_app))
            .unwrap();
        let compact_out = buffer_to_string(&compact_terminal);
        assert!(compact_out.contains("Cargo.toml"));
        assert!(compact_out.contains("?:Help"));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn renders_without_borders_and_without_footer() {
        let base = std::env::temp_dir().join(format!("lsi_noborder_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("src")).unwrap();
        fs::write(base.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();

        let args = Args::try_parse_from([
            "lsi",
            "--no-borders",
            "--no-footer",
            base.to_str().unwrap(),
        ])
        .unwrap();
        let mut app = App::new_with_config(args, LoadedConfig::default()).unwrap();
        assert!(!app.show_borders);
        assert!(!app.show_footer);
        assert_eq!(app.desired_inline_height(), 3);

        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let out = buffer_to_string(&terminal);
        assert!(out.contains("Cargo.toml"));
        assert!(out.contains("src"));
        assert!(!out.contains('╭'));
        assert!(!out.contains('╰'));
        assert!(!out.contains("?:Help"));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn renders_selection_clipboard_and_confirm_delete_prompt() {
        use crossterm::event::KeyModifiers;

        let base = std::env::temp_dir().join(format!("lsi_ops_ui_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("src")).unwrap();
        fs::write(base.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();

        let args = Args::try_parse_from(["lsi", base.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, LoadedConfig::default()).unwrap();

        // Multi-select first item ('src') with Space
        app.handle_key(KeyEvent::from(KeyCode::Char(' ')));
        let backend = TestBackend::new(100, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let sel_out = buffer_to_string(&terminal);
        assert!(sel_out.contains("● "));
        assert!(sel_out.contains("[1 sel]"));

        // Yank with 'y' shows '+' marker and '[1 yanked]' badge
        app.handle_key(KeyEvent::from(KeyCode::Char('y')));
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let yank_out = buffer_to_string(&terminal);
        assert!(yank_out.contains("+ "));
        assert!(yank_out.contains("[1 yanked]"));

        // Shift+Delete renders permanent delete confirmation prompt
        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::SHIFT));
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let del_out = buffer_to_string(&terminal);
        assert!(del_out.contains("DELETE"));
        assert!(del_out.contains("Permanently delete"));
        assert!(del_out.contains("(y/N)"));

        // Cancel delete and open NEW creation prompt via 'a'
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        app.handle_key(KeyEvent::from(KeyCode::Char('a')));
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        app.handle_key(KeyEvent::from(KeyCode::Char('/')));
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let new_out = buffer_to_string(&terminal);
        assert!(new_out.contains("NEW"));
        assert!(new_out.contains("d/"));
        assert!(new_out.contains("[folder]"));

        // Cancel create and open RENAME prompt prefilled with highlighted 'Cargo.toml'
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        app.handle_key(KeyEvent::from(KeyCode::Char('r')));
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let rename_out = buffer_to_string(&terminal);
        assert!(rename_out.contains("RENAME"));
        assert!(rename_out.contains("Cargo.toml▌"));
        assert!(rename_out.contains("(Enter: rename, Esc: cancel)"));

        // Moving cursor left places '▌' before 'toml'
        for _ in 0..4 {
            app.handle_key(KeyEvent::from(KeyCode::Left));
        }
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let cursor_out = buffer_to_string(&terminal);
        assert!(cursor_out.contains("Cargo.▌toml"));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn renders_custom_keybinds_in_footer_and_help_modal() {
        use crate::config::LoadedConfig;

        let base =
            std::env::temp_dir().join(format!("lsi_custom_kb_ui_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("src")).unwrap();
        fs::write(base.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();

        let keybinds = Keybinds::from_toml_str(
            r#"
rename_item = "F2"
toggle_help = "F1"
"#,
        )
        .unwrap();
        let loaded = LoadedConfig {
            keybinds,
            ..LoadedConfig::default()
        };
        let args = Args::try_parse_from(["lsi", base.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, loaded).unwrap();

        let backend = TestBackend::new(140, 32);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let footer_out = buffer_to_string(&terminal);
        assert!(footer_out.contains("F2:Rename"));
        assert!(footer_out.contains("F1:Help"));

        app.handle_key(KeyEvent::from(KeyCode::F(1)));
        assert_eq!(app.input_mode, InputMode::Help);
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let help_out = buffer_to_string(&terminal);
        assert!(help_out.contains("F2"));
        assert!(help_out.contains("F1"));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn renders_highlighted_text_and_image_preview_without_panicking() {
        let base =
            std::env::temp_dir().join(format!("lsi_real_preview_ui_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        fs::write(
            base.join("demo.rs"),
            "pub fn hello() -> &'static str {\n    \"world\"\n}\n",
        )
        .unwrap();

        #[cfg(feature = "image-preview")]
        {
            let img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
                image::ImageBuffer::from_pixel(16, 12, image::Rgba([20, 180, 90, 255]));
            img.save(base.join("sample.png")).unwrap();
        }

        let args = Args::try_parse_from(["lsi", "--preview", base.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, LoadedConfig::default()).unwrap();

        let backend = TestBackend::new(120, 25);
        let mut terminal = Terminal::new(backend).unwrap();

        // First draw sets preview area while worker loads demo.rs
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        app.wait_for_preview_for_test(std::time::Duration::from_secs(2));
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let code_out = buffer_to_string(&terminal);
        assert!(!code_out.contains("Preview:"));
        assert!(code_out.contains("hello"));

        #[cfg(feature = "image-preview")]
        {
            app.move_selection(1);
            assert_eq!(app.selected_entry().unwrap().name, "sample.png");
            app.wait_for_preview_for_test(std::time::Duration::from_secs(2));
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            let img_out = buffer_to_string(&terminal);
            assert!(img_out.contains("sample.png"));
            assert!(!img_out.contains("Preview:"));
            assert!(matches!(
                app.preview_data,
                PreviewData::Image {
                    width: 16,
                    height: 12,
                    ..
                }
            ));
        }

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
