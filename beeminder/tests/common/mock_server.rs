use beeminder::BeeminderClient;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use wiremock::matchers::{method, path_regex};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

#[derive(Deserialize)]
pub struct Fixture {
    #[serde(rename = "_meta")]
    pub meta: Option<FixtureMeta>,
    pub request: FixtureRequest,
    pub response: FixtureResponse,
}

#[derive(Deserialize)]
pub struct FixtureMeta {
    pub query: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Deserialize)]
pub struct FixtureRequest {
    pub method: String,
    pub path_pattern: String,
}

#[derive(Deserialize)]
pub struct FixtureResponse {
    pub status_code: u16,
    pub body: serde_json::Value,
}

pub struct BeeminderMock {
    pub server: MockServer,
}

impl BeeminderMock {
    pub async fn start() -> Self {
        Self {
            server: MockServer::start().await,
        }
    }

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    #[allow(dead_code)]
    pub async fn mount_fixture(&self, fixture_path: &str) {
        self.mount_fixture_in("min", fixture_path).await;
    }

    pub async fn mount_fixture_in(&self, fixture_set: &str, fixture_path: &str) {
        let full_path = Self::fixtures_dir().join(fixture_set).join(fixture_path);

        let content = fs::read_to_string(&full_path)
            .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", full_path.display(), e));

        let fixture: Fixture = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse fixture {}: {}", full_path.display(), e));

        let mut mock = Mock::given(method(fixture.request.method.as_str()))
            .and(path_regex(&fixture.request.path_pattern));

        if let Some(meta) = &fixture.meta {
            if let Some(query) = &meta.query {
                for (key, value) in query {
                    if key == "auth_token" {
                        continue;
                    }
                    if let Some(value) = query_value_to_string(value) {
                        mock = mock.and(query_param_normalized(key, value));
                    }
                }
            }
        }

        mock.respond_with(
            ResponseTemplate::new(fixture.response.status_code)
                .set_body_json(&fixture.response.body),
        )
        .mount(&self.server)
        .await;
    }

    pub fn client(&self) -> BeeminderClient {
        BeeminderClient::new("test_token".into())
            .with_base_url(format!("{}/api/v1/", self.server.uri()))
    }
}

fn query_value_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        serde_json::Value::Bool(value) => Some(value.to_string()),
        serde_json::Value::Null => None,
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => None,
    }
}

struct QueryParamNormalizedMatcher {
    key: String,
    expected: String,
}

fn query_param_normalized(
    key: impl Into<String>,
    expected: impl Into<String>,
) -> QueryParamNormalizedMatcher {
    QueryParamNormalizedMatcher {
        key: key.into(),
        expected: expected.into(),
    }
}

impl Match for QueryParamNormalizedMatcher {
    fn matches(&self, request: &Request) -> bool {
        if request.url.query_pairs().any(|pair| {
            if pair.0 != self.key.as_str() {
                return false;
            }
            let actual = pair.1.as_ref();
            values_match(&self.expected, actual)
        }) {
            return true;
        }

        if let Some(actual) = form_body_value(request, &self.key) {
            return values_match(&self.expected, &actual);
        }

        false
    }
}

fn values_match(expected: &str, actual: &str) -> bool {
    if expected == actual {
        return true;
    }

    let expected_num = expected.parse::<f64>();
    let actual_num = actual.parse::<f64>();
    if let (Ok(expected), Ok(actual)) = (expected_num, actual_num) {
        return (expected - actual).abs() < f64::EPSILON;
    }

    if looks_like_json(expected) && looks_like_json(actual) {
        if let (Ok(expected), Ok(actual)) = (
            serde_json::from_str::<serde_json::Value>(expected),
            serde_json::from_str::<serde_json::Value>(actual),
        ) {
            return expected == actual;
        }
    }

    false
}

fn looks_like_json(value: &str) -> bool {
    value
        .trim_start()
        .chars()
        .next()
        .map(|first| first == '{' || first == '[')
        .unwrap_or(false)
}

fn form_body_value(request: &Request, key: &str) -> Option<String> {
    let body = std::str::from_utf8(&request.body).ok()?;
    let mut found = None;
    for (k, v) in url::form_urlencoded::parse(body.as_bytes()) {
        if k == key {
            found = Some(v.into_owned());
            break;
        }
    }
    found
}
