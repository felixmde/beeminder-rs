use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct Goal {
    /// Unique identifier as hex string, useful when slugs change
    pub id: String,
    /// Final part of goal URL, used as identifier (e.g., "weight" in beeminder.com/alice/weight)  
    pub slug: String,
    /// User-specified title for the goal
    pub title: String,
    /// List of datapoints for this goal
    pub datapoints: Vec<Datapoint>,
    /// Summary of what needs to be done by when, e.g., "+2 within 1 day".
    pub limsum: String,
    /// Unix timestamp of the last time this goal was updated
    #[serde(with = "time::serde::timestamp")]
    pub updated_at: OffsetDateTime,
    /// User-provided description of what exactly they are committing to
    pub fineprint: Option<String>,
    /// Label for the y-axis of the graph
    pub yaxis: String,
    /// Unix timestamp of the goal date
    #[serde(with = "time::serde::timestamp::option")]
    pub goaldate: Option<OffsetDateTime>,
    /// Goal value - the number the bright red line will eventually reach
    pub goalval: Option<f64>,
    /// Slope of the (final section of the) bright red line, paired with runits
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
    pub autodata: Option<String>,
    /// Type of goal (hustler/biker/fatloser/gainer/inboxer/drinker/custom)
    pub goal_type: String,
    /// Unix timestamp of derailment if nothing is reported
    #[serde(with = "time::serde::timestamp")]
    pub losedate: OffsetDateTime,
    /// Key for sorting goals by decreasing urgency
    pub urgencykey: String,
    /// Whether the graph is currently being updated
    pub queued: bool,
    /// Whether goal requires login to view
    pub secret: bool,
    /// Whether datapoints require login to view
    pub datapublic: bool,
    /// Amount pledged in USD on the goal
    pub pledge: f64,
    /// Number of days until derailment (0 if in beemergency)
    pub safebuf: i32,
    /// Unix timestamp of the last (explicitly entered) datapoint
    #[serde(with = "time::serde::timestamp")]
    pub lastday: OffsetDateTime,
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
    #[serde(with = "time::serde::timestamp::option")]
    pub goaldate: Option<OffsetDateTime>,
    /// Goal value - the number the bright red line will eventually reach
    pub goalval: Option<f64>,
    /// Slope of the (final section of the) bright red line
    pub rate: Option<f64>,
    /// Unix timestamp of the last time this goal was updated
    #[serde(with = "time::serde::timestamp")]
    pub updated_at: OffsetDateTime,
    /// Whether the graph is currently being updated
    pub queued: bool,
    /// Number of days until derailment (0 if in beemergency)
    pub safebuf: i32,
    /// Unix timestamp of the last (explicitly entered) datapoint
    #[serde(with = "time::serde::timestamp")]
    pub lastday: OffsetDateTime,
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
}

/// Parameters for creating or updating a datapoint
#[must_use]
#[derive(Debug, Clone)]
pub struct CreateDatapoint {
    /// The value to record
    pub value: f64,
    /// Timestamp for the datapoint, defaults to now if None
    pub timestamp: Option<OffsetDateTime>,
    /// Date string (e.g. "20150831"), alternative to timestamp
    pub daystamp: Option<String>,
    /// Optional comment
    pub comment: Option<String>,
    /// Optional unique identifier for deduplication/updates
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
