//! Keyboard event handlers.

use crate::app::App;
use crate::state::{DetailState, EditInput, EditorCol, MainInput, Screen, StatusKind};
use beeconfig::{format_timestamp, parse_timestamp};
use beeminder::types::{CreateDatapoint, UpdateDatapoint};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::runtime::Runtime;

/// Handle a key event, returns true if the app should exit.
pub fn handle_key(app: &mut App, key: KeyEvent, runtime: &Runtime) -> bool {
    if matches!(app.screen, Screen::Main) {
        handle_main_key(app, key, runtime)
    } else {
        let mut detail = match std::mem::replace(&mut app.screen, Screen::Main) {
            Screen::Detail(detail) => detail,
            Screen::Main => return false,
        };
        let outcome = handle_detail_key(app, &mut detail, key, runtime);
        match outcome {
            DetailOutcome::Stay => app.screen = Screen::Detail(detail),
            DetailOutcome::Exit => app.screen = Screen::Main,
        }
        false
    }
}

fn handle_main_key(app: &mut App, key: KeyEvent, runtime: &Runtime) -> bool {
    match &mut app.main_input {
        MainInput::Normal => match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('r') => {
                if let Err(err) = app.refresh_goals(runtime) {
                    app.set_status(StatusKind::Error, err.to_string());
                } else {
                    app.set_status(StatusKind::Info, "Goals refreshed".to_string());
                }
            }
            KeyCode::Char('j') | KeyCode::Down => app.move_main_selection(1),
            KeyCode::Char('k') | KeyCode::Up => app.move_main_selection(-1),
            KeyCode::Enter => app.start_inline_add(),
            KeyCode::Char('e') => app.open_detail(runtime),
            KeyCode::Char('/') => app.enter_filter_mode(),
            _ => {}
        },
        MainInput::InlineAdd { buffer } => match key.code {
            KeyCode::Esc => app.cancel_inline_add(),
            KeyCode::Enter => {
                let input = buffer.clone();
                app.submit_inline_add(&input, runtime);
            }
            KeyCode::Backspace => {
                buffer.pop();
            }
            KeyCode::Char(c) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    buffer.push(c);
                }
            }
            _ => {}
        },
        MainInput::Filter { buffer } => match key.code {
            KeyCode::Esc => app.cancel_filter_mode(),
            KeyCode::Enter => {
                let next = buffer.clone();
                app.apply_filter(next);
            }
            KeyCode::Backspace => {
                buffer.pop();
                app.filter = buffer.clone();
                app.refresh_filtered();
            }
            KeyCode::Char(c) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    buffer.push(c);
                    app.filter = buffer.clone();
                    app.refresh_filtered();
                }
            }
            _ => {}
        },
    }
    false
}

enum DetailOutcome {
    Stay,
    Exit,
}

fn handle_detail_key(
    app: &mut App,
    detail: &mut DetailState,
    key: KeyEvent,
    runtime: &Runtime,
) -> DetailOutcome {
    if let Some(mut input) = detail.input.take() {
        match key.code {
            KeyCode::Esc => {
                detail.input = None;
            }
            KeyCode::Enter => {
                let buffer = input.buffer.clone();
                if let Err(err) = apply_detail_edit(detail, &buffer) {
                    app.set_status(StatusKind::Error, err);
                    detail.input = Some(input);
                } else {
                    detail.input = None;
                }
            }
            KeyCode::Backspace => {
                input.buffer.pop();
                detail.input = Some(input);
            }
            KeyCode::Char(c) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    input.buffer.push(c);
                }
                detail.input = Some(input);
            }
            _ => {
                detail.input = Some(input);
            }
        }
        return DetailOutcome::Stay;
    }

    match key.code {
        KeyCode::Esc => {
            if detail.dirty {
                if detail.confirm_discard {
                    return DetailOutcome::Exit;
                }
                app.set_status(
                    StatusKind::Info,
                    "Unsaved changes. Press Esc again to discard.".to_string(),
                );
                detail.confirm_discard = true;
            } else {
                return DetailOutcome::Exit;
            }
        }
        KeyCode::Char('j') | KeyCode::Down => detail.move_row(1),
        KeyCode::Char('k') | KeyCode::Up => detail.move_row(-1),
        KeyCode::Char('h') | KeyCode::Left => detail.move_col(-1),
        KeyCode::Char('l') | KeyCode::Right => detail.move_col(1),
        KeyCode::Enter => start_detail_edit(detail),
        KeyCode::Char('n') => detail.add_new_row(),
        KeyCode::Char('d') => detail.toggle_delete(),
        KeyCode::Char('s') => {
            if save_detail_changes(app, detail, runtime) {
                return DetailOutcome::Exit;
            }
        }
        _ => {}
    }

    DetailOutcome::Stay
}

fn start_detail_edit(detail: &mut DetailState) {
    let selected_col = detail.selected_col;
    if let Some(row) = detail.selected_row_mut() {
        let buffer = match selected_col {
            EditorCol::Timestamp => format_timestamp(row.timestamp),
            EditorCol::Value => row.value.to_string(),
            EditorCol::Comment => row.comment.clone(),
        };
        detail.input = Some(EditInput { buffer });
    }
}

fn apply_detail_edit(detail: &mut DetailState, input: &str) -> Result<(), String> {
    let selected_col = detail.selected_col;
    let trimmed = input.trim();
    let mut modified = false;
    let result = {
        let Some(row) = detail.selected_row_mut() else {
            return Ok(());
        };
        let result = match selected_col {
            EditorCol::Timestamp => parse_timestamp(trimmed)
                .map(|ts| {
                    row.timestamp = ts;
                })
                .map_err(|e| e.to_string()),
            EditorCol::Value => match trimmed.parse::<f64>() {
                Ok(value) => {
                    row.value = value;
                    Ok(())
                }
                Err(_) => Err("Invalid value".to_string()),
            },
            EditorCol::Comment => {
                row.comment = trimmed.to_string();
                Ok(())
            }
        };
        if result.is_ok() {
            modified = row.is_modified();
        }
        result
    };

    if modified {
        detail.mark_dirty();
    }

    result
}

fn save_detail_changes(app: &mut App, detail: &DetailState, runtime: &Runtime) -> bool {
    let mut creates = Vec::new();
    let mut updates = Vec::new();
    let mut deletes = Vec::new();

    for row in &detail.rows {
        if row.id.is_none() {
            if !row.is_deleted {
                let mut dp = CreateDatapoint::new(row.value).with_timestamp(row.timestamp);
                if !row.comment.trim().is_empty() {
                    dp = dp.with_comment(&row.comment);
                }
                creates.push(dp);
            }
            continue;
        }

        let id = match &row.id {
            Some(id) => id.clone(),
            None => continue,
        };

        if row.is_deleted {
            deletes.push(id);
            continue;
        }

        if row.is_modified() {
            let mut update = UpdateDatapoint::new(id)
                .with_timestamp(row.timestamp)
                .with_value(row.value);
            if row.comment.trim().is_empty() {
                update.comment = Some(String::new());
            } else {
                update = update.with_comment(&row.comment);
            }
            updates.push(update);
        }
    }

    if creates.is_empty() && updates.is_empty() && deletes.is_empty() {
        app.set_status(StatusKind::Info, "No changes to save".to_string());
        return false;
    }

    app.set_status(
        StatusKind::Info,
        format!(
            "Saving: {} new, {} updated, {} deleted",
            creates.len(),
            updates.len(),
            deletes.len()
        ),
    );

    let slug = detail.goal_slug.clone();
    let result = runtime.block_on(async {
        for dp in creates {
            app.client.create_datapoint(&slug, &dp).await?;
        }
        for update in updates {
            app.client.update_datapoint(&slug, &update).await?;
        }
        for id in deletes {
            app.client.delete_datapoint(&slug, &id).await?;
        }
        Ok::<(), beeminder::Error>(())
    });

    match result {
        Ok(()) => {
            if let Err(err) = app.refresh_goals(runtime) {
                app.set_status(StatusKind::Error, err.to_string());
            } else {
                app.set_status(StatusKind::Success, "Saved changes".to_string());
            }
            true
        }
        Err(err) => {
            app.set_status(StatusKind::Error, err.to_string());
            false
        }
    }
}
