use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::notification::Notification;

pub const RUN_RESULT_METHOD: &str = "hurl/runResult";
pub const CURL_RESULT_METHOD: &str = "hurl/curlResult";

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
}

pub enum RunResultNotification {}
impl Notification for RunResultNotification {
    type Params = RunResult;
    const METHOD: &'static str = RUN_RESULT_METHOD;
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
        };
        let value = serde_json::to_value(result).expect("json");
        assert_eq!(value["documentVersion"], 2);
        assert_eq!(value["entryLine"], 4);
        assert_eq!(value["unresolvedVariables"][0], "token");
        assert!(value.get("document_version").is_none());
    }
}
