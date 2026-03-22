use crate::syntax::{method_from_line, section_name_from_line};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunSummary {
    pub success: bool,
    pub failed_asserts: usize,
    pub duration_ms: Option<u64>,
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
    RunSummary {
        success,
        failed_asserts: parse_failed_assert_count(stderr),
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
}
