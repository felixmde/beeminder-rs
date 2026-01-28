#![allow(clippy::multiple_crate_versions)]

use anyhow::{Context, Result};
use beeconfig::BeeConfig;
use beeminder::types::{CreateDatapoint, Datapoint, GoalSummary, UpdateDatapoint};
use beeminder::BeeminderClient;
use crossterm::cursor::Show;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::time::{Duration, Instant};
use time::macros::format_description;
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};
use tokio::runtime::Runtime;

const STATUS_TTL: Duration = Duration::from_secs(4);
const TICK_RATE: Duration = Duration::from_millis(200);

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

struct App {
    config: BeeConfig,
    client: BeeminderClient,
    goals: Vec<GoalSummary>,
    filtered: Vec<usize>,
    filter: String,
    filter_backup: Option<String>,
    main_state: TableState,
    main_input: MainInput,
    screen: Screen,
    status: Option<StatusMessage>,
    last_success_goal: Option<(String, Instant)>,
}

impl App {
    fn new(config: BeeConfig, client: BeeminderClient) -> Self {
        Self {
            config,
            client,
            goals: Vec::new(),
            filtered: Vec::new(),
            filter: String::new(),
            filter_backup: None,
            main_state: TableState::default(),
            main_input: MainInput::Normal,
            screen: Screen::Main,
            status: None,
            last_success_goal: None,
        }
    }

    fn refresh_goals(&mut self, runtime: &Runtime) -> Result<()> {
        let mut goals = runtime
            .block_on(self.client.get_goals())
            .context("Failed to fetch goals")?;
        goals.sort_by(|a, b| {
            let today_cmp = has_entry_today(a).cmp(&has_entry_today(b));
            if today_cmp != std::cmp::Ordering::Equal {
                return today_cmp;
            }
            a.safebuf.cmp(&b.safebuf)
        });
        self.goals = goals;
        self.refresh_filtered();
        Ok(())
    }

    fn refresh_filtered(&mut self) {
        let needle = self.filter.to_ascii_lowercase();
        self.filtered = self
            .goals
            .iter()
            .enumerate()
            .filter(|(_, goal)| {
                if needle.is_empty() {
                    return true;
                }
                let slug = goal.slug.to_ascii_lowercase();
                let title = goal.title.to_ascii_lowercase();
                slug.contains(&needle) || title.contains(&needle)
            })
            .map(|(idx, _)| idx)
            .collect();
        if self.filtered.is_empty() {
            self.main_state.select(None);
        } else {
            let selected = self.main_state.selected().unwrap_or(0);
            let clamped = selected.min(self.filtered.len() - 1);
            self.main_state.select(Some(clamped));
        }
        *self.main_state.offset_mut() = 0;
    }

    fn selected_goal_index(&self) -> Option<usize> {
        let selected = self.main_state.selected()?;
        self.filtered.get(selected).copied()
    }

    fn selected_goal(&self) -> Option<&GoalSummary> {
        let idx = self.selected_goal_index()?;
        self.goals.get(idx)
    }

    fn select_goal_by_slug(&mut self, slug: &str) {
        if let Some((pos, _)) = self
            .filtered
            .iter()
            .enumerate()
            .find(|(_, idx)| self.goals.get(**idx).map(|g| g.slug.as_str()) == Some(slug))
        {
            self.main_state.select(Some(pos));
        }
    }

    fn set_status(&mut self, kind: StatusKind, text: String) {
        self.status = Some(StatusMessage {
            kind,
            text,
            created: Instant::now(),
        });
    }

    fn clear_expired_status(&mut self) {
        if let Some(status) = &self.status {
            if status.created.elapsed() > STATUS_TTL {
                self.status = None;
            }
        }
    }

    fn enter_filter_mode(&mut self) {
        self.filter_backup = Some(self.filter.clone());
        self.main_input = MainInput::Filter {
            buffer: self.filter.clone(),
        };
    }

    fn cancel_filter_mode(&mut self) {
        if let Some(prev) = self.filter_backup.take() {
            self.filter = prev;
            self.refresh_filtered();
        }
        self.main_input = MainInput::Normal;
    }

    fn apply_filter(&mut self, buffer: String) {
        self.filter = buffer;
        self.refresh_filtered();
        self.filter_backup = None;
        self.main_input = MainInput::Normal;
    }

    fn start_inline_add(&mut self) {
        if self.selected_goal().is_some() {
            self.main_input = MainInput::InlineAdd {
                buffer: String::new(),
            };
        } else {
            self.set_status(StatusKind::Info, "No goal selected".to_string());
        }
    }

    fn cancel_inline_add(&mut self) {
        self.main_input = MainInput::Normal;
    }

    fn submit_inline_add(&mut self, buffer: &str, runtime: &Runtime) {
        let Some(goal) = self.selected_goal() else {
            self.set_status(StatusKind::Info, "No goal selected".to_string());
            return;
        };

        let parsed = parse_value_and_comment(buffer);
        let (value, comment) = match parsed {
            Ok(parsed) => parsed,
            Err(err) => {
                self.set_status(StatusKind::Error, err);
                return;
            }
        };

        let mut dp = CreateDatapoint::new(value);
        if let Some(comment) = comment.as_deref() {
            dp = dp.with_comment(comment);
        }

        let slug = goal.slug.clone();
        let result = runtime.block_on(self.client.create_datapoint(&slug, &dp));
        match result {
            Ok(_) => {
                let refresh_result = self.refresh_goals(runtime);
                if let Err(err) = refresh_result {
                    self.set_status(
                        StatusKind::Error,
                        format!("Added datapoint to {slug}, but refresh failed: {err}"),
                    );
                } else {
                    self.set_status(StatusKind::Success, format!("Added datapoint to {slug}"));
                    self.last_success_goal = Some((slug.clone(), Instant::now()));
                    self.select_goal_by_slug(&slug);
                }
                self.main_input = MainInput::Normal;
            }
            Err(err) => {
                self.set_status(StatusKind::Error, err.to_string());
            }
        }
    }

    fn open_detail(&mut self, runtime: &Runtime) {
        let Some(goal) = self.selected_goal() else {
            self.set_status(StatusKind::Info, "No goal selected".to_string());
            return;
        };

        let limit = self.config.display.datapoints_limit as u64;
        let datapoints = runtime.block_on(self.client.get_datapoints(
            &goal.slug,
            Some("id"),
            Some(limit),
            None,
            None,
        ));

        match datapoints {
            Ok(points) => {
                let detail = DetailState::from_datapoints(goal, points);
                self.screen = Screen::Detail(detail);
            }
            Err(err) => {
                self.set_status(StatusKind::Error, err.to_string());
            }
        }
    }
}

#[derive(Debug)]
enum Screen {
    Main,
    Detail(DetailState),
}

#[derive(Debug)]
enum MainInput {
    Normal,
    InlineAdd { buffer: String },
    Filter { buffer: String },
}

#[derive(Debug)]
struct DetailState {
    goal_slug: String,
    goal_title: String,
    rows: Vec<EditorRow>,
    table_state: TableState,
    selected_col: EditorCol,
    input: Option<EditInput>,
    dirty: bool,
    confirm_discard: bool,
}

impl DetailState {
    fn from_datapoints(goal: &GoalSummary, datapoints: Vec<Datapoint>) -> Self {
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

    const fn selected_row_index(&self) -> Option<usize> {
        self.table_state.selected()
    }

    fn selected_row_mut(&mut self) -> Option<&mut EditorRow> {
        let idx = self.selected_row_index()?;
        self.rows.get_mut(idx)
    }

    const fn mark_dirty(&mut self) {
        self.dirty = true;
        self.confirm_discard = false;
    }

    fn move_row(&mut self, delta: i32) {
        if self.rows.is_empty() {
            return;
        }
        let selected = self.table_state.selected().unwrap_or(0);
        let max = self.rows.len().saturating_sub(1);
        let next = clamp_index(selected, delta, max);
        self.table_state.select(Some(next));
    }

    fn move_col(&mut self, delta: i32) {
        let current = EditorCol::VALUES
            .iter()
            .position(|col| *col == self.selected_col)
            .unwrap_or(0);
        let max = EditorCol::VALUES.len().saturating_sub(1);
        let next = clamp_index(current, delta, max);
        self.selected_col = EditorCol::VALUES[next];
    }

    fn toggle_delete(&mut self) {
        if let Some(row) = self.selected_row_mut() {
            row.is_deleted = !row.is_deleted;
            self.mark_dirty();
        }
    }

    fn add_new_row(&mut self) {
        let now = OffsetDateTime::now_utc();
        let row = EditorRow::new(now);
        self.rows.insert(0, row);
        self.table_state.select(Some(0));
        self.mark_dirty();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorCol {
    Timestamp,
    Value,
    Comment,
}

impl EditorCol {
    const VALUES: [Self; 3] = [Self::Timestamp, Self::Value, Self::Comment];

    const fn label(self) -> &'static str {
        match self {
            Self::Timestamp => "TIMESTAMP",
            Self::Value => "VALUE",
            Self::Comment => "COMMENT",
        }
    }
}

#[derive(Debug)]
struct EditInput {
    buffer: String,
}

#[derive(Debug)]
struct EditorRow {
    id: Option<String>,
    timestamp: OffsetDateTime,
    value: f64,
    comment: String,
    original: Option<RowSnapshot>,
    is_deleted: bool,
}

#[derive(Debug)]
struct RowSnapshot {
    timestamp: OffsetDateTime,
    value: f64,
    comment: String,
}

impl EditorRow {
    fn from_datapoint(dp: Datapoint) -> Self {
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

    const fn new(timestamp: OffsetDateTime) -> Self {
        Self {
            id: None,
            timestamp,
            value: 0.0,
            comment: String::new(),
            original: None,
            is_deleted: false,
        }
    }

    fn is_modified(&self) -> bool {
        self.original.as_ref().is_none_or(|orig| {
            self.timestamp != orig.timestamp
                || (self.value - orig.value).abs() > f64::EPSILON
                || self.comment != orig.comment
        })
    }

    fn marker(&self) -> &'static str {
        if self.id.is_none() {
            "+"
        } else if self.is_modified() {
            "*"
        } else {
            " "
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum StatusKind {
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone)]
struct StatusMessage {
    kind: StatusKind,
    text: String,
    created: Instant,
}

fn handle_key(app: &mut App, key: KeyEvent, runtime: &Runtime) -> bool {
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
            KeyCode::Char('j') | KeyCode::Down => move_main_selection(app, 1),
            KeyCode::Char('k') | KeyCode::Up => move_main_selection(app, -1),
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

fn move_main_selection(app: &mut App, delta: i32) {
    if app.filtered.is_empty() {
        return;
    }
    let selected = app.main_state.selected().unwrap_or(0);
    let max = app.filtered.len().saturating_sub(1);
    let next = clamp_index(selected, delta, max);
    app.main_state.select(Some(next));
}

fn clamp_index(current: usize, delta: i32, max: usize) -> usize {
    let current = isize::try_from(current).unwrap_or(0);
    let max = isize::try_from(max).unwrap_or(0);
    let delta = isize::try_from(delta).unwrap_or(0);
    let next = (current + delta).clamp(0, max);
    usize::try_from(next).unwrap_or(0)
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

fn apply_detail_edit(detail: &mut DetailState, input: &str) -> std::result::Result<(), String> {
    let selected_col = detail.selected_col;
    let trimmed = input.trim();
    let mut modified = false;
    let result = {
        let Some(row) = detail.selected_row_mut() else {
            return Ok(());
        };
        let result = match selected_col {
            EditorCol::Timestamp => parse_timestamp(trimmed).map(|ts| {
                row.timestamp = ts;
            }),
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

fn render_app(f: &mut ratatui::Frame, app: &mut App) {
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
        set_footer_cursor(f, layout[1], prompt.len());
    }

    if let MainInput::Filter { buffer } = &app.main_input {
        let prompt = format!("Filter: {buffer}");
        set_footer_cursor(f, layout[1], prompt.len());
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

    if let Some(input) = &detail.input {
        let prompt = format!("Edit {}: {}", detail.selected_col.label(), input.buffer);
        set_footer_cursor(f, layout[1], prompt.len());
    }
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

    let line = detail.input.as_ref().map_or_else(
        || {
            Line::from("j/k or up/down: move  h/l or left/right: column  Enter: edit  n: new  d: delete  s: save  Esc: back")
        },
        |input| {
            Line::from(vec![
                Span::raw(format!(
                    "Edit {}: {}",
                    detail.selected_col.label(),
                    input.buffer
                )),
                Span::raw("  Enter: confirm  Esc: cancel"),
            ])
        },
    );

    let footer = Paragraph::new(line);
    f.render_widget(footer, layout[1]);
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
    let cursor_x = area.x.saturating_add(offset.saturating_add(1)).min(max_x);
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

fn has_entry_today(goal: &GoalSummary) -> bool {
    let now = OffsetDateTime::now_utc();
    let today_date = UtcOffset::current_local_offset()
        .map_or_else(|_| now, |offset| now.to_offset(offset))
        .date();
    goal.lastday.date() == today_date
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

fn goal_pledge(goal: &GoalSummary) -> Option<f64> {
    goal.extra.get("pledge").and_then(serde_json::Value::as_f64)
}

fn parse_value_and_comment(input: &str) -> std::result::Result<(f64, Option<String>), String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter a value".to_string());
    }
    let mut parts = trimmed.splitn(2, |c: char| c.is_whitespace());
    let value_str = parts.next().unwrap_or("");
    let value = value_str
        .parse::<f64>()
        .map_err(|_| "Invalid value".to_string())?;
    let comment = parts
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    Ok((value, comment))
}

fn format_timestamp(ts: OffsetDateTime) -> String {
    let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    ts.format(format).unwrap_or_else(|_| ts.to_string())
}

fn parse_timestamp(input: &str) -> std::result::Result<OffsetDateTime, String> {
    let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    let naive = PrimitiveDateTime::parse(input, format)
        .map_err(|_| "Invalid timestamp (expected YYYY-MM-DD HH:MM:SS)".to_string())?;
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    Ok(naive.assume_offset(offset))
}
