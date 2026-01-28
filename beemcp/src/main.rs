#![allow(clippy::multiple_crate_versions)]

use anyhow::{Context, Result};
use beeconfig::BeeConfig;
use beeminder::types::{
    CreateAllResponse, CreateDatapoint, CreateGoal, Datapoint, GoalSummary, GoalType,
    UpdateDatapoint, UpdateGoal,
};
use beeminder::{BeeminderClient, Error as BeeminderError};
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct DatapointInput {
    value: f64,
    #[serde(default)]
    timestamp: Option<i64>,
    #[serde(default)]
    daystamp: Option<String>,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    requestid: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct BeeminderRequest {
    action: String,
    #[serde(default)]
    goal: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    goal_type: Option<String>,
    #[serde(default)]
    value: Option<f64>,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    timestamp: Option<i64>,
    #[serde(default)]
    daystamp: Option<String>,
    #[serde(default)]
    requestid: Option<String>,
    #[serde(default)]
    datapoint_id: Option<String>,
    #[serde(default)]
    datapoints: Option<Vec<DatapointInput>>,
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    count: Option<u64>,
    #[serde(default)]
    page: Option<u64>,
    #[serde(default)]
    per: Option<u64>,
    #[serde(default)]
    goalval: Option<f64>,
    #[serde(default)]
    rate: Option<f64>,
    #[serde(default)]
    goaldate: Option<i64>,
    #[serde(default)]
    runits: Option<String>,
    #[serde(default)]
    initval: Option<f64>,
    #[serde(default)]
    initday: Option<i64>,
    #[serde(default)]
    gunits: Option<String>,
    #[serde(default)]
    yaxis: Option<String>,
    #[serde(default)]
    fineprint: Option<String>,
    #[serde(default)]
    secret: Option<bool>,
    #[serde(default)]
    datapublic: Option<bool>,
    #[serde(default)]
    archived: Option<bool>,
    #[serde(default)]
    include_archived: Option<bool>,
    #[serde(default)]
    max_datapoints_per_goal: Option<u64>,
    #[serde(default)]
    max_goals: Option<u64>,
}

#[derive(Clone)]
struct BeeminderService {
    client: Arc<BeeminderClient>,
    tool_router: ToolRouter<Self>,
}

impl BeeminderService {
    fn new(client: BeeminderClient) -> Self {
        Self {
            client: Arc::new(client),
            tool_router: Self::tool_router(),
        }
    }
}

fn normalize_action(action: &str) -> String {
    action
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-', '_'], "")
}

fn parse_unix_timestamp(value: Option<i64>) -> Result<Option<OffsetDateTime>, String> {
    value
        .map(OffsetDateTime::from_unix_timestamp)
        .transpose()
        .map_err(|err| format!("Invalid unix timestamp: {err}"))
}

fn tool_text(message: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![Content::text(message.into())])
}

fn tool_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![Content::text(message.into())])
}

fn tool_json<T: Serialize>(value: &T) -> CallToolResult {
    serde_json::to_string_pretty(value)
        .map_or_else(|_| tool_error("Failed to serialize response"), tool_text)
}

fn format_http_error(status: u16, reason: &str, body: &str) -> String {
    let reason = if reason.is_empty() {
        "HTTP error"
    } else {
        reason
    };
    let mut output = format!("Beeminder API error ({status} {reason}):");

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(errors) = value.get("errors").and_then(|v| v.as_object()) {
            let mut lines = Vec::new();
            for (key, val) in errors {
                if let Some(arr) = val.as_array() {
                    for item in arr {
                        if let Some(text) = item.as_str() {
                            let normalized = text.replace('\n', " ");
                            lines.push(format!("{key}: {normalized}"));
                        } else {
                            lines.push(format!("{key}: {item}"));
                        }
                    }
                } else if let Some(text) = val.as_str() {
                    let normalized = text.replace('\n', " ");
                    lines.push(format!("{key}: {normalized}"));
                } else {
                    lines.push(format!("{key}: {val}"));
                }
            }
            if !lines.is_empty() {
                output.push('\n');
                for line in lines {
                    let _ = writeln!(output, "  - {line}");
                }
                return output;
            }
        }

        if let Ok(pretty) = serde_json::to_string_pretty(&value) {
            output.push('\n');
            output.push_str(&pretty);
            return output;
        }
    }

    if !body.trim().is_empty() {
        output.push('\n');
        output.push_str(body);
    }

    output
}

fn format_beeminder_error(err: &BeeminderError) -> String {
    match err {
        BeeminderError::HttpStatus {
            status,
            reason,
            body,
        } => format_http_error(*status, reason, body),
        BeeminderError::Http(inner) => format!("HTTP error: {inner}"),
        BeeminderError::Json(inner) => format!("JSON error: {inner}"),
    }
}

#[tool_router]
impl BeeminderService {
    #[tool(
        name = "beeminder",
        description = "Unified Beeminder tool. Use action plus optional fields.\n\nActions: list, list-archived, add, edit, get-datapoints, update-datapoint, delete-datapoint, backup, goal-create, goal-update, goal-refresh, add-batch, shortcircuit, stepdown, cancel-stepdown.\n\nNotes: goal-create requires goal (slug), title, goal_type, gunits, and exactly two of goalval/rate/goaldate. goal-update accepts archived=true/false. add-batch accepts datapoints[] with value + optional timestamp/comment/daystamp/requestid."
    )]
    async fn beeminder(
        &self,
        Parameters(request): Parameters<BeeminderRequest>,
    ) -> Result<CallToolResult, McpError> {
        let action = normalize_action(&request.action);
        let client = self.client.as_ref();

        let result = match action.as_str() {
            "list" | "listgoals" => match client.get_goals().await {
                Ok(goals) => tool_json(&goals),
                Err(err) => tool_error(format_beeminder_error(&err)),
            },
            "listarchived" | "listarchivedgoals" => match client.get_archived_goals().await {
                Ok(goals) => tool_json(&goals),
                Err(err) => tool_error(format_beeminder_error(&err)),
            },
            "add" | "adddatapoint" => {
                let Some(goal) = request.goal.as_deref() else {
                    return Ok(tool_error("Missing required field: goal"));
                };
                let Some(value) = request.value else {
                    return Ok(tool_error("Missing required field: value"));
                };
                let timestamp = match parse_unix_timestamp(request.timestamp) {
                    Ok(ts) => ts,
                    Err(err) => return Ok(tool_error(err)),
                };
                let mut datapoint = CreateDatapoint::new(value);
                if let Some(timestamp) = timestamp {
                    datapoint = datapoint.with_timestamp(timestamp);
                }
                if let Some(daystamp) = request.daystamp.as_deref() {
                    datapoint = datapoint.with_daystamp(daystamp);
                }
                if let Some(comment) = request.comment.as_deref() {
                    datapoint = datapoint.with_comment(comment);
                }
                if let Some(requestid) = request.requestid.as_deref() {
                    datapoint = datapoint.with_requestid(requestid);
                }

                match client.create_datapoint(goal, &datapoint).await {
                    Ok(datapoint) => tool_json(&datapoint),
                    Err(err) => tool_error(format_beeminder_error(&err)),
                }
            }
            "getdatapoints" | "edit" | "editdatapoints" => {
                let Some(goal) = request.goal.as_deref() else {
                    return Ok(tool_error("Missing required field: goal"));
                };
                let is_edit = matches!(action.as_str(), "edit" | "editdatapoints");
                let sort = request
                    .sort
                    .as_deref()
                    .or(if is_edit { Some("timestamp") } else { None });
                let count = request.count.or(if is_edit { Some(20) } else { None });

                match client
                    .get_datapoints(goal, sort, count, request.page, request.per)
                    .await
                {
                    Ok(datapoints) => tool_json(&datapoints),
                    Err(err) => tool_error(format_beeminder_error(&err)),
                }
            }
            "updatedatapoint" => {
                let Some(goal) = request.goal.as_deref() else {
                    return Ok(tool_error("Missing required field: goal"));
                };
                let Some(datapoint_id) = request.datapoint_id.as_deref() else {
                    return Ok(tool_error("Missing required field: datapoint_id"));
                };
                if request.value.is_none() && request.comment.is_none() && request.timestamp.is_none() {
                    return Ok(tool_error(
                        "Provide at least one of: value, comment, timestamp",
                    ));
                }
                let timestamp = match parse_unix_timestamp(request.timestamp) {
                    Ok(ts) => ts,
                    Err(err) => return Ok(tool_error(err)),
                };

                let update = UpdateDatapoint {
                    id: datapoint_id.to_string(),
                    timestamp,
                    value: request.value,
                    comment: request.comment.clone(),
                };

                match client.update_datapoint(goal, &update).await {
                    Ok(datapoint) => tool_json(&datapoint),
                    Err(err) => tool_error(format_beeminder_error(&err)),
                }
            }
            "deletedatapoint" => {
                let Some(goal) = request.goal.as_deref() else {
                    return Ok(tool_error("Missing required field: goal"));
                };
                let Some(datapoint_id) = request.datapoint_id.as_deref() else {
                    return Ok(tool_error("Missing required field: datapoint_id"));
                };

                match client.delete_datapoint(goal, datapoint_id).await {
                    Ok(datapoint) => tool_json(&datapoint),
                    Err(err) => tool_error(format_beeminder_error(&err)),
                }
            }
            "goalcreate" => {
                let Some(goal) = request.goal.as_deref() else {
                    return Ok(tool_error("Missing required field: goal"));
                };
                let Some(title) = request.title.as_deref() else {
                    return Ok(tool_error("Missing required field: title"));
                };
                let Some(goal_type) = request.goal_type.as_deref() else {
                    return Ok(tool_error("Missing required field: goal_type"));
                };
                let goal_type = match goal_type.parse::<GoalType>() {
                    Ok(parsed) => parsed,
                    Err(err) => return Ok(tool_error(err.to_string())),
                };
                let Some(gunits) = request.gunits.clone() else {
                    return Ok(tool_error("Missing required field: gunits"));
                };
                let trio_count = u8::from(request.goalval.is_some())
                    + u8::from(request.rate.is_some())
                    + u8::from(request.goaldate.is_some());
                if trio_count != 2 {
                    return Ok(tool_error(
                        "Goal creation requires exactly two of: goalval, rate, goaldate",
                    ));
                }

                let goaldate = match parse_unix_timestamp(request.goaldate) {
                    Ok(ts) => ts,
                    Err(err) => return Ok(tool_error(err)),
                };
                let initday = match parse_unix_timestamp(request.initday) {
                    Ok(ts) => ts,
                    Err(err) => return Ok(tool_error(err)),
                };

                let mut create = CreateGoal::new(goal, title, goal_type);
                create.goalval = request.goalval;
                create.rate = request.rate;
                create.goaldate = goaldate;
                create.runits.clone_from(&request.runits);
                create.initval = request.initval;
                create.initday = initday;
                create.gunits = Some(gunits);
                create.yaxis.clone_from(&request.yaxis);
                create.fineprint.clone_from(&request.fineprint);
                create.secret = request.secret;
                create.datapublic = request.datapublic;

                match client.create_goal(&create).await {
                    Ok(goal) => tool_json(&goal),
                    Err(err) => tool_error(format_beeminder_error(&err)),
                }
            }
            "goalupdate" => {
                let Some(goal) = request.goal.as_deref() else {
                    return Ok(tool_error("Missing required field: goal"));
                };
                if request.title.is_none()
                    && request.goalval.is_none()
                    && request.rate.is_none()
                    && request.goaldate.is_none()
                    && request.runits.is_none()
                    && request.yaxis.is_none()
                    && request.fineprint.is_none()
                    && request.secret.is_none()
                    && request.datapublic.is_none()
                    && request.archived.is_none()
                {
                    return Ok(tool_error(
                        "Provide at least one field to update (title, goalval, rate, goaldate, runits, yaxis, fineprint, secret, datapublic, archived)",
                    ));
                }

                let goaldate = match parse_unix_timestamp(request.goaldate) {
                    Ok(ts) => ts,
                    Err(err) => return Ok(tool_error(err)),
                };

                let mut update = UpdateGoal::new();
                update.title.clone_from(&request.title);
                update.goalval = request.goalval;
                update.rate = request.rate;
                update.goaldate = goaldate;
                update.runits.clone_from(&request.runits);
                update.yaxis.clone_from(&request.yaxis);
                update.fineprint.clone_from(&request.fineprint);
                update.secret = request.secret;
                update.datapublic = request.datapublic;
                update.archived = request.archived;

                match client.update_goal(goal, &update).await {
                    Ok(goal) => tool_json(&goal),
                    Err(err) => tool_error(format_beeminder_error(&err)),
                }
            }
            "goalrefresh" => {
                let Some(goal) = request.goal.as_deref() else {
                    return Ok(tool_error("Missing required field: goal"));
                };
                match client.refresh_graph(goal).await {
                    Ok(refreshed) => tool_json(&refreshed),
                    Err(err) => tool_error(format_beeminder_error(&err)),
                }
            }
            "addbatch" | "createall" => {
                let Some(goal) = request.goal.as_deref() else {
                    return Ok(tool_error("Missing required field: goal"));
                };
                let datapoints = match request.datapoints.as_ref() {
                    Some(datapoints) if !datapoints.is_empty() => datapoints,
                    _ => return Ok(tool_error("Provide datapoints as a non-empty array")),
                };

                let mut payload = Vec::with_capacity(datapoints.len());
                for input in datapoints {
                    let timestamp = match parse_unix_timestamp(input.timestamp) {
                        Ok(ts) => ts,
                        Err(err) => return Ok(tool_error(err)),
                    };
                    let mut datapoint = CreateDatapoint::new(input.value);
                    if let Some(timestamp) = timestamp {
                        datapoint = datapoint.with_timestamp(timestamp);
                    }
                    if let Some(daystamp) = input.daystamp.as_deref() {
                        datapoint = datapoint.with_daystamp(daystamp);
                    }
                    if let Some(comment) = input.comment.as_deref() {
                        datapoint = datapoint.with_comment(comment);
                    }
                    if let Some(requestid) = input.requestid.as_deref() {
                        datapoint = datapoint.with_requestid(requestid);
                    }
                    payload.push(datapoint);
                }

                match client.create_all_datapoints(goal, &payload).await {
                    Ok(CreateAllResponse::Success(successes)) => tool_json(&successes),
                    Ok(CreateAllResponse::Partial { successes, errors }) => {
                        tool_json(&serde_json::json!({
                            "successes": successes,
                            "errors": errors,
                        }))
                    }
                    Err(err) => tool_error(format_beeminder_error(&err)),
                }
            }
            "shortcircuit" => {
                let Some(goal) = request.goal.as_deref() else {
                    return Ok(tool_error("Missing required field: goal"));
                };
                match client.shortcircuit(goal).await {
                    Ok(goal) => tool_json(&goal),
                    Err(err) => tool_error(format_beeminder_error(&err)),
                }
            }
            "stepdown" => {
                let Some(goal) = request.goal.as_deref() else {
                    return Ok(tool_error("Missing required field: goal"));
                };
                match client.stepdown(goal).await {
                    Ok(goal) => tool_json(&goal),
                    Err(err) => tool_error(format_beeminder_error(&err)),
                }
            }
            "cancelstepdown" => {
                let Some(goal) = request.goal.as_deref() else {
                    return Ok(tool_error("Missing required field: goal"));
                };
                match client.cancel_stepdown(goal).await {
                    Ok(goal) => tool_json(&goal),
                    Err(err) => tool_error(format_beeminder_error(&err)),
                }
            }
            "backup" => match backup(client, &request).await {
                Ok(data) => tool_json(&data),
                Err(err) => tool_error(err),
            },
            _ => tool_error("Unknown action. Try: list, add, edit, goal-create, goal-update, goal-refresh, add-batch, shortcircuit, stepdown, cancel-stepdown, get-datapoints, update-datapoint, delete-datapoint, backup"),
        };

        Ok(result)
    }
}

#[tool_handler]
impl ServerHandler for BeeminderService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Use the beeminder tool with an action field. Example: {action: \"list\"}. For goal-create, provide goal/title/goal_type, gunits, and exactly two of goalval/rate/goaldate.".to_string(),
            ),
            ..Default::default()
        }
    }
}

#[derive(Serialize)]
struct BackupData {
    metadata: BackupMetadata,
    goals: BackupGoals,
}

#[derive(Serialize)]
struct BackupMetadata {
    backup_timestamp: OffsetDateTime,
    beemcp_version: String,
}

#[derive(Serialize)]
struct BackupGoals {
    active: Vec<GoalWithDatapoints>,
    archived: Vec<GoalWithDatapoints>,
}

#[derive(Serialize)]
struct GoalWithDatapoints {
    goal: GoalSummary,
    datapoints: Vec<Datapoint>,
}

async fn backup(
    client: &BeeminderClient,
    request: &BeeminderRequest,
) -> Result<BackupData, String> {
    let max_datapoints = request.max_datapoints_per_goal;
    let include_archived = request.include_archived.unwrap_or(true);
    let max_goals = match request.max_goals {
        Some(value) => Some(
            usize::try_from(value).map_err(|_| "max_goals exceeds the maximum supported size")?,
        ),
        None => None,
    };

    let mut active_goals = client
        .get_goals()
        .await
        .map_err(|err| format_beeminder_error(&err))?;
    if let Some(limit) = max_goals {
        active_goals.truncate(limit);
    }

    let mut archived_goals = if include_archived {
        client
            .get_archived_goals()
            .await
            .map_err(|err| format_beeminder_error(&err))?
    } else {
        Vec::new()
    };
    if let Some(limit) = max_goals {
        archived_goals.truncate(limit);
    }

    let mut active = Vec::new();
    for goal in active_goals {
        let datapoints = client
            .get_datapoints(&goal.slug, Some("timestamp"), max_datapoints, None, None)
            .await
            .map_err(|err| format_beeminder_error(&err))?;
        active.push(GoalWithDatapoints { goal, datapoints });
    }

    let mut archived = Vec::new();
    for goal in archived_goals {
        let datapoints = client
            .get_datapoints(&goal.slug, Some("timestamp"), max_datapoints, None, None)
            .await
            .map_err(|err| format_beeminder_error(&err))?;
        archived.push(GoalWithDatapoints { goal, datapoints });
    }

    Ok(BackupData {
        metadata: BackupMetadata {
            backup_timestamp: OffsetDateTime::now_utc(),
            beemcp_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        goals: BackupGoals { active, archived },
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = BeeConfig::load_or_onboard().with_context(|| "Failed to load beeminder config")?;
    let api_key = config
        .api_key()
        .with_context(|| "Missing api_key in beeminder config")?;

    let service = BeeminderService::new(BeeminderClient::new(api_key));
    let server = service.serve(stdio()).await?;
    server.waiting().await?;
    Ok(())
}
