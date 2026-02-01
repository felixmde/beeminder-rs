//! State types for the TUI application.

use beeminder::types::{Datapoint, GoalSummary};
use ratatui::widgets::TableState;
use std::time::{Duration, Instant};
use time::OffsetDateTime;
use unicode_width::UnicodeWidthStr;

pub const STATUS_TTL: Duration = Duration::from_secs(4);
pub const TICK_RATE: Duration = Duration::from_millis(200);

/// The current screen being displayed.
#[derive(Debug)]
pub enum Screen {
    Main,
    Detail(DetailState),
}

/// Input mode for the main screen.
#[derive(Debug)]
pub enum MainInput {
    Normal,
    InlineAdd { buffer: String },
    Filter { buffer: String },
}

/// Status message severity.
#[derive(Debug, Clone, Copy)]
pub enum StatusKind {
    Info,
    Success,
    Error,
}

/// A status message with expiration tracking.
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub kind: StatusKind,
    pub text: String,
    pub created: Instant,
}

/// State for the detail/edit screen.
#[derive(Debug)]
pub struct DetailState {
    pub goal_slug: String,
    pub goal_title: String,
    pub rows: Vec<EditorRow>,
    pub table_state: TableState,
    pub selected_col: EditorCol,
    pub input: Option<EditInput>,
    pub dirty: bool,
    pub confirm_discard: bool,
}

impl DetailState {
    pub fn from_datapoints(goal: &GoalSummary, datapoints: Vec<Datapoint>) -> Self {
        let rows = datapoints
            .into_iter()
            .map(EditorRow::from_datapoint)
            .collect::<Vec<_>>();
        let mut table_state = TableState::default();
        if !rows.is_empty() {
            table_state.select(Some(0));
        }
        Self {
            goal_slug: goal.slug.clone(),
            goal_title: goal.title.clone(),
            rows,
            table_state,
            selected_col: EditorCol::Timestamp,
            input: None,
            dirty: false,
            confirm_discard: false,
        }
    }

    pub const fn selected_row_index(&self) -> Option<usize> {
        self.table_state.selected()
    }

    pub fn selected_row_mut(&mut self) -> Option<&mut EditorRow> {
        let idx = self.selected_row_index()?;
        self.rows.get_mut(idx)
    }

    pub const fn mark_dirty(&mut self) {
        self.dirty = true;
        self.confirm_discard = false;
    }

    pub fn move_row(&mut self, delta: i32) {
        if self.rows.is_empty() {
            return;
        }
        let selected = self.table_state.selected().unwrap_or(0);
        let max = self.rows.len().saturating_sub(1);
        let next = clamp_index(selected, delta, max);
        self.table_state.select(Some(next));
    }

    pub fn move_col(&mut self, delta: i32) {
        let current = EditorCol::VALUES
            .iter()
            .position(|col| *col == self.selected_col)
            .unwrap_or(0);
        let max = EditorCol::VALUES.len().saturating_sub(1);
        let next = clamp_index(current, delta, max);
        self.selected_col = EditorCol::VALUES[next];
    }

    pub fn toggle_delete(&mut self) {
        if let Some(row) = self.selected_row_mut() {
            row.is_deleted = !row.is_deleted;
            self.mark_dirty();
        }
    }

    pub fn add_new_row(&mut self) {
        let now = OffsetDateTime::now_utc();
        let row = EditorRow::new(now);
        self.rows.insert(0, row);
        self.table_state.select(Some(0));
        self.mark_dirty();
    }
}

/// Column identifiers for the editor table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorCol {
    Timestamp,
    Value,
    Comment,
}

impl EditorCol {
    pub const VALUES: [Self; 3] = [Self::Timestamp, Self::Value, Self::Comment];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Timestamp => "TIMESTAMP",
            Self::Value => "VALUE",
            Self::Comment => "COMMENT",
        }
    }
}

/// Input state when editing a cell.
#[derive(Debug)]
pub struct EditInput {
    pub buffer: String,
    pub cursor: usize,
}

impl EditInput {
    /// Creates a new EditInput with cursor at the end.
    pub fn new(buffer: String) -> Self {
        let cursor = buffer.len();
        Self { buffer, cursor }
    }

    /// Insert a character at the cursor position.
    pub fn insert(&mut self, c: char) {
        self.buffer.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Delete the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let prev = self.buffer[..self.cursor]
                .chars()
                .last()
                .map(char::len_utf8)
                .unwrap_or(0);
            self.cursor -= prev;
            self.buffer.remove(self.cursor);
        }
    }

    /// Delete the character at the cursor (delete key).
    pub fn delete(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
        }
    }

    /// Move cursor left by one character.
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            let prev = self.buffer[..self.cursor]
                .chars()
                .last()
                .map(char::len_utf8)
                .unwrap_or(0);
            self.cursor -= prev;
        }
    }

    /// Move cursor right by one character.
    pub fn move_right(&mut self) {
        if self.cursor < self.buffer.len() {
            let next = self.buffer[self.cursor..]
                .chars()
                .next()
                .map(char::len_utf8)
                .unwrap_or(0);
            self.cursor += next;
        }
    }

    /// Move cursor to the beginning.
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to the end.
    pub fn move_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    /// Returns the cursor position in display columns.
    pub fn cursor_display_width(&self) -> usize {
        self.buffer[..self.cursor].width()
    }
}

/// A row in the datapoint editor.
#[derive(Debug)]
pub struct EditorRow {
    pub id: Option<String>,
    pub timestamp: OffsetDateTime,
    pub value: f64,
    pub comment: String,
    pub original: Option<RowSnapshot>,
    pub is_deleted: bool,
}

/// Snapshot of original datapoint values for change detection.
#[derive(Debug)]
pub struct RowSnapshot {
    pub timestamp: OffsetDateTime,
    pub value: f64,
    pub comment: String,
}

impl EditorRow {
    pub fn from_datapoint(dp: Datapoint) -> Self {
        let comment = dp.comment.unwrap_or_default();
        Self {
            id: Some(dp.id),
            timestamp: dp.timestamp,
            value: dp.value,
            comment: comment.clone(),
            original: Some(RowSnapshot {
                timestamp: dp.timestamp,
                value: dp.value,
                comment,
            }),
            is_deleted: false,
        }
    }

    pub const fn new(timestamp: OffsetDateTime) -> Self {
        Self {
            id: None,
            timestamp,
            value: 0.0,
            comment: String::new(),
            original: None,
            is_deleted: false,
        }
    }

    pub fn is_modified(&self) -> bool {
        self.original.as_ref().is_none_or(|orig| {
            self.timestamp != orig.timestamp
                || (self.value - orig.value).abs() > f64::EPSILON
                || self.comment != orig.comment
        })
    }

    pub fn marker(&self) -> &'static str {
        if self.id.is_none() {
            "+"
        } else if self.is_modified() {
            "*"
        } else {
            " "
        }
    }
}

/// Clamp an index after applying a delta.
pub fn clamp_index(current: usize, delta: i32, max: usize) -> usize {
    let current = isize::try_from(current).unwrap_or(0);
    let max = isize::try_from(max).unwrap_or(0);
    let delta = isize::try_from(delta).unwrap_or(0);
    let next = (current + delta).clamp(0, max);
    usize::try_from(next).unwrap_or(0)
}
