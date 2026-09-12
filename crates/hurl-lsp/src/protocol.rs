use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::notification::Notification;

pub const RUN_RESULT_METHOD: &str = "hurl/runResult";
pub const RUN_TASK_UPDATE_METHOD: &str = "hurl/runTaskUpdate";
pub const CURL_RESULT_METHOD: &str = "hurl/curlResult";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RunTaskState {
    Queued,
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunTaskUpdate {
    pub task_id: String,
    pub uri: String,
    pub document_version: i32,
    pub entry_line: u32,
    pub target: String,
    pub state: RunTaskState,
    pub started_at: String,
    pub elapsed_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderField {
    pub name: String,
    pub value: String,
    pub sensitive: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BodyContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    pub encoding: String,
    pub original_bytes: usize,
    pub truncated: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestData {
    pub method: String,
    pub url: String,
    pub headers: Vec<HeaderField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<BodyContent>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponseData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    pub headers: Vec<HeaderField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<BodyContent>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpExchange {
    pub request: HttpRequestData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<HttpResponseData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timings: Option<HttpTimings>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpTimings {
    pub dns_ms: u64,
    pub tcp_ms: u64,
    pub tls_ms: u64,
    pub ttfb_ms: u64,
    pub download_ms: u64,
    pub total_ms: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunPhaseTimings {
    pub prepare_ms: u64,
    pub process_ms: u64,
    pub report_ms: u64,
    pub total_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedAssertion {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub uri: String,
    pub document_version: i32,
    pub entry_line: u32,
    pub target: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub profile_sources: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_timings: Option<RunPhaseTimings>,
    pub exchanges: Vec<HttpExchange>,
    pub failed_assertions: Vec<FailedAssertion>,
    pub stdout: String,
    pub stderr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_warning: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurlResult {
    pub uri: String,
    pub document_version: i32,
    pub entry_line: u32,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_command: Option<String>,
    pub unresolved_variables: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub copy_to_clipboard: bool,
}

pub enum RunResultNotification {}
impl Notification for RunResultNotification {
    type Params = RunResult;
    const METHOD: &'static str = RUN_RESULT_METHOD;
}

pub enum RunTaskUpdateNotification {}
impl Notification for RunTaskUpdateNotification {
    type Params = RunTaskUpdate;
    const METHOD: &'static str = RUN_TASK_UPDATE_METHOD;
}

pub enum CurlResultNotification {}
impl Notification for CurlResultNotification {
    type Params = CurlResult;
    const METHOD: &'static str = CURL_RESULT_METHOD;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_wire_names_as_camel_case() {
        let result = CurlResult {
            uri: "file:///tmp/a.hurl".into(),
            document_version: 2,
            entry_line: 4,
            ok: false,
            command: None,
            display_command: None,
            unresolved_variables: vec!["token".into()],
            error: Some("missing".into()),
            copy_to_clipboard: false,
        };
        let value = serde_json::to_value(result).expect("json");
        assert_eq!(value["documentVersion"], 2);
        assert_eq!(value["entryLine"], 4);
        assert_eq!(value["unresolvedVariables"][0], "token");
        assert!(value.get("document_version").is_none());
    }

    #[test]
    fn serializes_run_task_updates_as_camel_case() {
        let update = RunTaskUpdate {
            task_id: "task-1".into(),
            uri: "file:///tmp/a.hurl".into(),
            document_version: 3,
            entry_line: 4,
            target: "entry".into(),
            state: RunTaskState::Running,
            started_at: "2026-09-12T00:00:00Z".into(),
            elapsed_ms: 25,
            profile_name: Some("Local".into()),
            message: None,
            stdout: None,
            stderr: None,
        };
        let value = serde_json::to_value(update).expect("json");
        assert_eq!(value["taskId"], "task-1");
        assert_eq!(value["documentVersion"], 3);
        assert_eq!(value["state"], "running");
        assert_eq!(value["profileName"], "Local");
    }
}
