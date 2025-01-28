pub mod types;
use crate::types::{
    CreateDatapoint, Datapoint, GoalSummary, UpdateDatapoint, UserInfo, UserInfoDiff,
};
use reqwest::Client;
use time::OffsetDateTime;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

pub struct BeeminderClient {
    client: Client,
    api_key: String,
    base_url: String,
    username: String,
}

impl BeeminderClient {
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
            .await?
            .error_for_status()?;
        response.json().await.map_err(Error::from)
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
            .query(query)
            .send()
            .await?
            .error_for_status()?;
        response.json().await.map_err(Error::from)
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
        }
    }

    /// Sets a username for this client.
    #[must_use]
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = username.into();
        self
    }

    /// Retrieves user information for user associated with client.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_user(&self) -> Result<UserInfo, Error> {
        let endpoint = format!("users/{}.json", self.username);
        self.get(&endpoint, &()).await
    }

    /// Retrieves detailed user information with changes since the specified timestamp.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_user_diff(&self, diff_since: OffsetDateTime) -> Result<UserInfoDiff, Error> {
        let diff_since = diff_since.unix_timestamp().to_string();
        let query = [("diff_since", &diff_since)];
        let endpoint = format!("users/{}.json", self.username);
        self.get(&endpoint, &query).await
    }

    /// Retrieves datapoints for a specific goal.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_datapoints(
        &self,
        goal: &str,
        sort: Option<&str>,
        count: Option<u64>,
    ) -> Result<Vec<Datapoint>, Error> {
        let query: Vec<(&str, String)> = vec![
            Some(("sort", sort.unwrap_or("timestamp").to_string())),
            count.map(|c| ("count", c.to_string())),
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
    /// Returns an error if the HTTP request fails or if the response cannot be parsed
    pub async fn update_datapoint(
        &self,
        goal: &str,
        update: &UpdateDatapoint,
    ) -> Result<Datapoint, Error> {
        let endpoint = format!(
            "users/{}/goals/{}/datapoints/{}.json",
            self.username, goal, update.id
        );

        let response = self
            .client
            .put(format!("{}{}", self.base_url, endpoint))
            .query(&[("auth_token", self.api_key.as_str())])
            .query(update)
            .send()
            .await?
            .error_for_status()?;

        response.json().await.map_err(Error::from)
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
        let query = vec![("auth_token", self.api_key.as_str())];

        let response = self
            .client
            .delete(format!("{}{}", self.base_url, endpoint))
            .query(&query)
            .send()
            .await?
            .error_for_status()?;

        response.json().await.map_err(Error::from)
    }

    /// Retrieves all goals for the user.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_goals(&self) -> Result<Vec<GoalSummary>, Error> {
        let endpoint = format!("users/{}/goals.json", self.username);
        self.get(&endpoint, &()).await
    }

    /// Retrieves archived goals for the user.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_archived_goals(&self) -> Result<Vec<GoalSummary>, Error> {
        let endpoint = format!("users/{}/goals/archived.json", self.username);
        self.get(&endpoint, &()).await
    }
}
