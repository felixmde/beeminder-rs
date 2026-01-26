use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;

// =============================================================================
// EFFICIENT TYPES - Lean structs with commonly-needed fields
// =============================================================================

/// Efficient datapoint representation with 7 commonly-needed fields.
/// Use `DatapointFull` if you need all API fields.
#[derive(Debug, Serialize, Deserialize)]
pub struct Datapoint {
    /// A unique ID, used to identify a datapoint when deleting or editing it
    pub id: String,
    /// The value measured at this datapoint
    pub value: f64,
    /// Unix timestamp (in seconds) of the datapoint
    #[serde(with = "time::serde::timestamp")]
    pub timestamp: OffsetDateTime,
    /// Date of the datapoint (e.g., "20150831"), accounts for goal deadlines
    pub daystamp: String,
    /// Optional comment about the datapoint
    pub comment: Option<String>,
    /// Unix timestamp when this datapoint was entered or last updated
    #[serde(with = "time::serde::timestamp")]
    pub updated_at: OffsetDateTime,
    /// Echo of API request ID if provided during creation
    pub requestid: Option<String>,
}

/// Efficient goal representation with ~22 commonly-needed fields.
/// Use `GoalFull` if you need all API fields.
#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Goal {
    // Identification
    /// Unique identifier as hex string, useful when slugs change
    pub id: String,
    /// Final part of goal URL, used as identifier (e.g., "weight" in beeminder.com/alice/weight)
    pub slug: String,
    /// User-specified title for the goal
    pub title: String,

    // Urgency/Status
    /// Number of days until derailment (0 if in beemergency)
    #[serde(default)]
    pub safebuf: i32,
    /// Unix timestamp of derailment if nothing is reported
    #[serde(with = "time::serde::timestamp")]
    pub losedate: OffsetDateTime,
    /// Summary of what needs to be done by when, e.g., "+2 within 1 day"
    pub limsum: String,

    // Target info
    /// Amount pledged in USD on the goal
    #[serde(default)]
    pub pledge: f64,
    /// Goal value - the number the bright red line will eventually reach
    #[serde(default)]
    pub goalval: Option<f64>,
    /// Slope of the (final section of the) bright red line, paired with runits
    #[serde(default)]
    pub rate: Option<f64>,
    /// Unix timestamp of the goal date
    #[serde(default, with = "time::serde::timestamp::option")]
    pub goaldate: Option<OffsetDateTime>,

    // Type/Display
    /// Type of goal (hustler/biker/fatloser/gainer/inboxer/drinker/custom)
    pub goal_type: String,
    /// Goal units (e.g., "hours", "pushups")
    #[serde(default)]
    pub gunits: String,
    /// Label for the y-axis of the graph
    pub yaxis: String,
    /// URL for the goal's graph image
    pub graph_url: String,
    /// URL for the goal's graph thumbnail image
    pub thumb_url: String,

    // State flags
    /// Whether the goal is frozen/paused
    #[serde(default)]
    pub frozen: bool,
    /// Whether the goal was successfully completed
    #[serde(default)]
    pub won: bool,
    /// Whether the goal is currently off track
    #[serde(default)]
    pub lost: bool,
    /// Whether the graph is currently being updated
    #[serde(default)]
    pub queued: bool,

    // Timestamps
    /// Unix timestamp of the last time this goal was updated
    #[serde(with = "time::serde::timestamp")]
    pub updated_at: OffsetDateTime,
    /// Unix timestamp of the last (explicitly entered) datapoint
    #[serde(with = "time::serde::timestamp")]
    pub lastday: OffsetDateTime,
}

// =============================================================================
// FULL TYPES - Complete structs with all API fields + catch-all HashMap
// =============================================================================

/// Full datapoint representation with all API fields.
/// Core identity fields (id, timestamp, daystamp) are non-optional.
#[derive(Debug, Serialize, Deserialize)]
pub struct DatapointFull {
    // Always present (non-optional)
    /// A unique ID, used to identify a datapoint when deleting or editing it
    pub id: String,
    /// Unix timestamp (in seconds) of the datapoint
    #[serde(with = "time::serde::timestamp")]
    pub timestamp: OffsetDateTime,
    /// Date of the datapoint (e.g., "20150831"), accounts for goal deadlines
    pub daystamp: String,

    // Everything else optional
    /// The value measured at this datapoint
    pub value: Option<f64>,
    /// Optional comment about the datapoint
    pub comment: Option<String>,
    /// Unix timestamp when this datapoint was entered or last updated
    #[serde(default, with = "time::serde::timestamp::option")]
    pub updated_at: Option<OffsetDateTime>,
    /// Echo of API request ID if provided during creation
    pub requestid: Option<String>,
    /// Where the datapoint came from (e.g., "web", "api", "duolingo")
    pub origin: Option<String>,
    /// User who created the datapoint (for group goals)
    pub creator: Option<String>,
    /// True if this is a system-generated datapoint (e.g., #DERAIL)
    pub is_dummy: Option<bool>,
    /// True if this is the initial datapoint added at goal creation
    pub is_initial: Option<bool>,
    /// Timestamp when the datapoint was created (ISO 8601 format from API)
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub created_at: Option<OffsetDateTime>,

    /// Catch-all for any additional fields from the API
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Contract information for a goal (pledge amount and stepdown schedule)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    /// Amount at risk in USD
    #[serde(default)]
    pub amount: Option<f64>,
    /// Scheduled time for pledge stepdown
    #[serde(default, with = "time::serde::timestamp::option")]
    pub stepdown_at: Option<OffsetDateTime>,
}

/// Full goal representation with all API fields.
/// Core identity fields (id, slug) are non-optional.
#[derive(Debug, Serialize, Deserialize)]
pub struct GoalFull {
    // Always present (non-optional)
    /// Unique identifier as hex string, useful when slugs change
    pub id: String,
    /// Final part of goal URL, used as identifier
    pub slug: String,

    // All other documented fields as Option<T>
    /// User-specified title for the goal
    pub title: Option<String>,
    /// Number of days until derailment (0 if in beemergency)
    pub safebuf: Option<i32>,
    /// Unix timestamp of derailment if nothing is reported
    #[serde(default, with = "time::serde::timestamp::option")]
    pub losedate: Option<OffsetDateTime>,
    /// Summary of what needs to be done by when
    pub limsum: Option<String>,
    /// Amount pledged in USD on the goal
    pub pledge: Option<f64>,
    /// Goal value - the number the bright red line will eventually reach
    pub goalval: Option<f64>,
    /// Slope of the (final section of the) bright red line
    pub rate: Option<f64>,
    /// Unix timestamp of the goal date
    #[serde(default, with = "time::serde::timestamp::option")]
    pub goaldate: Option<OffsetDateTime>,
    /// Type of goal (hustler/biker/fatloser/gainer/inboxer/drinker/custom)
    pub goal_type: Option<String>,
    /// Goal units (e.g., "hours", "pushups")
    pub gunits: Option<String>,
    /// Label for the y-axis of the graph
    pub yaxis: Option<String>,
    /// URL for the goal's graph image
    pub graph_url: Option<String>,
    /// URL for the goal's graph thumbnail image
    pub thumb_url: Option<String>,
    /// URL for the goal's graph SVG
    pub svg_url: Option<String>,
    /// Whether the goal is frozen/paused
    pub frozen: Option<bool>,
    /// Whether the goal was successfully completed
    pub won: Option<bool>,
    /// Whether the goal is currently off track
    pub lost: Option<bool>,
    /// Whether the graph is currently being updated
    pub queued: Option<bool>,
    /// Whether goal requires login to view
    pub secret: Option<bool>,
    /// Whether datapoints require login to view
    pub datapublic: Option<bool>,
    /// Unix timestamp of the last time this goal was updated
    #[serde(default, with = "time::serde::timestamp::option")]
    pub updated_at: Option<OffsetDateTime>,
    /// Unix timestamp of the last (explicitly entered) datapoint
    #[serde(default, with = "time::serde::timestamp::option")]
    pub lastday: Option<OffsetDateTime>,
    /// User-provided description of what exactly they are committing to
    pub fineprint: Option<String>,
    /// Name of automatic data source, null for manual goals
    pub autodata: Option<String>,
    /// Key for sorting goals by decreasing urgency
    pub urgencykey: Option<String>,
    /// Whether the goal is cumulative (auto-summing)
    pub kyoom: Option<bool>,
    /// Whether to treat zeros as odometer resets
    pub odom: Option<bool>,
    /// How datapoints on the same day are aggregated
    pub aggday: Option<String>,
    /// Whether to plot all datapoints
    pub plotall: Option<bool>,
    /// Whether to show a steppy line
    pub steppy: Option<bool>,
    /// Whether to show a rosy line
    pub rosy: Option<bool>,
    /// Whether to show a moving average
    pub movingav: Option<bool>,
    /// Whether to show the aura
    pub aura: Option<bool>,
    /// Number of datapoints
    pub numpts: Option<i64>,
    /// Distance from the red line to today's datapoint
    pub delta: Option<f64>,
    /// Value needed to get one more day of safety buffer
    pub safebump: Option<f64>,
    /// Max days of safety buffer (autoratchet setting)
    pub autoratchet: Option<f64>,
    /// Whether to assume integer values
    pub integery: Option<bool>,
    /// Seconds offset from midnight for deadline
    pub deadline: Option<i64>,
    /// Days before derailing to start reminders
    pub leadtime: Option<i64>,
    /// Rate units: y/m/w/d/h for yearly/monthly/weekly/daily/hourly
    pub runits: Option<String>,
    /// Unix timestamp of the initial day
    #[serde(default, with = "time::serde::timestamp::option")]
    pub initday: Option<OffsetDateTime>,
    /// Initial value
    pub initval: Option<f64>,
    /// Unix timestamp of the current day
    #[serde(default, with = "time::serde::timestamp::option")]
    pub curday: Option<OffsetDateTime>,
    /// Current value
    pub curval: Option<f64>,
    /// Current rate
    pub currate: Option<f64>,
    /// Red line ahead
    pub rah: Option<f64>,
    /// Road data
    pub road: Option<serde_json::Value>,
    /// All road data
    pub roadall: Option<serde_json::Value>,
    /// Full road data
    pub fullroad: Option<serde_json::Value>,
    /// Contract/pledge information
    pub contract: Option<Contract>,
    /// List of goal tags
    pub tags: Option<Vec<String>>,
    /// List of datapoints for this goal
    pub datapoints: Option<Vec<DatapointFull>>,
    /// The last datapoint entered
    pub last_datapoint: Option<DatapointFull>,

    /// Catch-all for any additional fields from the API
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

// =============================================================================
// ALWAYS-FULL TYPES - No efficient variant needed
// =============================================================================

/// Summary information for a goal (used in goal lists)
#[derive(Debug, Serialize, Deserialize)]
pub struct GoalSummary {
    /// Final part of goal URL, used as identifier
    pub slug: String,
    /// User-specified title for the goal
    pub title: String,
    /// Type of goal (hustler/biker/fatloser/gainer/inboxer/drinker/custom)
    pub goal_type: String,
    /// Summary of what needs to be done by when
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
pub struct UserInfo {
    /// Username of the Beeminder account
    pub username: String,
    /// User's timezone, e.g. "`America/Los_Angeles`"
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
    /// User's timezone, e.g. "`America/Los_Angeles`"
    pub timezone: String,
    /// Timestamp when this user's information was last updated
    #[serde(with = "time::serde::timestamp")]
    pub updated_at: OffsetDateTime,
    /// List of user's goals with detailed information and datapoints
    pub goals: Vec<GoalFull>,
    /// List of goals that have been deleted since the diff timestamp
    pub deleted_goals: Vec<DeletedGoal>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeletedGoal {
    /// ID of the deleted goal
    pub id: String,
}

// =============================================================================
// REQUEST TYPES - For creating and updating data
// =============================================================================

/// Parameters for creating a datapoint
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
    pub const fn new(value: f64) -> Self {
        Self {
            value,
            timestamp: None,
            daystamp: None,
            comment: None,
            requestid: None,
        }
    }

    /// Adds a timestamp
    pub const fn with_timestamp(mut self, timestamp: OffsetDateTime) -> Self {
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
        default,
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
    pub const fn with_timestamp(mut self, timestamp: OffsetDateTime) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets a new value
    #[must_use]
    pub const fn with_value(mut self, value: f64) -> Self {
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

// =============================================================================
// REQUEST TYPES - Goals and batch datapoints
// =============================================================================

/// Supported Beeminder goal types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalType {
    Hustler,
    Biker,
    Fatloser,
    Gainer,
    Inboxer,
    Drinker,
    Custom,
}

impl GoalType {
    /// Canonical string values accepted by the API.
    pub const VALUES: [&'static str; 7] = [
        "hustler", "biker", "fatloser", "gainer", "inboxer", "drinker", "custom",
    ];

    /// Returns the canonical API string for this goal type.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hustler => "hustler",
            Self::Biker => "biker",
            Self::Fatloser => "fatloser",
            Self::Gainer => "gainer",
            Self::Inboxer => "inboxer",
            Self::Drinker => "drinker",
            Self::Custom => "custom",
        }
    }
}

impl std::fmt::Display for GoalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct GoalTypeParseError {
    value: String,
}

impl std::fmt::Display for GoalTypeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid goal type '{}'; expected one of: {}",
            self.value,
            GoalType::VALUES.join(", ")
        )
    }
}

impl std::error::Error for GoalTypeParseError {}

impl std::str::FromStr for GoalType {
    type Err = GoalTypeParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim().to_ascii_lowercase().replace([' ', '-'], "");
        let goal_type = match normalized.as_str() {
            "hustler" => Self::Hustler,
            "biker" => Self::Biker,
            "fatloser" => Self::Fatloser,
            "gainer" => Self::Gainer,
            "inboxer" => Self::Inboxer,
            "drinker" => Self::Drinker,
            "custom" => Self::Custom,
            _ => {
                return Err(GoalTypeParseError {
                    value: value.to_string(),
                })
            }
        };
        Ok(goal_type)
    }
}

impl From<GoalType> for String {
    fn from(value: GoalType) -> Self {
        value.as_str().to_string()
    }
}

/// Parameters for creating a goal
#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGoal {
    /// Goal slug (URL identifier)
    pub slug: String,
    /// Goal title
    pub title: String,
    /// Goal type (hustler/biker/fatloser/gainer/inboxer/drinker/custom)
    pub goal_type: String,
    /// Goal value - the number the bright red line will eventually reach
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goalval: Option<f64>,
    /// Slope of the bright red line
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<f64>,
    /// Unix timestamp of the goal date
    #[serde(
        with = "time::serde::timestamp::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub goaldate: Option<OffsetDateTime>,
    /// Rate units: y/m/w/d/h for yearly/monthly/weekly/daily/hourly
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runits: Option<String>,
    /// Initial value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initval: Option<f64>,
    /// Unix timestamp of the initial day
    #[serde(
        with = "time::serde::timestamp::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub initday: Option<OffsetDateTime>,
    /// Goal units (e.g., "hours", "pushups")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gunits: Option<String>,
    /// Label for the y-axis of the graph
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yaxis: Option<String>,
    /// Whether goal requires login to view
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<bool>,
    /// Whether datapoints require login to view
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datapublic: Option<bool>,
    /// User-provided description of what exactly they are committing to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fineprint: Option<String>,
}

impl CreateGoal {
    /// Creates a new goal with required fields
    pub fn new(
        slug: impl Into<String>,
        title: impl Into<String>,
        goal_type: impl Into<String>,
    ) -> Self {
        Self {
            slug: slug.into(),
            title: title.into(),
            goal_type: goal_type.into(),
            goalval: None,
            rate: None,
            goaldate: None,
            runits: None,
            initval: None,
            initday: None,
            gunits: None,
            yaxis: None,
            secret: None,
            datapublic: None,
            fineprint: None,
        }
    }
}

/// Parameters for updating a goal
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateGoal {
    /// New title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// New goal value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goalval: Option<f64>,
    /// New rate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<f64>,
    /// New goal date
    #[serde(
        with = "time::serde::timestamp::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub goaldate: Option<OffsetDateTime>,
    /// New rate units
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runits: Option<String>,
    /// New y-axis label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yaxis: Option<String>,
    /// New fineprint/commitment description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fineprint: Option<String>,
    /// Whether goal requires login to view
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<bool>,
    /// Whether datapoints require login to view
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datapublic: Option<bool>,
    /// Whether goal is archived
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
}

impl UpdateGoal {
    /// Creates an empty update
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Response from creating multiple datapoints
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CreateAllResponse {
    /// All datapoints created successfully
    Success(Vec<DatapointFull>),
    /// Partial success with errors
    Partial {
        successes: Vec<DatapointFull>,
        errors: Vec<serde_json::Value>,
    },
}

/// Response from auth token endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthTokenResponse {
    /// The user's auth token (present when authenticated)
    pub auth_token: Option<String>,
    /// Error message (present when not authenticated)
    pub error: Option<String>,
}
