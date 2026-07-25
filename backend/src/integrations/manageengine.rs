use chrono::{DateTime, TimeZone, Utc};
use regex::Regex;
use serde::Deserialize;

use crate::config::ManageEngineConfig;

/// Kết quả parse từ ManageEngine API.
#[derive(Debug, Clone)]
pub struct ChangeDetail {
    pub id: String,
    pub title: Option<String>,
    pub status: Option<String>,
    pub scheduled_start_time: Option<DateTime<Utc>>,
    pub scheduled_end_time: Option<DateTime<Utc>>,
}

/// Client gọi ManageEngine ServiceDesk Plus API.
#[derive(Clone)]
pub struct ManageEngineClient {
    http: reqwest::Client,
    config: ManageEngineConfig,
}

impl ManageEngineClient {
    pub fn new(config: &ManageEngineConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            config: config.clone(),
        }
    }

    /// Fetch change details từ ManageEngine API.
    ///
    /// Endpoint: GET {base_url}/api/v3/changes/{change_id}
    /// Auth: Header `authtoken: {token}`
    pub async fn get_change(&self, change_id: &str) -> Result<ChangeDetail, ManageEngineError> {
        if !self.config.enabled {
            return Err(ManageEngineError::Disabled);
        }

        let url = format!("{}/api/v3/changes/{}", self.config.base_url, change_id);

        let resp = self
            .http
            .get(&url)
            .header("Accept", "application/vnd.manageengine.sdp.v3+json")
            .header("authtoken", &self.config.auth_token)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| ManageEngineError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(ManageEngineError::ApiError(format!(
                "HTTP {} from ManageEngine",
                resp.status()
            )));
        }

        let body: ApiResponse = resp
            .json()
            .await
            .map_err(|e| ManageEngineError::ParseError(e.to_string()))?;

        if body.response_status.status_code != 2000 {
            return Err(ManageEngineError::ApiError(format!(
                "ManageEngine error: {}",
                body.response_status.status
            )));
        }

        let change = body.change;
        Ok(ChangeDetail {
            id: change.id.to_string(),
            title: change.title,
            status: change.status.map(|s| s.name).flatten(),
            scheduled_start_time: change
                .scheduled_start_time
                .and_then(|t| millis_to_datetime(t.value)),
            scheduled_end_time: change
                .scheduled_end_time
                .and_then(|t| millis_to_datetime(t.value)),
        })
    }
}

/// Extract change_id từ ManageEngine URL.
///
/// Hỗ trợ patterns:
/// - `.../changes/{id}/...`
/// - `.../changes/{id}`
/// - `...WorkOrder.do?...CHANGEID={id}`
pub fn parse_cr_url(url: &str) -> Result<String, ManageEngineError> {
    // Pattern 1: REST-style URL — /changes/{id}
    let re = Regex::new(r"/changes/(\d+)").unwrap();
    if let Some(cap) = re.captures(url) {
        return Ok(cap[1].to_string());
    }

    // Pattern 2: Legacy URL with query param CHANGEID
    let re2 = Regex::new(r"CHANGEID=(\d+)").unwrap();
    if let Some(cap) = re2.captures(url) {
        return Ok(cap[1].to_string());
    }

    // Pattern 3: Generic numeric ID in path (e.g. /ui/requests/12345/details)
    let re3 = Regex::new(r"/(\d+)(?:/details)?/?$").unwrap();
    if let Some(cap) = re3.captures(url) {
        return Ok(cap[1].to_string());
    }

    Err(ManageEngineError::InvalidUrl(
        "Cannot extract Change ID from URL. Expected format: .../changes/{id}/...".into(),
    ))
}

// ── Error type ─────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ManageEngineError {
    #[error("ManageEngine integration is disabled")]
    Disabled,

    #[error("invalid ManageEngine URL: {0}")]
    InvalidUrl(String),

    #[error("ManageEngine API error: {0}")]
    ApiError(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("response parse error: {0}")]
    ParseError(String),
}

// ── Internal API response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ApiResponse {
    response_status: ResponseStatus,
    change: ChangeData,
}

#[derive(Debug, Deserialize)]
struct ResponseStatus {
    status_code: i32,
    status: String,
}

#[derive(Debug, Deserialize)]
struct ChangeData {
    id: serde_json::Value, // ManageEngine returns id as string or number
    title: Option<String>,
    status: Option<StatusData>,
    scheduled_start_time: Option<DateTimeData>,
    scheduled_end_time: Option<DateTimeData>,
}

#[derive(Debug, Deserialize)]
struct StatusData {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DateTimeData {
    value: Option<String>, // milliseconds as string
}

/// Convert milliseconds string to DateTime<Utc>.
fn millis_to_datetime(value: Option<String>) -> Option<DateTime<Utc>> {
    value
        .and_then(|v| v.parse::<i64>().ok())
        .and_then(|ms| Utc.timestamp_millis_opt(ms).single())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cr_url_changes_path() {
        let url = "https://servicedesk.company.com/app/itdesk/ui/changes/12345/details";
        assert_eq!(parse_cr_url(url).unwrap(), "12345");
    }

    #[test]
    fn test_parse_cr_url_changes_no_trailing() {
        let url = "https://servicedesk.company.com/api/v3/changes/999";
        assert_eq!(parse_cr_url(url).unwrap(), "999");
    }

    #[test]
    fn test_parse_cr_url_legacy_query_param() {
        let url = "https://servicedesk.company.com/WorkOrder.do?module=Change&CHANGEID=54321";
        assert_eq!(parse_cr_url(url).unwrap(), "54321");
    }

    #[test]
    fn test_parse_cr_url_generic_numeric_id() {
        let url = "https://servicedesk.company.com/app/itdesk/ui/requests/67890/details";
        assert_eq!(parse_cr_url(url).unwrap(), "67890");
    }

    #[test]
    fn test_parse_cr_url_invalid() {
        let url = "https://example.com/no-id-here";
        assert!(parse_cr_url(url).is_err());
    }

    #[test]
    fn test_millis_to_datetime() {
        // 2017-02-16T00:00:00Z in millis
        let dt = millis_to_datetime(Some("1487183400000".to_string()));
        assert!(dt.is_some());
    }

    #[test]
    fn test_millis_to_datetime_none() {
        assert!(millis_to_datetime(None).is_none());
        assert!(millis_to_datetime(Some("not_a_number".to_string())).is_none());
    }
}
