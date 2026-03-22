use crate::syntax::{method_from_line, section_name_from_line};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

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

fn parse_failed_assert(detail: &str) -> Option<&str> {
    detail
        .split_once("assert failed:")
        .map(|(_, suffix)| suffix.trim())
        .filter(|value| !value.is_empty())
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
}
