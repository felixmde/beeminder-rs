#![allow(clippy::multiple_crate_versions)]

use anyhow::{Context, Result};
use beeconfig::BeeConfig;
use beeminder::types::{
    CreateAllResponse, CreateDatapoint, CreateGoal, GoalSummary, GoalType, UpdateGoal,
};
use beeminder::{BeeminderClient, Error as BeeminderError};
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::{Color, Colorize};
use std::fmt::Write;
use std::fs;
use std::io::{self, Read};
use std::process;
use time::{OffsetDateTime, UtcOffset};
mod backup;
mod edit;

#[derive(Parser)]
#[command(name = "beeline", about = "A CLI for Beeminder")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List all goals
    List,
    /// Add a datapoint
    Add {
        /// The name of the goal
        goal: String,
        /// The value of the datapoint
        value: f64,
        /// An optional comment for the datapoint
        comment: Option<String>,
    },
    /// Edit recent datapoints for a goal
    Edit {
        /// The name of the goal
        goal: String,
    },
    /// Backup all user data to JSON file
    Backup {
        /// Output file name
        #[arg(default_value = "beedata.json")]
        filename: String,
    },
    /// Create a goal
    #[command(
        long_about = "Create a goal.\n\nRequirements:\n- goal_type must be one of: hustler, biker, fatloser, gainer, inboxer, drinker, custom\n- For most goal types, set exactly two of: --goalval, --rate, --goaldate\n- Goal units are required: --gunits\n\nExample:\n  beeline goal-create reading \"Reading\" hustler --goalval 10 --rate 1 --runits w --gunits pages"
    )]
    GoalCreate {
        /// Goal slug (URL identifier)
        slug: String,
        /// Goal title
        title: String,
        /// Goal type (hustler/biker/fatloser/gainer/inboxer/drinker/custom)
        goal_type: String,
        /// Goal value - the number the bright red line will eventually reach
        #[arg(long)]
        goalval: Option<f64>,
        /// Slope of the bright red line
        #[arg(long)]
        rate: Option<f64>,
        /// Unix timestamp for goal date
        #[arg(long)]
        goaldate: Option<i64>,
        /// Rate units: y/m/w/d/h
        #[arg(long)]
        runits: Option<String>,
        /// Initial value
        #[arg(long)]
        initval: Option<f64>,
        /// Unix timestamp for the initial day
        #[arg(long)]
        initday: Option<i64>,
        /// Goal units (e.g., "hours", "pushups")
        #[arg(long)]
        gunits: Option<String>,
        /// Label for the y-axis of the graph
        #[arg(long)]
        yaxis: Option<String>,
        /// User-provided description of what exactly they are committing to
        #[arg(long)]
        fineprint: Option<String>,
        /// Whether goal requires login to view (true/false)
        #[arg(long, value_parser = clap::value_parser!(bool))]
        secret: Option<bool>,
        /// Whether datapoints require login to view (true/false)
        #[arg(long, value_parser = clap::value_parser!(bool))]
        datapublic: Option<bool>,
    },
    /// Update a goal
    GoalUpdate {
        /// Goal slug (URL identifier)
        goal: String,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New goal value
        #[arg(long)]
        goalval: Option<f64>,
        /// New rate
        #[arg(long)]
        rate: Option<f64>,
        /// Unix timestamp for new goal date
        #[arg(long)]
        goaldate: Option<i64>,
        /// New rate units: y/m/w/d/h
        #[arg(long)]
        runits: Option<String>,
        /// New y-axis label
        #[arg(long)]
        yaxis: Option<String>,
        /// New fineprint/commitment description
        #[arg(long)]
        fineprint: Option<String>,
        /// Whether goal requires login to view (true/false)
        #[arg(long, value_parser = clap::value_parser!(bool))]
        secret: Option<bool>,
        /// Whether datapoints require login to view (true/false)
        #[arg(long, value_parser = clap::value_parser!(bool))]
        datapublic: Option<bool>,
        /// Archive or unarchive goal (true/false)
        #[arg(long, value_parser = clap::value_parser!(bool))]
        archived: Option<bool>,
    },
    /// Refresh a goal's graph (autodata refetch)
    GoalRefresh {
        /// Goal slug (URL identifier)
        goal: String,
    },
    /// Create multiple datapoints from a JSON array
    AddBatch {
        /// Goal slug (URL identifier)
        goal: String,
        /// Path to JSON file with datapoints (use - for stdin)
        file: String,
    },
    /// Short-circuit a goal (charges current pledge and increases pledge level)
    Shortcircuit {
        /// Goal slug (URL identifier)
        goal: String,
    },
    /// Schedule a pledge stepdown for a goal
    Stepdown {
        /// Goal slug (URL identifier)
        goal: String,
    },
    /// Cancel a pledge stepdown for a goal
    CancelStepdown {
        /// Goal slug (URL identifier)
        goal: String,
    },
    /// Generate shell completions
    #[command(hide = true)]
    Completions {
        /// The shell to generate completions for
        shell: Shell,
    },
    /// List goal names (for shell completion)
    #[command(hide = true)]
    ListGoals,
}

#[derive(Debug)]
pub struct EditableDatapoint {
    pub id: Option<String>,
    pub timestamp: Option<OffsetDateTime>,
    pub value: Option<f64>,
    pub comment: Option<String>,
}

fn has_entry_today(goal: &GoalSummary) -> bool {
    let now = OffsetDateTime::now_utc();
    let today_date = UtcOffset::current_local_offset()
        .map_or_else(|_| now, |offset| now.to_offset(offset))
        .date();
    goal.lastday.date() == today_date
}

fn format_goal(goal: &GoalSummary) -> String {
    let has_entry_today = if has_entry_today(goal) { "✓" } else { " " };
    let slug_padded = format!("{:20}", goal.slug);

    let color = match goal.safebuf {
        0 => Color::Red,
        1 => Color::Yellow,
        2 => Color::Blue,
        3..=6 => Color::Green,
        _ => Color::White,
    };

    format!("{} {} [{}]", has_entry_today, slug_padded, goal.limsum)
        .color(color)
        .to_string()
}

fn get_client() -> Result<BeeminderClient> {
    let config = BeeConfig::load_or_onboard().with_context(|| "Failed to load beeminder config")?;
    let api_key = config
        .api_key()
        .with_context(|| "Missing api_key in beeminder config")?;
    Ok(BeeminderClient::new(api_key))
}

fn parse_unix_timestamp(value: Option<i64>) -> Result<Option<OffsetDateTime>> {
    value
        .map(OffsetDateTime::from_unix_timestamp)
        .transpose()
        .map_err(|err| anyhow::anyhow!("Invalid unix timestamp: {err}"))
}

fn read_json_input(path: &str) -> Result<String> {
    if path == "-" {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .with_context(|| "Failed to read JSON from stdin")?;
        Ok(buffer)
    } else {
        fs::read_to_string(path).with_context(|| format!("Failed to read file: {path}"))
    }
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

fn handle_error(err: &anyhow::Error) -> ! {
    if let Some(BeeminderError::HttpStatus {
        status,
        reason,
        body,
    }) = err.downcast_ref::<BeeminderError>()
    {
        eprintln!("{}", format_http_error(*status, reason, body));
        process::exit(1);
    }

    eprintln!("{err}");
    process::exit(1);
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            if err.kind() == ErrorKind::MissingRequiredArgument {
                eprintln!("{err}");
                let msg = err.to_string();
                if msg.contains("goal-create") {
                    eprintln!(
                        "\nTip: beeline goal-create <slug> <title> <goal_type>\n  goal_type must be one of: {}\n  most goal types also require: --goalval --rate --runits --goaldate",
                        GoalType::VALUES.join(", ")
                    );
                }
                process::exit(2);
            }
            err.exit();
        }
    };

    if let Err(err) = run(cli).await {
        handle_error(&err);
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Completions { shell } => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "beeline", &mut std::io::stdout());
        }
        Command::ListGoals => {
            let client = get_client()?;
            let goals: Vec<GoalSummary> = client.get_goals().await?;
            for goal in goals {
                println!("{}", goal.slug);
            }
        }
        Command::List => {
            let client = get_client()?;
            let mut goals: Vec<GoalSummary> = client.get_goals().await?;

            goals.sort_by(|a, b| {
                let today_cmp = has_entry_today(a).cmp(&has_entry_today(b));
                if today_cmp != std::cmp::Ordering::Equal {
                    return today_cmp;
                }

                a.safebuf.cmp(&b.safebuf)
            });

            for goal in goals {
                println!("{}", format_goal(&goal));
            }
        }
        Command::Add {
            goal,
            value,
            comment,
        } => {
            let client = get_client()?;
            let mut dp = CreateDatapoint::new(value);
            if let Some(comment) = comment {
                dp = dp.with_comment(&comment);
            }
            client.create_datapoint(&goal, &dp).await?;
        }
        Command::Edit { goal } => {
            let client = get_client()?;
            edit::edit_datapoints(&client, &goal).await?;
        }
        Command::Backup { filename } => {
            let client = get_client()?;
            backup::backup_user_data(&client, &filename).await?;
        }
        Command::GoalCreate {
            slug,
            title,
            goal_type,
            goalval,
            rate,
            goaldate,
            runits,
            initval,
            initday,
            gunits,
            yaxis,
            fineprint,
            secret,
            datapublic,
        } => {
            let client = get_client()?;
            let trio_count = u8::from(goalval.is_some())
                + u8::from(rate.is_some())
                + u8::from(goaldate.is_some());
            if trio_count != 2 {
                return Err(anyhow::anyhow!(
                    "Goal creation requires exactly two of: --goalval, --rate, --goaldate"
                ));
            }
            let goal_type = goal_type.parse::<GoalType>()?;
            let mut goal = CreateGoal::new(slug, title, goal_type);
            goal.goalval = goalval;
            goal.rate = rate;
            goal.goaldate = parse_unix_timestamp(goaldate)?;
            goal.runits = runits;
            goal.initval = initval;
            goal.initday = parse_unix_timestamp(initday)?;
            goal.gunits = gunits;
            goal.yaxis = yaxis;
            goal.fineprint = fineprint;
            goal.secret = secret;
            goal.datapublic = datapublic;
            let created = client.create_goal(&goal).await?;
            println!("{}", created.slug);
        }
        Command::GoalUpdate {
            goal,
            title,
            goalval,
            rate,
            goaldate,
            runits,
            yaxis,
            fineprint,
            secret,
            datapublic,
            archived,
        } => {
            let client = get_client()?;
            let mut update = UpdateGoal::new();
            update.title = title;
            update.goalval = goalval;
            update.rate = rate;
            update.goaldate = parse_unix_timestamp(goaldate)?;
            update.runits = runits;
            update.yaxis = yaxis;
            update.fineprint = fineprint;
            update.secret = secret;
            update.datapublic = datapublic;
            update.archived = archived;
            let updated = client.update_goal(&goal, &update).await?;
            println!("{}", updated.slug);
        }
        Command::GoalRefresh { goal } => {
            let client = get_client()?;
            let refreshed = client.refresh_graph(&goal).await?;
            println!("{refreshed}");
        }
        Command::AddBatch { goal, file } => {
            let client = get_client()?;
            let payload = read_json_input(&file)?;
            let datapoints: Vec<CreateDatapoint> = serde_json::from_str(&payload)
                .with_context(|| "Failed to parse datapoints JSON array")?;
            let result = client.create_all_datapoints(&goal, &datapoints).await?;
            match result {
                CreateAllResponse::Success(successes) => {
                    println!("Created {} datapoints.", successes.len());
                }
                CreateAllResponse::Partial { successes, errors } => {
                    println!(
                        "Created {} datapoints with {} errors.",
                        successes.len(),
                        errors.len()
                    );
                    if !errors.is_empty() {
                        eprintln!(
                            "{}",
                            serde_json::to_string_pretty(&errors)
                                .unwrap_or_else(|_| "Failed to format errors".to_string())
                        );
                    }
                }
            }
        }
        Command::Shortcircuit { goal } => {
            let client = get_client()?;
            let updated = client.shortcircuit(&goal).await?;
            println!("{}", updated.slug);
        }
        Command::Stepdown { goal } => {
            let client = get_client()?;
            let updated = client.stepdown(&goal).await?;
            println!("{}", updated.slug);
        }
        Command::CancelStepdown { goal } => {
            let client = get_client()?;
            let updated = client.cancel_stepdown(&goal).await?;
            println!("{}", updated.slug);
        }
    }

    Ok(())
}
