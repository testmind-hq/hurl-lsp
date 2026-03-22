use crate::syntax::{
    canonical_section_name, is_http_method, is_identifier, is_known_section, section_label,
    section_name_from_line, variable_placeholders, visible_variables_before_line,
};
use hurl_core::error::DisplaySourceError;
use std::collections::BTreeSet;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

#[derive(Clone, Debug, Default)]
pub struct Entry {
    pub method: String,
    pub path: String,
    pub line: u32,
}

#[derive(Clone, Debug, Default)]
pub struct ParsedDocument {
    pub entries: Vec<Entry>,
}

pub fn parse_document(text: &str) -> ParsedDocument {
    let mut entries = Vec::new();

    for (line_idx, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("HTTP ") {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(first) = parts.next() else { continue };
        let Some(second) = parts.next() else { continue };

        if is_http_method(first) {
            entries.push(Entry {
                method: first.to_string(),
                path: second.to_string(),
                line: line_idx as u32,
            });
        }
    }

    ParsedDocument { entries }
}

pub fn collect_diagnostics(text: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen_sections_in_request = BTreeSet::new();
    if let Err(error) = hurl_core::parser::parse_hurl_file(text) {
        let line = error.pos.line.saturating_sub(1) as u32;
        let character = error.pos.column.saturating_sub(1) as u32;
        diagnostics.push(Diagnostic {
            range: Range::new(
                Position::new(line, character),
                Position::new(line, character.saturating_add(1)),
            ),
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("hurl_core".into()),
            message: error.description(),
            ..Default::default()
        });
    }

    for (line_idx, raw_line) in text.lines().enumerate() {
        let trimmed = raw_line.trim();

        let known_variables = visible_variables_before_line(text, line_idx);

        if let Some(section_name) = section_name_from_line(trimmed) {
            let section = section_label(section_name);
            let section_key = canonical_section_name(section_name);
            if !is_known_section(section_name) {
                diagnostics.push(Diagnostic {
                    range: Range::new(
                        Position::new(line_idx as u32, 0),
                        Position::new(line_idx as u32, raw_line.len() as u32),
                    ),
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("hurl-lsp".into()),
                    message: format!("Unknown section `{section}`"),
                    ..Default::default()
                });
            }
            if !seen_sections_in_request.insert(section_key.to_string()) {
                diagnostics.push(Diagnostic {
                    range: Range::new(
                        Position::new(line_idx as u32, 0),
                        Position::new(line_idx as u32, raw_line.len() as u32),
                    ),
                    severity: Some(DiagnosticSeverity::WARNING),
                    source: Some("hurl-lsp".into()),
                    message: format!("Duplicate section `{section}`"),
                    ..Default::default()
                });
            }
        }

        if is_probable_method_line(trimmed) {
            let method = trimmed.split_whitespace().next().unwrap_or_default();
            if !is_http_method(method) {
                diagnostics.push(Diagnostic {
                    range: Range::new(
                        Position::new(line_idx as u32, 0),
                        Position::new(line_idx as u32, method.len() as u32),
                    ),
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("hurl-lsp".into()),
                    message: format!("Unknown HTTP method `{method}`"),
                    ..Default::default()
                });
            } else {
                seen_sections_in_request.clear();
            }
        }

        if let Some(status) = trimmed
            .strip_prefix("HTTP ")
            .and_then(|rest| rest.split_whitespace().next())
        {
            if !is_valid_status(status) {
                diagnostics.push(Diagnostic {
                    range: Range::new(
                        Position::new(line_idx as u32, 0),
                        Position::new(line_idx as u32, raw_line.len() as u32),
                    ),
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("hurl-lsp".into()),
                    message: format!("Invalid HTTP status code `{status}`"),
                    ..Default::default()
                });
            }
        }

        if trimmed.starts_with('#') {
            continue;
        }

        for (start, end, variable) in variable_placeholders(raw_line) {
            if !is_identifier(variable) || known_variables.contains(variable) {
                continue;
            }
            diagnostics.push(Diagnostic {
                range: Range::new(
                    Position::new(line_idx as u32, start as u32),
                    Position::new(line_idx as u32, end as u32),
                ),
                severity: Some(DiagnosticSeverity::WARNING),
                source: Some("hurl-lsp".into()),
                message: format!("Undefined variable `{{{{{variable}}}}}`"),
                ..Default::default()
            });
        }
    }

    diagnostics.sort_by_key(|diagnostic| {
        (
            diagnostic.range.start.line,
            diagnostic.range.start.character,
            diagnostic.message.clone(),
        )
    });
    diagnostics.dedup_by(|left, right| {
        left.range == right.range
            && left.message == right.message
            && left.severity == right.severity
    });

    diagnostics
}

fn is_valid_status(status: &str) -> bool {
    status == "*" || (status.len() == 3 && status.chars().all(|ch| ch.is_ascii_digit()))
}

fn is_probable_method_line(line: &str) -> bool {
    if line.is_empty()
        || line.starts_with('#')
        || section_name_from_line(line).is_some()
        || line.starts_with('{')
        || line.starts_with("HTTP ")
    {
        return false;
    }

    let token = line.split_whitespace().next().unwrap_or_default();
    token.chars().all(|ch| ch.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries() {
        let parsed = parse_document("GET https://example.com\nHTTP 200\n\nPOST /users\nHTTP 201\n");
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].method, "GET");
        assert_eq!(parsed.entries[1].path, "/users");
    }

    #[test]
    fn flags_unknown_section() {
        let diagnostics = collect_diagnostics("[Headerz]\nfoo: bar\n");
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Unknown section")));
    }

    #[test]
    fn warns_for_undefined_variables() {
        let diagnostics = collect_diagnostics("GET https://example.com/{{missing}}\nHTTP 200\n");
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Undefined variable")));
    }

    #[test]
    fn does_not_warn_for_captured_variables() {
        let diagnostics = collect_diagnostics(
            "GET https://example.com/users\nHTTP 200\n[Captures]\nuser_id: jsonpath \"$.id\"\n\nGET https://example.com/users/{{user_id}}\nHTTP 200\n",
        );
        assert!(!diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Undefined variable")));
    }

    #[test]
    fn warns_for_duplicate_section() {
        let diagnostics =
            collect_diagnostics("GET /users\nHTTP 200\n[Headers]\nx-a: b\n[Headers]\nx-c: d\n");
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Duplicate section")));
    }

    #[test]
    fn errors_for_invalid_http_status_format() {
        let diagnostics = collect_diagnostics("GET /users\nHTTP abc\n");
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Invalid HTTP status code")));
    }

    #[test]
    fn allows_http_star_status() {
        let diagnostics = collect_diagnostics("GET /users\nHTTP *\n");
        assert!(!diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Invalid HTTP status code")));
    }

    #[test]
    fn accepts_short_section_names() {
        let diagnostics = collect_diagnostics("GET /users\nHTTP 200\n[Query]\nid: 1\n");
        assert!(!diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Unknown section")));
    }

    #[test]
    fn ignores_json_array_as_section() {
        let diagnostics = collect_diagnostics("GET /users\nHTTP 200\n[\n  1,\n  2\n]\n");
        assert!(!diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Unknown section")));
    }

    #[test]
    fn does_not_treat_later_capture_as_visible() {
        let diagnostics = collect_diagnostics(
            "GET /users/{{user_id}}\nHTTP 200\n\nGET /users\nHTTP 200\n[Captures]\nuser_id: jsonpath \"$.id\"\n",
        );
        assert!(diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("Undefined variable `{{user_id}}`")));
    }

    #[test]
    fn recognizes_builtin_template_variables() {
        let diagnostics = collect_diagnostics("GET /users/{{newUuid}}\nHTTP 200\n");
        assert!(!diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Undefined variable")));
    }

    #[test]
    fn warns_for_duplicate_section_aliases() {
        let diagnostics =
            collect_diagnostics("GET /users\nHTTP 200\n[Query]\na: 1\n[QueryStringParams]\nb: 2\n");
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Duplicate section")));
    }
}
