pub mod types;
use crate::types::{CreateDatapoint, Datapoint, UserInfo, UserInfoDiff};
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
    async fn request<T>(&self, endpoint: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let response = self
            .client
            .get(format!("{}{}", self.base_url, endpoint))
            .query(&[("auth_token", &self.api_key)])
            .send()
            .await?
            .error_for_status()?;

        response.json().await.map_err(Error::from)
    }

    async fn post<T>(&self, endpoint: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, endpoint))
            .query(&[("auth_token", &self.api_key)])
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
        self.request(&format!("users/{username}.json")).await
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
        let timestamp = diff_since.unix_timestamp();
        self.request(&format!("users/{username}.json?diff_since={timestamp}"))
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
        let mut endpoint = format!("users/{username}/goals/{goal}/datapoints.json");

        let mut query = Vec::new();

        if let Some(sort) = sort {
            query.push(format!("sort={sort}"));
        } else {
            query.push("sort=timestamp".to_string());
        }

        if let Some(count) = count {
            query.push(format!("count={count}"));
        }

        if !query.is_empty() {
            endpoint = format!("{}?{}", endpoint, query.join("&"));
        }

        self.request(&endpoint).await
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
        let mut query = Vec::new();
        query.push(format!("value={}", datapoint.value));
        if let Some(ts) = datapoint.timestamp {
            query.push(format!("timestamp={}", ts.unix_timestamp()));
        }
        if let Some(ds) = &datapoint.daystamp {
            query.push(format!("daystamp={ds}"));
        }
        if let Some(c) = &datapoint.comment {
            query.push(format!("comment={c}"));
        }
        if let Some(rid) = &datapoint.requestid {
            query.push(format!("requestid={rid}"));
        }

        let mut endpoint = format!("users/{username}/goals/{goal}/datapoints.json");
        endpoint = format!("{}?{}", endpoint, query.join("&"));
        self.post(&endpoint).await
    }
}
