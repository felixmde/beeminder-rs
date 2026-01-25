use anyhow::{Context, Result};
use beeconfig::BeeConfig;
use beeminder::types::{CreateDatapoint, GoalSummary};
use beeminder::BeeminderClient;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::{Color, Colorize};
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
    let config =
        BeeConfig::load_or_onboard().with_context(|| "Failed to load beeminder config")?;
    let api_key = config
        .api_key()
        .with_context(|| "Missing api_key in beeminder config")?;
    Ok(BeeminderClient::new(api_key))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

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
    }

    Ok(())
}
