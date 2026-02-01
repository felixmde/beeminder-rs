//! UI rendering functions.

use crate::app::{has_entry_today, App};
use crate::state::{
    DetailState, EditorCol, EditorRow, MainInput, Screen, StatusKind, StatusMessage,
};
use beeconfig::format_timestamp;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState};
use std::time::Duration;
use unicode_width::UnicodeWidthStr;

/// Render the application based on current screen.
pub fn render_app(f: &mut ratatui::Frame, app: &mut App) {
    if matches!(app.screen, Screen::Main) {
        render_main(f, app);
    } else {
        let status = app.status.clone();
        let mut detail = match std::mem::replace(&mut app.screen, Screen::Main) {
            Screen::Detail(detail) => detail,
            Screen::Main => return,
        };
        render_detail(f, status.as_ref(), &mut detail);
        app.screen = Screen::Detail(detail);
    }
}

fn render_main(f: &mut ratatui::Frame, app: &mut App) {
    let size = f.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(2)])
        .split(size);

    let block = Block::default()
        .title_top("beetui")
        .title_top(Line::from("[r]efresh  [q]uit").right_aligned())
        .borders(Borders::ALL);

    let inner = block.inner(layout[0]);
    f.render_widget(block, layout[0]);

    let rows = build_goal_rows(app);
    let widths = build_goal_widths(app);

    let table = Table::new(rows, widths)
        .column_spacing(1)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    if app.filtered.is_empty() {
        let empty = Paragraph::new("No goals").alignment(Alignment::Center);
        f.render_widget(empty, inner);
    } else {
        ensure_table_state_visible(&mut app.main_state, inner.height as usize);
        f.render_stateful_widget(table, inner, &mut app.main_state);
    }

    render_footer_main(
        f,
        app.status.as_ref(),
        &app.main_input,
        &app.filter,
        layout[1],
    );

    if let MainInput::InlineAdd { buffer } = &app.main_input {
        let prompt = format!("Add datapoint: {buffer}");
        set_footer_cursor(f, layout[1], UnicodeWidthStr::width(prompt.as_str()));
    }

    if let MainInput::Filter { buffer } = &app.main_input {
        let prompt = format!("Filter: {buffer}");
        set_footer_cursor(f, layout[1], UnicodeWidthStr::width(prompt.as_str()));
    }
}

fn build_goal_rows(app: &App) -> Vec<Row<'static>> {
    let mut rows = Vec::new();
    let highlight_goal = app
        .last_success_goal
        .as_ref()
        .filter(|(_, at)| at.elapsed() < Duration::from_secs(2))
        .map(|(slug, _)| slug.as_str());

    for (row_idx, goal_idx) in app.filtered.iter().enumerate() {
        let Some(goal) = app.goals.get(*goal_idx) else {
            continue;
        };
        let check = if has_entry_today(goal) { "x" } else { " " };
        let mut slug = goal.slug.clone();
        let mut limsum = goal.limsum.clone();

        if let MainInput::InlineAdd { buffer } = &app.main_input {
            if Some(row_idx) == app.main_state.selected() {
                slug = format!("{}: {}", goal.slug, buffer);
                limsum.clear();
            }
        }

        let mut cells = Vec::new();
        cells.push(Cell::from(check));
        cells.push(Cell::from(slug));
        cells.push(Cell::from(limsum));

        if app.config.display.show_pledge {
            let pledge =
                goal_pledge(goal).map_or_else(|| "-".to_string(), |value| format!("${value:.0}"));
            cells.push(Cell::from(pledge));
        }

        let mut style = Style::default().fg(goal_color(goal.safebuf));
        if let Some(slug) = highlight_goal {
            if goal.slug == slug {
                style = style.bg(Color::Green).fg(Color::Black);
            }
        }

        rows.push(Row::new(cells).style(style));
    }

    rows
}

fn build_goal_widths(app: &App) -> Vec<Constraint> {
    let mut widths = vec![
        Constraint::Length(2),
        Constraint::Length(20),
        Constraint::Min(10),
    ];
    if app.config.display.show_pledge {
        widths.push(Constraint::Length(7));
    }
    widths
}

fn render_detail(f: &mut ratatui::Frame, status: Option<&StatusMessage>, detail: &mut DetailState) {
    let size = f.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(2)])
        .split(size);

    let display = if detail.goal_title.is_empty() {
        detail.goal_slug.clone()
    } else {
        detail.goal_title.clone()
    };
    let title = format!("{display} - Edit Datapoints");
    let block = Block::default()
        .title_top(title)
        .title_top(Line::from("[?]help").right_aligned())
        .borders(Borders::ALL);

    let inner = block.inner(layout[0]);
    f.render_widget(block, layout[0]);

    let header_cells = EditorCol::VALUES.iter().map(|col| {
        let style = if *col == detail.selected_col {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        Cell::from(col.label()).style(style)
    });
    let header = Row::new(header_cells).style(Style::default().add_modifier(Modifier::BOLD));

    let rows = detail
        .rows
        .iter()
        .enumerate()
        .map(|(idx, row)| build_editor_row(row, detail, idx))
        .collect::<Vec<_>>();

    let widths = vec![
        Constraint::Length(20),
        Constraint::Length(8),
        Constraint::Min(10),
    ];
    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    if detail.rows.is_empty() {
        let empty = Paragraph::new("No datapoints").alignment(Alignment::Center);
        f.render_widget(empty, inner);
    } else {
        let available = inner.height.saturating_sub(1) as usize;
        ensure_table_state_visible(&mut detail.table_state, available);
        f.render_stateful_widget(table, inner, &mut detail.table_state);
    }

    render_footer_detail(f, status, detail, layout[1]);
    render_detail_input_modal(f, detail, size);
}

fn build_editor_row<'a>(row: &'a EditorRow, detail: &DetailState, idx: usize) -> Row<'a> {
    let timestamp = format!("{}{}", row.marker(), format_timestamp(row.timestamp));
    let value = if row.id.is_none() && row.value == 0.0 {
        String::new()
    } else {
        row.value.to_string()
    };
    let mut comment = row.comment.clone();
    if row.is_deleted {
        if comment.is_empty() {
            comment = "[DEL]".to_string();
        } else {
            comment = format!("{comment} [DEL]");
        }
    }

    let mut style = Style::default();
    if row.is_deleted {
        style = style.fg(Color::Red).add_modifier(Modifier::DIM);
    } else if row.id.is_none() {
        style = style.fg(Color::Cyan);
    } else if row.is_modified() {
        style = style.fg(Color::Yellow);
    }

    let mut cells = vec![
        Cell::from(timestamp),
        Cell::from(value),
        Cell::from(comment),
    ];

    if let Some(selected) = detail.table_state.selected() {
        if selected == idx {
            if let Some(input) = &detail.input {
                let buffer = input.buffer.clone();
                match detail.selected_col {
                    EditorCol::Timestamp => cells[0] = Cell::from(buffer),
                    EditorCol::Value => cells[1] = Cell::from(buffer),
                    EditorCol::Comment => cells[2] = Cell::from(buffer),
                }
            }
        }
    }

    Row::new(cells).style(style)
}

fn render_status_line(f: &mut ratatui::Frame, status: Option<&StatusMessage>, area: Rect) {
    let widget = status.map(|status| {
        let style = match status.kind {
            StatusKind::Info => Style::default().fg(Color::Blue),
            StatusKind::Success => Style::default().fg(Color::Green),
            StatusKind::Error => Style::default().fg(Color::Red),
        };
        Paragraph::new(status.text.clone()).style(style)
    });

    if let Some(widget) = widget {
        f.render_widget(widget, area);
    } else {
        f.render_widget(Paragraph::new(""), area);
    }
}

fn render_footer_detail(
    f: &mut ratatui::Frame,
    status: Option<&StatusMessage>,
    detail: &DetailState,
    area: Rect,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    render_status_line(f, status, layout[0]);

    let line = if detail.input.is_some() {
        Line::from("Enter: confirm  Esc: cancel")
    } else {
        Line::from("j/k or up/down: move  h/l or left/right: column  Enter: edit  n: new  d: delete  s: save  Esc: back")
    };

    let footer = Paragraph::new(line);
    f.render_widget(footer, layout[1]);
}

fn render_detail_input_modal(f: &mut ratatui::Frame, detail: &DetailState, area: Rect) {
    let Some(input) = &detail.input else {
        return;
    };

    let popup = centered_rect(60, 20, area);
    f.render_widget(Clear, popup);

    let title = format!("Edit {}", detail.selected_col.label());
    let block = Block::default().title_top(title).borders(Borders::ALL);
    let inner = block.inner(popup);
    f.render_widget(block, popup);
    f.render_widget(Paragraph::new(input.buffer.as_str()), inner);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let offset = u16::try_from(input.cursor_display_width()).unwrap_or(u16::MAX);
    let max_x = inner.x + inner.width - 1;
    let cursor_x = inner.x.saturating_add(offset).min(max_x);
    let cursor_y = inner.y;
    f.set_cursor_position(Position::new(cursor_x, cursor_y));
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);

    horizontal[1]
}

fn render_footer_main(
    f: &mut ratatui::Frame,
    status: Option<&StatusMessage>,
    main_input: &MainInput,
    filter: &str,
    area: Rect,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    render_status_line(f, status, layout[0]);

    let line = match main_input {
        MainInput::InlineAdd { buffer } => Line::from(vec![
            Span::raw(format!("Add datapoint: {buffer}")),
            Span::raw("  Enter: submit  Esc: cancel"),
        ]),
        MainInput::Filter { buffer } => Line::from(vec![
            Span::raw(format!("Filter: {buffer}")),
            Span::raw("  Enter: apply  Esc: cancel"),
        ]),
        MainInput::Normal => {
            if filter.is_empty() {
                Line::from(
                    "j/k or up/down: navigate  Enter: add  e: edit  /: filter  r: refresh  q: quit",
                )
            } else {
                Line::from(vec![
                    Span::raw(format!("Filter: {filter}  ")),
                    Span::raw("j/k: navigate  Enter: add  e: edit  /: filter"),
                ])
            }
        }
    };

    let footer = Paragraph::new(line);
    f.render_widget(footer, layout[1]);
}

fn set_footer_cursor(f: &mut ratatui::Frame, area: Rect, x_offset: usize) {
    if area.width == 0 || area.height < 2 {
        return;
    }
    let offset = u16::try_from(x_offset).unwrap_or(u16::MAX);
    let max_x = area.x + area.width - 1;
    let cursor_x = area.x.saturating_add(offset).min(max_x);
    let cursor_y = area.y + 1;
    f.set_cursor_position(Position::new(cursor_x, cursor_y));
}

fn ensure_table_state_visible(state: &mut TableState, height: usize) {
    if height == 0 {
        return;
    }
    let Some(selected) = state.selected() else {
        return;
    };
    let offset = state.offset();
    if selected < offset {
        *state.offset_mut() = selected;
    } else if selected >= offset + height {
        *state.offset_mut() = selected.saturating_sub(height - 1);
    }
}

const fn goal_color(safebuf: i32) -> Color {
    match safebuf {
        0 => Color::Red,
        1 => Color::Yellow,
        2 => Color::Blue,
        3..=6 => Color::Green,
        _ => Color::White,
    }
}

fn goal_pledge(goal: &beeminder::types::GoalSummary) -> Option<f64> {
    goal.extra.get("pledge").and_then(serde_json::Value::as_f64)
}
