use crate::syntax::{method_from_line, section_name_from_line};
use crate::{
    protocol::{
        BodyContent, FailedAssertion, HeaderField, HttpExchange, HttpRequestData, HttpResponseData,
        HttpTimings, RunResult,
    },
    variables::is_sensitive_name,
};
use serde_json::Value;
use std::{fs, path::Path};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use url::Url;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunSummary {
    pub success: bool,
    pub failed_asserts: usize,
    pub duration_ms: Option<u64>,
}

const BODY_LIMIT: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug)]
pub enum RunTarget {
    Entry,
    Chain,
    File,
}

impl RunTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::Chain => "chain",
            Self::File => "file",
        }
    }
}

pub struct RunResultContext<'a> {
    pub uri: &'a Url,
    pub document_version: i32,
    pub entry_line: u32,
    pub target: RunTarget,
    pub success: bool,
    pub exit_code: Option<i32>,
}

pub fn parse_hurl_report_result(
    context: RunResultContext<'_>,
    report_root: &Path,
    stdout: &[u8],
    stderr: &[u8],
) -> RunResult {
    let stdout_text = String::from_utf8_lossy(stdout).into_owned();
    let stderr_text = String::from_utf8_lossy(stderr).into_owned();
    let report_path = report_root.join("report.json");
    let parsed = fs::read(&report_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    let mut result = RunResult {
        uri: context.uri.to_string(),
        document_version: context.document_version,
        entry_line: context.entry_line,
        target: context.target.as_str().into(),
        success: context.success,
        exit_code: context.exit_code,
        started_at: String::new(),
        duration_ms: None,
        exchanges: Vec::new(),
        failed_assertions: Vec::new(),
        stdout: stdout_text,
        stderr: stderr_text,
        parse_warning: None,
    };
    let Some(root) = parsed else {
        result.parse_warning =
            Some("Hurl JSON report was unavailable or invalid; showing raw output.".into());
        return result;
    };
    let Some(file) = root
        .as_array()
        .and_then(|items| items.first())
        .or_else(|| root.as_object().map(|_| &root))
    else {
        result.parse_warning = Some("Hurl JSON report did not contain a file result.".into());
        return result;
    };
    result.duration_ms = file.get("time").and_then(Value::as_u64);
    for entry in file
        .get("entries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for assertion in entry
            .get("asserts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if assertion.get("success").and_then(Value::as_bool) == Some(false) {
                result.failed_assertions.push(FailedAssertion {
                    message: assertion
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("Assertion failed")
                        .into(),
                    line: assertion
                        .get("line")
                        .and_then(Value::as_u64)
                        .map(|line| line.saturating_sub(1) as u32),
                });
            }
        }
        for call in entry
            .get("calls")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let request = call.get("request").unwrap_or(&Value::Null);
            let response = call.get("response");
            if result.started_at.is_empty() {
                result.started_at = call
                    .get("timings")
                    .and_then(|v| v.get("begin_call"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .into();
            }
            let duration_ms = call
                .get("timings")
                .and_then(|v| v.get("total"))
                .and_then(Value::as_u64)
                .map(|micros| micros / 1000);
            let timings = call.get("timings").and_then(parse_http_timings);
            result.exchanges.push(HttpExchange {
                request: HttpRequestData {
                    method: request
                        .get("method")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .into(),
                    url: request
                        .get("url")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .into(),
                    headers: parse_headers(request.get("headers")),
                    body: read_body(request.get("body"), request.get("headers"), report_root)
                        .or_else(|| {
                            read_request_body_from_curl(
                                entry.get("curl_cmd"),
                                request.get("headers"),
                            )
                        }),
                },
                response: response.map(|response| HttpResponseData {
                    version: response
                        .get("http_version")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    status: response
                        .get("status")
                        .and_then(Value::as_u64)
                        .map(|v| v as u16),
                    headers: parse_headers(response.get("headers")),
                    body: read_body(response.get("body"), response.get("headers"), report_root),
                }),
                duration_ms,
                timings,
            });
        }
    }
    if result.started_at.is_empty() {
        result.started_at = "unknown".into();
    }
    result
}

fn parse_http_timings(value: &Value) -> Option<HttpTimings> {
    let total = value.get("total")?.as_u64()?;
    let name_lookup = value
        .get("name_lookup")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let connect = value
        .get("connect")
        .and_then(Value::as_u64)
        .unwrap_or(name_lookup);
    let app_connect = value
        .get("app_connect")
        .and_then(Value::as_u64)
        .unwrap_or(connect);
    let start_transfer = value
        .get("start_transfer")
        .and_then(Value::as_u64)
        .unwrap_or(app_connect);
    Some(HttpTimings {
        dns_ms: name_lookup / 1000,
        tcp_ms: connect.saturating_sub(name_lookup) / 1000,
        tls_ms: app_connect.saturating_sub(connect) / 1000,
        ttfb_ms: start_transfer.saturating_sub(app_connect.max(connect)) / 1000,
        download_ms: total.saturating_sub(start_transfer) / 1000,
        total_ms: total / 1000,
    })
}

fn parse_headers(value: Option<&Value>) -> Vec<HeaderField> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|header| {
            let name = header
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            HeaderField {
                sensitive: is_sensitive_name(&name)
                    || matches!(name.to_ascii_lowercase().as_str(), "cookie" | "set-cookie"),
                name,
                value: header
                    .get("value")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            }
        })
        .collect()
}

fn read_body(
    reference: Option<&Value>,
    headers: Option<&Value>,
    report_root: &Path,
) -> Option<BodyContent> {
    let relative = reference.and_then(Value::as_str)?;
    let bytes = fs::read(report_root.join(relative)).ok()?;
    let original_bytes = bytes.len();
    let shown = &bytes[..bytes.len().min(BODY_LIMIT)];
    let media_type = parse_headers(headers)
        .into_iter()
        .find(|h| h.name.eq_ignore_ascii_case("content-type"))
        .map(|h| h.value);
    match std::str::from_utf8(shown) {
        Ok(text) => Some(BodyContent {
            text: Some(text.to_string()),
            media_type,
            encoding: "utf8".into(),
            original_bytes,
            truncated: original_bytes > BODY_LIMIT,
        }),
        Err(_) => Some(BodyContent {
            text: None,
            media_type,
            encoding: "binary".into(),
            original_bytes,
            truncated: original_bytes > BODY_LIMIT,
        }),
    }
}

fn read_request_body_from_curl(
    curl_command: Option<&Value>,
    headers: Option<&Value>,
) -> Option<BodyContent> {
    let command = curl_command.and_then(Value::as_str)?;
    let argument = ["--data-raw ", "--data-binary ", "--data "]
        .iter()
        .find_map(|flag| {
            command
                .find(flag)
                .map(|index| &command[index + flag.len()..])
        })?;
    let text = parse_shell_argument(argument.trim_start())?;
    if text.starts_with('@') {
        return None;
    }
    let original_bytes = text.len();
    let mut shown_bytes = original_bytes.min(BODY_LIMIT);
    while !text.is_char_boundary(shown_bytes) {
        shown_bytes -= 1;
    }
    let truncated = shown_bytes < original_bytes;
    let text = text[..shown_bytes].to_string();
    let media_type = parse_headers(headers)
        .into_iter()
        .find(|header| header.name.eq_ignore_ascii_case("content-type"))
        .map(|header| header.value);
    Some(BodyContent {
        text: Some(text),
        media_type,
        encoding: "utf8".into(),
        original_bytes,
        truncated,
    })
}

fn parse_shell_argument(input: &str) -> Option<String> {
    if let Some(rest) = input.strip_prefix("$'") {
        return decode_ansi_c_string(rest);
    }
    if let Some(rest) = input.strip_prefix('\'') {
        return rest.find('\'').map(|end| rest[..end].to_string());
    }
    let end = input.find(char::is_whitespace).unwrap_or(input.len());
    (!input[..end].is_empty()).then(|| input[..end].to_string())
}

fn decode_ansi_c_string(input: &str) -> Option<String> {
    let mut output = String::new();
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\'' => return Some(output),
            '\\' => {
                let escaped = chars.next()?;
                output.push(match escaped {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'b' => '\u{0008}',
                    'f' => '\u{000c}',
                    other => other,
                });
            }
            other => output.push(other),
        }
    }
    None
}

pub fn execution_diagnostics_for_result(line: u32, success: bool, detail: &str) -> Vec<Diagnostic> {
    if success {
        return Vec::new();
    }
    vec![Diagnostic {
        range: Range::new(Position::new(line, 0), Position::new(line, 1)),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("hurl-lsp-run".to_string()),
        message: format!("Run failed: {detail}"),
        ..Default::default()
    }]
}

pub fn execution_diagnostics_for_entry_failure(
    source: &str,
    entry_line: u32,
    detail: &str,
) -> Vec<Diagnostic> {
    let failed_assert = parse_failed_assert(detail);
    let mut diagnostics = Vec::new();
    let mut in_entry = false;
    let mut in_asserts = false;

    for (idx, raw_line) in source.lines().enumerate() {
        let line_no = idx as u32;
        if line_no == entry_line {
            in_entry = true;
            continue;
        }
        if !in_entry {
            continue;
        }

        let trimmed = raw_line.trim();
        if method_from_line(trimmed).is_some() {
            break;
        }
        if let Some(section) = section_name_from_line(trimmed) {
            in_asserts = section == "Asserts";
            continue;
        }
        if !in_asserts || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(expected) = failed_assert {
            if !trimmed.contains(expected) {
                continue;
            }
        }

        diagnostics.push(failure_diag(line_no, detail));
    }

    if diagnostics.is_empty() {
        return execution_diagnostics_for_result(entry_line, false, detail);
    }
    diagnostics
}

pub fn parse_run_summary(stderr: &str, stdout: &str, success: bool) -> RunSummary {
    let failed_asserts = parse_failed_assert_count(stderr).max(parse_failed_assert_count(stdout));
    RunSummary {
        success,
        failed_asserts,
        duration_ms: parse_duration_ms(stderr).or_else(|| parse_duration_ms(stdout)),
    }
}

fn parse_failed_assert(detail: &str) -> Option<&str> {
    const MARKER: &[u8] = b"assert failed:";
    for (idx, _) in detail.char_indices() {
        let end = idx + MARKER.len();
        if end > detail.len() {
            break;
        }
        if detail.as_bytes()[idx..end].eq_ignore_ascii_case(MARKER) {
            let suffix = detail.get(end..)?.trim();
            if !suffix.is_empty() {
                return Some(suffix);
            }
            return None;
        }
    }
    None
}

fn parse_failed_assert_count(detail: &str) -> usize {
    let lower = detail.to_ascii_lowercase();
    let marker = "assert failed";
    let Some(pos) = lower.find(marker) else {
        return 0;
    };
    let prefix = &lower[..pos];
    let digits_rev: String = prefix
        .chars()
        .rev()
        .skip_while(|ch| ch.is_whitespace())
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    if digits_rev.is_empty() {
        1
    } else {
        digits_rev
            .chars()
            .rev()
            .collect::<String>()
            .parse::<usize>()
            .unwrap_or(1)
    }
}

fn parse_duration_ms(detail: &str) -> Option<u64> {
    let lower = detail.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    for idx in 1..bytes.len() {
        if bytes[idx - 1] == b'm' && bytes[idx] == b's' {
            let mut begin = idx.saturating_sub(2);
            while begin > 0 && bytes[begin].is_ascii_whitespace() {
                begin -= 1;
            }
            while begin > 0 && bytes[begin - 1].is_ascii_digit() {
                begin -= 1;
            }
            let candidate = lower[begin..idx.saturating_sub(1)].trim();
            if let Ok(value) = candidate.parse::<u64>() {
                return Some(value);
            }
        }
    }
    None
}

fn failure_diag(line: u32, detail: &str) -> Diagnostic {
    Diagnostic {
        range: Range::new(Position::new(line, 0), Position::new(line, 1)),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("hurl-lsp-run".to_string()),
        message: format!("Run failed: {detail}"),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_error_diagnostic_for_failed_run() {
        let diagnostics =
            execution_diagnostics_for_result(12, false, "assert failed: status == 200");
        assert_eq!(diagnostics.len(), 1);
        let diag = &diagnostics[0];
        assert_eq!(diag.range.start.line, 12);
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
        assert!(diag.message.contains("assert failed"));
    }

    #[test]
    fn maps_failed_run_to_assert_lines_when_present() {
        let source = "POST /users\nHTTP 201\n[Asserts]\nstatus == 201\njsonpath \"$.id\" exists\n";
        let diagnostics =
            execution_diagnostics_for_entry_failure(source, 0, "assert failed: status == 201");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].range.start.line, 3);
    }

    #[test]
    fn matches_failed_assert_marker_case_insensitively() {
        let source = "POST /users\nHTTP 201\n[Asserts]\nstatus == 201\n";
        let diagnostics =
            execution_diagnostics_for_entry_failure(source, 0, "Assert Failed: status == 201");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].range.start.line, 3);
    }

    #[test]
    fn parses_run_summary_with_failed_asserts_and_duration() {
        let summary = parse_run_summary("2 assert failed · 230ms", "", false);
        assert!(!summary.success);
        assert_eq!(summary.failed_asserts, 2);
        assert_eq!(summary.duration_ms, Some(230));
    }

    #[test]
    fn parses_failed_assert_count_from_stdout_when_stderr_empty() {
        let summary = parse_run_summary("", "1 assert failed · 120ms", false);
        assert!(!summary.success);
        assert_eq!(summary.failed_asserts, 1);
        assert_eq!(summary.duration_ms, Some(120));
    }

    #[test]
    fn parses_report_metadata_and_body() {
        let root = std::env::temp_dir().join(format!("hurl-lsp-report-{}", std::process::id()));
        fs::create_dir_all(root.join("store")).expect("mkdir");
        fs::write(root.join("store/body.json"), "{\"ok\":true}").expect("body");
        fs::write(root.join("report.json"), r#"[{"success":true,"time":15,"entries":[{"asserts":[],"curl_cmd":"curl --data $'{\"name\":\"O\\'Brien\"}\\n' 'https://example.com'","calls":[{"request":{"method":"POST","url":"https://example.com","headers":[{"name":"Authorization","value":"Bearer token"},{"name":"Content-Type","value":"application/json"}]},"response":{"http_version":"HTTP/2","status":200,"headers":[{"name":"Content-Type","value":"application/json"}],"body":"store/body.json"},"timings":{"begin_call":"2026-09-05T00:00:00Z","name_lookup":1000,"connect":3000,"app_connect":8000,"start_transfer":12000,"total":15000}}]}]}]"#).expect("report");
        let uri = Url::parse("file:///tmp/a.hurl").expect("uri");
        let result = parse_hurl_report_result(
            RunResultContext {
                uri: &uri,
                document_version: 2,
                entry_line: 0,
                target: RunTarget::Entry,
                success: true,
                exit_code: Some(0),
            },
            &root,
            b"",
            b"",
        );
        assert_eq!(result.duration_ms, Some(15));
        let timings = result.exchanges[0].timings.as_ref().expect("timings");
        assert_eq!((timings.dns_ms, timings.tcp_ms, timings.tls_ms), (1, 2, 5));
        assert_eq!(
            (timings.ttfb_ms, timings.download_ms, timings.total_ms),
            (4, 3, 15)
        );
        assert_eq!(
            result.exchanges[0].response.as_ref().and_then(|r| r.status),
            Some(200)
        );
        assert_eq!(
            result.exchanges[0]
                .response
                .as_ref()
                .and_then(|r| r.body.as_ref())
                .and_then(|b| b.text.as_deref()),
            Some("{\"ok\":true}")
        );
        assert!(result.exchanges[0].request.headers[0].sensitive);
        assert_eq!(
            result.exchanges[0]
                .request
                .body
                .as_ref()
                .and_then(|body| body.text.as_deref()),
            Some("{\"name\":\"O'Brien\"}\n")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn falls_back_to_raw_output_for_invalid_report() {
        let uri = Url::parse("file:///tmp/a.hurl").expect("uri");
        let result = parse_hurl_report_result(
            RunResultContext {
                uri: &uri,
                document_version: 1,
                entry_line: 0,
                target: RunTarget::Entry,
                success: false,
                exit_code: Some(2),
            },
            Path::new("/missing"),
            b"raw",
            b"error",
        );
        assert!(result.parse_warning.is_some());
        assert_eq!(result.stdout, "raw");
    }
}
