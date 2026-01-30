//! Main application state and logic.

use crate::state::{
    clamp_index, DetailState, MainInput, Screen, StatusKind, StatusMessage, STATUS_TTL,
};
use anyhow::{Context, Result};
use beeconfig::BeeConfig;
use beeminder::types::{CreateDatapoint, GoalSummary};
use beeminder::BeeminderClient;
use ratatui::widgets::TableState;
use std::time::Instant;
use time::{OffsetDateTime, UtcOffset};
use tokio::runtime::Runtime;

/// Main application state.
pub struct App {
    pub config: BeeConfig,
    pub client: BeeminderClient,
    pub goals: Vec<GoalSummary>,
    pub filtered: Vec<usize>,
    pub filter: String,
    pub filter_backup: Option<String>,
    pub main_state: TableState,
    pub main_input: MainInput,
    pub screen: Screen,
    pub status: Option<StatusMessage>,
    pub last_success_goal: Option<(String, Instant)>,
}

impl App {
    pub fn new(config: BeeConfig, client: BeeminderClient) -> Self {
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

    pub fn refresh_goals(&mut self, runtime: &Runtime) -> Result<()> {
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

    pub fn refresh_filtered(&mut self) {
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

    pub fn selected_goal_index(&self) -> Option<usize> {
        let selected = self.main_state.selected()?;
        self.filtered.get(selected).copied()
    }

    pub fn selected_goal(&self) -> Option<&GoalSummary> {
        let idx = self.selected_goal_index()?;
        self.goals.get(idx)
    }

    pub fn select_goal_by_slug(&mut self, slug: &str) {
        if let Some((pos, _)) = self
            .filtered
            .iter()
            .enumerate()
            .find(|(_, idx)| self.goals.get(**idx).map(|g| g.slug.as_str()) == Some(slug))
        {
            self.main_state.select(Some(pos));
        }
    }

    pub fn set_status(&mut self, kind: StatusKind, text: String) {
        self.status = Some(StatusMessage {
            kind,
            text,
            created: Instant::now(),
        });
    }

    pub fn clear_expired_status(&mut self) {
        if let Some(status) = &self.status {
            if status.created.elapsed() > STATUS_TTL {
                self.status = None;
            }
        }
    }

    pub fn enter_filter_mode(&mut self) {
        self.filter_backup = Some(self.filter.clone());
        self.main_input = MainInput::Filter {
            buffer: self.filter.clone(),
        };
    }

    pub fn cancel_filter_mode(&mut self) {
        if let Some(prev) = self.filter_backup.take() {
            self.filter = prev;
            self.refresh_filtered();
        }
        self.main_input = MainInput::Normal;
    }

    pub fn apply_filter(&mut self, buffer: String) {
        self.filter = buffer;
        self.refresh_filtered();
        self.filter_backup = None;
        self.main_input = MainInput::Normal;
    }

    pub fn start_inline_add(&mut self) {
        if self.selected_goal().is_some() {
            self.main_input = MainInput::InlineAdd {
                buffer: String::new(),
            };
        } else {
            self.set_status(StatusKind::Info, "No goal selected".to_string());
        }
    }

    pub fn cancel_inline_add(&mut self) {
        self.main_input = MainInput::Normal;
    }

    pub fn submit_inline_add(&mut self, buffer: &str, runtime: &Runtime) {
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

    pub fn open_detail(&mut self, runtime: &Runtime) {
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

    pub fn move_main_selection(&mut self, delta: i32) {
        if self.filtered.is_empty() {
            return;
        }
        let selected = self.main_state.selected().unwrap_or(0);
        let max = self.filtered.len().saturating_sub(1);
        let next = clamp_index(selected, delta, max);
        self.main_state.select(Some(next));
    }
}

/// Check if a goal has an entry today (in local time).
pub fn has_entry_today(goal: &GoalSummary) -> bool {
    let now = OffsetDateTime::now_utc();
    let today_date = UtcOffset::current_local_offset()
        .map_or_else(|_| now, |offset| now.to_offset(offset))
        .date();
    goal.lastday.date() == today_date
}

/// Parse input as "value [comment]".
fn parse_value_and_comment(input: &str) -> Result<(f64, Option<String>), String> {
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
