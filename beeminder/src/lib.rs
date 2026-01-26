pub mod types;
use crate::types::{
    AuthTokenResponse, CreateAllResponse, CreateDatapoint, CreateGoal, Datapoint, DatapointFull,
    Goal, GoalFull, GoalSummary, UpdateDatapoint, UpdateGoal, UserInfo, UserInfoDiff,
};
use reqwest::Client;
use time::OffsetDateTime;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("HTTP status {status} {reason}: {body}")]
    HttpStatus {
        status: u16,
        reason: String,
        body: String,
    },
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct BeeminderClient {
    client: Client,
    api_key: String,
    base_url: String,
    username: String,
    emaciated: bool,
}

impl BeeminderClient {
    async fn parse_response<T>(response: reqwest::Response) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<failed to read body>".to_string());
            let reason = status
                .canonical_reason()
                .unwrap_or("HTTP error")
                .to_string();
            return Err(Error::HttpStatus {
                status: status.as_u16(),
                reason,
                body,
            });
        }
        response.json().await.map_err(Error::from)
    }

    async fn get<T, U>(&self, endpoint: &str, query: &U) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
        U: serde::ser::Serialize,
    {
        let response = self
            .client
            .get(format!("{}{}", self.base_url, endpoint))
            .query(&[("auth_token", self.api_key.as_str())])
            .query(&query)
            .send()
            .await?;
        Self::parse_response(response).await
    }

    async fn get_no_auth<T, U>(&self, endpoint: &str, query: &U) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
        U: serde::ser::Serialize,
    {
        let response = self
            .client
            .get(format!("{}{}", self.base_url, endpoint))
            .query(&query)
            .send()
            .await?;
        Self::parse_response(response).await
    }

    async fn post<T, U>(&self, endpoint: &str, query: &U) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
        U: serde::ser::Serialize,
    {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, endpoint))
            .query(&[("auth_token", self.api_key.as_str())])
            .form(query)
            .send()
            .await?;
        Self::parse_response(response).await
    }

    async fn put<T, U>(&self, endpoint: &str, query: &U) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
        U: serde::ser::Serialize,
    {
        let response = self
            .client
            .put(format!("{}{}", self.base_url, endpoint))
            .query(&[("auth_token", self.api_key.as_str())])
            .form(query)
            .send()
            .await?;
        Self::parse_response(response).await
    }

    async fn delete<T, U>(&self, endpoint: &str, query: &U) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
        U: serde::ser::Serialize,
    {
        let response = self
            .client
            .delete(format!("{}{}", self.base_url, endpoint))
            .query(&[("auth_token", self.api_key.as_str())])
            .query(query)
            .send()
            .await?;
        Self::parse_response(response).await
    }

    /// Creates a new `BeeminderClient` with the given API key.
    /// Default username is set to 'me'.
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://www.beeminder.com/api/v1/".to_string(),
            username: "me".to_string(),
            emaciated: false,
        }
    }

    /// Sets a username for this client.
    #[must_use]
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = username.into();
        self
    }

    /// Enables emaciated mode, stripping road/roadall/fullroad from goal responses.
    /// Default is false.
    #[must_use]
    pub fn with_emaciated(mut self, emaciated: bool) -> Self {
        self.emaciated = emaciated;
        self
    }

    /// Sets a custom base URL for this client.
    /// Useful for testing with mock servers.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Retrieves user information for user associated with client.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_user(&self) -> Result<UserInfo, Error> {
        let query: Vec<(&str, &str)> = if self.emaciated {
            vec![("emaciated", "true")]
        } else {
            vec![]
        };
        let endpoint = format!("users/{}.json", self.username);
        self.get(&endpoint, &query).await
    }

    /// Retrieves auth token for the current logged-in session (requires browser session/cookies).
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_auth_token(&self) -> Result<AuthTokenResponse, Error> {
        self.get_no_auth("auth_token.json", &()).await
    }

    /// Retrieves detailed user information with changes since the specified timestamp.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_user_diff(&self, diff_since: OffsetDateTime) -> Result<UserInfoDiff, Error> {
        let diff_since = diff_since.unix_timestamp().to_string();
        let mut query: Vec<(&str, &str)> = vec![("diff_since", &diff_since)];
        if self.emaciated {
            query.push(("emaciated", "true"));
        }
        let endpoint = format!("users/{}.json", self.username);
        self.get(&endpoint, &query).await
    }

    /// Retrieves datapoints for a specific goal (efficient type).
    ///
    /// # Arguments
    /// * `goal` - The goal slug
    /// * `sort` - Attribute to sort on descending. Defaults to "id"
    /// * `count` - Limit results (ignored when page is set)
    /// * `page` - Page number (1-indexed) for pagination
    /// * `per` - Results per page (default 25, requires page)
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_datapoints(
        &self,
        goal: &str,
        sort: Option<&str>,
        count: Option<u64>,
        page: Option<u64>,
        per: Option<u64>,
    ) -> Result<Vec<Datapoint>, Error> {
        self.fetch_datapoints(goal, sort, count, page, per).await
    }

    /// Retrieves datapoints for a specific goal (full type with all fields).
    ///
    /// # Arguments
    /// * `goal` - The goal slug
    /// * `sort` - Attribute to sort on descending. Defaults to "id"
    /// * `count` - Limit results (ignored when page is set)
    /// * `page` - Page number (1-indexed) for pagination
    /// * `per` - Results per page (default 25, requires page)
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_datapoints_full(
        &self,
        goal: &str,
        sort: Option<&str>,
        count: Option<u64>,
        page: Option<u64>,
        per: Option<u64>,
    ) -> Result<Vec<DatapointFull>, Error> {
        self.fetch_datapoints(goal, sort, count, page, per).await
    }

    /// Private helper for fetching datapoints with generic return type
    async fn fetch_datapoints<T: serde::de::DeserializeOwned>(
        &self,
        goal: &str,
        sort: Option<&str>,
        count: Option<u64>,
        page: Option<u64>,
        per: Option<u64>,
    ) -> Result<Vec<T>, Error> {
        let query: Vec<(&str, String)> = vec![
            sort.map(|s| ("sort", s.to_string())),
            count.map(|c| ("count", c.to_string())),
            page.map(|p| ("page", p.to_string())),
            per.map(|p| ("per", p.to_string())),
        ]
        .into_iter()
        .flatten()
        .collect();

        let endpoint = format!("users/{}/goals/{goal}/datapoints.json", self.username);
        self.get(&endpoint, &query).await
    }

    /// Creates a new datapoint for a goal.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn create_datapoint(
        &self,
        goal: &str,
        datapoint: &CreateDatapoint,
    ) -> Result<Datapoint, Error> {
        let endpoint = format!("users/{}/goals/{goal}/datapoints.json", self.username);
        self.post(&endpoint, datapoint).await
    }

    /// Updates an existing datapoint for a goal.
    ///
    /// # Arguments
    /// * `goal` - The slug/name of the goal to update
    /// * `update` - The datapoint update containing the ID and fields to update
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or if the response cannot be parsed.
    pub async fn update_datapoint(
        &self,
        goal: &str,
        update: &UpdateDatapoint,
    ) -> Result<Datapoint, Error> {
        let endpoint = format!(
            "users/{}/goals/{}/datapoints/{}.json",
            self.username, goal, update.id
        );
        self.put(&endpoint, update).await
    }

    /// Deletes a specific datapoint for a goal.
    ///
    /// # Arguments
    /// * `goal` - The name of the goal.
    /// * `datapoint_id` - The ID of the datapoint to delete.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or if the response cannot be parsed.
    pub async fn delete_datapoint(
        &self,
        goal: &str,
        datapoint_id: &str,
    ) -> Result<Datapoint, Error> {
        let endpoint = format!(
            "users/{}/goals/{goal}/datapoints/{datapoint_id}.json",
            self.username
        );
        self.delete(&endpoint, &()).await
    }

    /// Creates multiple datapoints for a goal.
    ///
    /// # Errors
    /// Returns an error if serialization fails or the HTTP request fails.
    pub async fn create_all_datapoints(
        &self,
        goal: &str,
        datapoints: &[CreateDatapoint],
    ) -> Result<CreateAllResponse, Error> {
        let datapoints_json = serde_json::to_string(datapoints)?;
        let query = vec![("datapoints", datapoints_json)];
        let endpoint = format!(
            "users/{}/goals/{goal}/datapoints/create_all.json",
            self.username
        );
        self.post(&endpoint, &query).await
    }

    /// Retrieves all goals for the user.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_goals(&self) -> Result<Vec<GoalSummary>, Error> {
        let query: Vec<(&str, &str)> = if self.emaciated {
            vec![("emaciated", "true")]
        } else {
            vec![]
        };
        let endpoint = format!("users/{}/goals.json", self.username);
        self.get(&endpoint, &query).await
    }

    /// Retrieves archived goals for the user.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_archived_goals(&self) -> Result<Vec<GoalSummary>, Error> {
        let query: Vec<(&str, &str)> = if self.emaciated {
            vec![("emaciated", "true")]
        } else {
            vec![]
        };
        let endpoint = format!("users/{}/goals/archived.json", self.username);
        self.get(&endpoint, &query).await
    }

    /// Retrieves detailed information for a specific goal (efficient type).
    ///
    /// # Arguments
    /// * `goal` - The goal slug
    /// * `datapoints` - Whether to include datapoints in response
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_goal(&self, goal: &str, datapoints: bool) -> Result<Goal, Error> {
        self.fetch_goal(goal, datapoints).await
    }

    /// Retrieves detailed information for a specific goal (full type with all fields).
    ///
    /// # Arguments
    /// * `goal` - The goal slug
    /// * `datapoints` - Whether to include datapoints in response
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_goal_full(&self, goal: &str, datapoints: bool) -> Result<GoalFull, Error> {
        self.fetch_goal(goal, datapoints).await
    }

    /// Private helper for fetching goals with generic return type
    async fn fetch_goal<T: serde::de::DeserializeOwned>(
        &self,
        goal: &str,
        datapoints: bool,
    ) -> Result<T, Error> {
        let mut query: Vec<(&str, &str)> = vec![];
        if datapoints {
            query.push(("datapoints", "true"));
        }
        if self.emaciated {
            query.push(("emaciated", "true"));
        }
        let endpoint = format!("users/{}/goals/{goal}.json", self.username);
        self.get(&endpoint, &query).await
    }

    /// Creates a new goal.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn create_goal(&self, goal: &CreateGoal) -> Result<GoalFull, Error> {
        let endpoint = format!("users/{}/goals.json", self.username);
        self.post(&endpoint, goal).await
    }

    /// Updates an existing goal.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn update_goal(&self, goal: &str, update: &UpdateGoal) -> Result<GoalFull, Error> {
        let endpoint = format!("users/{}/goals/{goal}.json", self.username);
        self.put(&endpoint, update).await
    }

    /// Refreshes a goal's graph (autodata refetch).
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn refresh_graph(&self, goal: &str) -> Result<bool, Error> {
        let endpoint = format!("users/{}/goals/{goal}/refresh_graph.json", self.username);
        self.get(&endpoint, &()).await
    }

    /// Short-circuits a goal (charges current pledge and increases pledge level).
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn shortcircuit(&self, goal: &str) -> Result<GoalFull, Error> {
        let endpoint = format!("users/{}/goals/{goal}/shortcircuit.json", self.username);
        self.post(&endpoint, &()).await
    }

    /// Schedules a pledge stepdown for a goal.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn stepdown(&self, goal: &str) -> Result<GoalFull, Error> {
        let endpoint = format!("users/{}/goals/{goal}/stepdown.json", self.username);
        self.post(&endpoint, &()).await
    }

    /// Cancels a pledge stepdown for a goal.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn cancel_stepdown(&self, goal: &str) -> Result<GoalFull, Error> {
        let endpoint = format!("users/{}/goals/{goal}/cancel_stepdown.json", self.username);
        self.post(&endpoint, &()).await
    }
}
