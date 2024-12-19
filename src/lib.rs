pub mod types;
use crate::types::{CreateDatapoint, Datapoint, GoalSummary, UserInfo, UserInfoDiff};
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
}

impl BeeminderClient {
    async fn request<T>(
        &self,
        endpoint: &str,
        params: Option<Vec<(&str, &str)>>,
    ) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut query = vec![("auth_token", self.api_key.as_str())];
        if let Some(additional_params) = params {
            query.extend(additional_params);
        }

        let response = self
            .client
            .get(format!("{}{}", self.base_url, endpoint))
            .query(&query)
            .send()
            .await?
            .error_for_status()?;
        response.json().await.map_err(Error::from)
    }

    async fn post<T>(&self, endpoint: &str, params: Option<Vec<(&str, &str)>>) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut query = vec![("auth_token", self.api_key.as_str())];
        if let Some(additional_params) = params {
            query.extend(additional_params);
        }

        let response = self
            .client
            .post(format!("{}{}", self.base_url, endpoint))
            .query(&query)
            .send()
            .await?
            .error_for_status()?;

        response.json().await.map_err(Error::from)
    }

    /// Creates a new `BeeminderClient` with the given API key.
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://www.beeminder.com/api/v1/".to_string(),
        }
    }

    /// Retrieves information about a user.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_user(&self, username: &str) -> Result<UserInfo, Error> {
        self.request(&format!("users/{username}.json"), None).await
    }

    /// Retrieves detailed user information with changes since the specified timestamp.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_user_diff(
        &self,
        username: &str,
        diff_since: OffsetDateTime,
    ) -> Result<UserInfoDiff, Error> {
        let diff_since = diff_since.unix_timestamp().to_string();
        let params = vec![("diff_since", diff_since.as_str())];
        self.request(&format!("users/{username}.json"), Some(params))
            .await
    }

    /// Retrieves datapoints for a specific goal.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_datapoints(
        &self,
        username: &str,
        goal: &str,
        sort: Option<&str>,
        count: Option<u64>,
    ) -> Result<Vec<Datapoint>, Error> {
        let mut params = Vec::new();
        params.push(("sort", sort.unwrap_or("timestamp")));

        let count_str;
        if let Some(count) = count {
            count_str = count.to_string();
            params.push(("count", &count_str));
        }

        let endpoint = format!("users/{username}/goals/{goal}/datapoints.json");
        self.request(&endpoint, Some(params)).await
    }

    /// Creates a new datapoint for a goal.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn create_datapoint(
        &self,
        username: &str,
        goal: &str,
        datapoint: &CreateDatapoint,
    ) -> Result<Datapoint, Error> {
        let mut params = Vec::new();

        let value_str = datapoint.value.to_string();
        params.push(("value", value_str.as_str()));

        let timestamp_str;
        if let Some(ts) = datapoint.timestamp {
            timestamp_str = ts.unix_timestamp().to_string();
            params.push(("timestamp", timestamp_str.as_str()));
        }

        if let Some(ds) = &datapoint.daystamp {
            params.push(("daystamp", ds.as_str()));
        }

        if let Some(c) = &datapoint.comment {
            params.push(("comment", c.as_str()));
        }

        if let Some(rid) = &datapoint.requestid {
            params.push(("requestid", rid.as_str()));
        }

        let endpoint = format!("users/{username}/goals/{goal}/datapoints.json");
        self.post(&endpoint, Some(params)).await
    }

    /// Deletes a specific datapoint for a user's goal.
    ///
    /// # Arguments
    /// * `username` - The username of the user.
    /// * `goal` - The name of the goal.
    /// * `datapoint_id` - The ID of the datapoint to delete.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or if the response cannot be parsed.
    pub async fn delete_datapoint(
        &self,
        username: &str,
        goal: &str,
        datapoint_id: &str,
    ) -> Result<Datapoint, Error> {
        let endpoint = format!("users/{username}/goals/{goal}/datapoints/{datapoint_id}.json");
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

    /// Retrieves all goals for a user.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_goals(&self, username: &str) -> Result<Vec<GoalSummary>, Error> {
        self.request(&format!("users/{username}/goals.json"), None)
            .await
    }

    /// Retrieves archived goals for a user.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or response cannot be parsed.
    pub async fn get_archived_goals(&self, username: &str) -> Result<Vec<GoalSummary>, Error> {
        self.request(&format!("users/{username}/goals/archived.json"), None)
            .await
    }
}
