use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;

/// Contract information for a goal (pledge amount and stepdown schedule)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    /// Amount at risk in USD
    #[serde(default)]
    pub amount: f64,
    /// Scheduled time for pledge stepdown
    #[serde(default, with = "time::serde::timestamp::option")]
    pub stepdown_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Goal {
    /// Unique identifier as hex string, useful when slugs change
    pub id: String,
    /// Final part of goal URL, used as identifier (e.g., "weight" in beeminder.com/alice/weight)
    pub slug: String,
    /// User-specified title for the goal
    pub title: String,
    /// List of datapoints for this goal
    #[serde(default)]
    pub datapoints: Vec<Datapoint>,
    /// Summary of what needs to be done by when, e.g., "+2 within 1 day".
    pub limsum: String,
    /// Unix timestamp of the last time this goal was updated
    #[serde(with = "time::serde::timestamp")]
    pub updated_at: OffsetDateTime,
    /// User-provided description of what exactly they are committing to
    #[serde(default)]
    pub fineprint: Option<String>,
    /// Label for the y-axis of the graph
    pub yaxis: String,
    /// Unix timestamp of the goal date
    #[serde(default, with = "time::serde::timestamp::option")]
    pub goaldate: Option<OffsetDateTime>,
    /// Goal value - the number the bright red line will eventually reach
    #[serde(default)]
    pub goalval: Option<f64>,
    /// Slope of the (final section of the) bright red line, paired with runits
    #[serde(default)]
    pub rate: Option<f64>,
    /// Rate units: y/m/w/d/h for yearly/monthly/weekly/daily/hourly
    pub runits: String,
    /// URL for the goal's graph SVG
    pub svg_url: String,
    /// URL for the goal's graph image
    pub graph_url: String,
    /// URL for the goal's graph thumbnail image
    pub thumb_url: String,
    /// Name of automatic data source, null for manual goals
    #[serde(default)]
    pub autodata: Option<String>,
    /// Type of goal (hustler/biker/fatloser/gainer/inboxer/drinker/custom)
    pub goal_type: String,
    /// Unix timestamp of derailment if nothing is reported
    #[serde(with = "time::serde::timestamp")]
    pub losedate: OffsetDateTime,
    /// Key for sorting goals by decreasing urgency
    pub urgencykey: String,
    /// Whether the graph is currently being updated
    #[serde(default)]
    pub queued: bool,
    /// Whether goal requires login to view
    #[serde(default)]
    pub secret: bool,
    /// Whether datapoints require login to view
    #[serde(default)]
    pub datapublic: bool,
    /// Amount pledged in USD on the goal
    #[serde(default)]
    pub pledge: f64,
    /// Number of days until derailment (0 if in beemergency)
    #[serde(default)]
    pub safebuf: i32,
    /// Unix timestamp of the last (explicitly entered) datapoint
    #[serde(with = "time::serde::timestamp")]
    pub lastday: OffsetDateTime,

    // Additional commonly-used fields

    /// Whether the goal is frozen/paused
    #[serde(default)]
    pub frozen: bool,
    /// Whether the goal was successfully completed
    #[serde(default)]
    pub won: bool,
    /// Whether the goal is currently off track
    #[serde(default)]
    pub lost: bool,
    /// Whether the goal is cumulative (auto-summing)
    #[serde(default)]
    pub kyoom: bool,
    /// Whether to treat zeros as odometer resets
    #[serde(default)]
    pub odom: bool,
    /// Goal units (e.g., "hours", "pushups")
    #[serde(default)]
    pub gunits: String,
    /// Distance from the red line to today's datapoint
    #[serde(default)]
    pub delta: f64,
    /// Value needed to get one more day of safety buffer
    #[serde(default)]
    pub safebump: f64,
    /// Max days of safety buffer (autoratchet setting)
    #[serde(default)]
    pub autoratchet: Option<f64>,
    /// Whether to assume integer values
    #[serde(default)]
    pub integery: bool,
    /// List of goal tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Seconds offset from midnight for deadline (-17*3600 to 6*3600)
    #[serde(default)]
    pub deadline: i64,
    /// Days before derailing to start reminders
    #[serde(default)]
    pub leadtime: i64,
    /// The last datapoint entered
    #[serde(default)]
    pub last_datapoint: Option<Datapoint>,
    /// Contract/pledge information
    #[serde(default)]
    pub contract: Option<Contract>,

    /// Catch-all for any additional fields from the API
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoalSummary {
    /// Final part of goal URL, used as identifier (e.g., "weight" in beeminder.com/alice/weight)
    pub slug: String,
    /// User-specified title for the goal
    pub title: String,
    /// Type of goal (hustler/biker/fatloser/gainer/inboxer/drinker/custom)
    pub goal_type: String,
    /// Summary of what needs to be done by when, e.g., "+2 within 1 day".
    pub limsum: String,
    /// URL for the goal's graph SVG
    pub svg_url: String,
    /// URL for the goal's graph image
    pub graph_url: String,
    /// URL for the goal's graph thumbnail image
    pub thumb_url: String,
    /// Unix timestamp of derailment if nothing is reported
    #[serde(with = "time::serde::timestamp")]
    pub losedate: OffsetDateTime,
    /// Unix timestamp of the goal date
    #[serde(default, with = "time::serde::timestamp::option")]
    pub goaldate: Option<OffsetDateTime>,
    /// Goal value - the number the bright red line will eventually reach
    #[serde(default)]
    pub goalval: Option<f64>,
    /// Slope of the (final section of the) bright red line
    #[serde(default)]
    pub rate: Option<f64>,
    /// Unix timestamp of the last time this goal was updated
    #[serde(with = "time::serde::timestamp")]
    pub updated_at: OffsetDateTime,
    /// Whether the graph is currently being updated
    #[serde(default)]
    pub queued: bool,
    /// Number of days until derailment (0 if in beemergency)
    #[serde(default)]
    pub safebuf: i32,
    /// Unix timestamp of the last (explicitly entered) datapoint
    #[serde(with = "time::serde::timestamp")]
    pub lastday: OffsetDateTime,

    /// Catch-all for any additional fields from the API
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Datapoint {
    /// A unique ID, used to identify a datapoint when deleting or editing it
    pub id: String,
    /// Unix timestamp (in seconds) of the datapoint
    #[serde(with = "time::serde::timestamp")]
    pub timestamp: OffsetDateTime,
    /// Date of the datapoint (e.g., "20150831"), accounts for goal deadlines
    pub daystamp: String,
    /// The value measured at this datapoint
    pub value: f64,
    /// Optional comment about the datapoint
    pub comment: Option<String>,
    /// Unix timestamp when this datapoint was entered or last updated
    #[serde(with = "time::serde::timestamp")]
    pub updated_at: OffsetDateTime,
    /// Echo of API request ID if provided during creation
    pub requestid: Option<String>,
    /// Where the datapoint came from (e.g., "web", "api", "duolingo")
    #[serde(default)]
    pub origin: Option<String>,
    /// User who created the datapoint (for group goals)
    #[serde(default)]
    pub creator: Option<String>,
    /// True if this is a system-generated datapoint (e.g., #DERAIL)
    #[serde(default)]
    pub is_dummy: Option<bool>,
    /// True if this is the initial datapoint added at goal creation
    #[serde(default)]
    pub is_initial: Option<bool>,
    /// Timestamp when the datapoint was created (may differ from timestamp)
    #[serde(default, with = "time::serde::timestamp::option")]
    pub created_at: Option<OffsetDateTime>,
}

/// Parameters for creating or updating a datapoint
#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDatapoint {
    /// The value to record
    pub value: f64,
    /// Timestamp for the datapoint, defaults to now if None
    #[serde(
        default,
        with = "time::serde::timestamp::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp: Option<OffsetDateTime>,
    /// Date string (e.g. "20150831"), alternative to timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daystamp: Option<String>,
    /// Optional comment
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Optional unique identifier for deduplication/updates
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requestid: Option<String>,
}

impl CreateDatapoint {
    /// Creates a new datapoint with just a value, all other fields None
    pub fn new(value: f64) -> Self {
        Self {
            value,
            timestamp: None,
            daystamp: None,
            comment: None,
            requestid: None,
        }
    }

    /// Adds a timestamp
    pub fn with_timestamp(mut self, timestamp: OffsetDateTime) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Adds a daystamp
    pub fn with_daystamp(mut self, daystamp: &str) -> Self {
        self.daystamp = Some(daystamp.to_string());
        self
    }

    /// Adds a comment
    pub fn with_comment(mut self, comment: &str) -> Self {
        self.comment = Some(comment.to_string());
        self
    }

    /// Adds a request ID
    pub fn with_requestid(mut self, requestid: &str) -> Self {
        self.requestid = Some(requestid.to_string());
        self
    }
}

/// Parameters for updating an existing datapoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDatapoint {
    /// ID of the datapoint to update
    pub id: String,
    /// Optional new timestamp for the datapoint
    #[serde(
        with = "time::serde::timestamp::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp: Option<OffsetDateTime>,
    /// Optional new value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// Optional new comment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

impl From<&Datapoint> for UpdateDatapoint {
    fn from(datapoint: &Datapoint) -> Self {
        Self {
            id: datapoint.id.clone(),
            timestamp: Some(datapoint.timestamp),
            value: Some(datapoint.value),
            comment: datapoint.comment.clone(),
        }
    }
}

impl UpdateDatapoint {
    /// Creates an empty update for the given datapoint ID
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            timestamp: None,
            value: None,
            comment: None,
        }
    }

    /// Creates an update from an existing datapoint with no changes
    #[must_use]
    pub fn from_datapoint(datapoint: &Datapoint) -> Self {
        Self {
            id: datapoint.id.clone(),
            timestamp: Some(datapoint.timestamp),
            value: Some(datapoint.value),
            comment: datapoint.comment.clone(),
        }
    }

    /// Sets a new timestamp
    #[must_use]
    pub fn with_timestamp(mut self, timestamp: OffsetDateTime) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets a new value
    #[must_use]
    pub fn with_value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self
    }

    /// Sets a new comment
    #[must_use]
    pub fn with_comment(mut self, comment: &str) -> Self {
        self.comment = Some(comment.to_string());
        self
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    /// Username of the Beeminder account
    pub username: String,
    /// User's timezone, e.g. "America/Los_Angeles"
    pub timezone: String,
    /// Timestamp when this user's information was last updated
    #[serde(with = "time::serde::timestamp")]
    pub updated_at: OffsetDateTime,
    /// Current urgency load (priority level of pending tasks)
    pub urgency_load: u64,
    /// Whether the user has an unpaid subscription
    pub deadbeat: bool,
    /// List of the user's goal slugs
    pub goals: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfoDiff {
    /// Username of the Beeminder account
    pub username: String,
    /// User's timezone, e.g. "America/Los_Angeles"
    pub timezone: String,
    /// Timestamp when this user's information was last updated
    #[serde(with = "time::serde::timestamp")]
    pub updated_at: OffsetDateTime,
    /// List of user's goals with detailed information and datapoints
    pub goals: Vec<Goal>,
    /// List of goals that have been deleted since the diff timestamp
    pub deleted_goals: Vec<DeletedGoal>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeletedGoal {
    /// ID of the deleted goal
    pub id: String,
}
