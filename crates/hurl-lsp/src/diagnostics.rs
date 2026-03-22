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

const HTTP_METHODS: &[&str] = &[
    "GET", "HEAD", "POST", "PUT", "DELETE", "CONNECT", "OPTIONS", "TRACE", "PATCH",
];
const SECTIONS: &[&str] = &[
    "[Asserts]",
    "[BasicAuth]",
    "[Captures]",
    "[Cookies]",
    "[FormParams]",
    "[Headers]",
    "[MultipartFormData]",
    "[Options]",
    "[QueryStringParams]",
    "[QueryStringParameters]",
];

pub fn parse_document(text: &str) -> ParsedDocument {
    let mut entries = Vec::new();

    for (line_idx, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with('[')
            || line.starts_with("HTTP ")
        {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(first) = parts.next() else {
            continue;
        };
        let Some(second) = parts.next() else {
            continue;
        };

        if HTTP_METHODS.contains(&first) {
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
    let known_variables = collect_known_variables(text);
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

        if trimmed.starts_with('[') && !SECTIONS.contains(&trimmed) {
            diagnostics.push(Diagnostic {
                range: Range::new(
                    Position::new(line_idx as u32, 0),
                    Position::new(line_idx as u32, raw_line.len() as u32),
                ),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("hurl-lsp".into()),
                message: format!("Unknown section `{trimmed}`"),
                ..Default::default()
            });
        }
        if trimmed.starts_with('[')
            && SECTIONS.contains(&trimmed)
            && !seen_sections_in_request.insert(trimmed.to_string())
        {
            diagnostics.push(Diagnostic {
                range: Range::new(
                    Position::new(line_idx as u32, 0),
                    Position::new(line_idx as u32, raw_line.len() as u32),
                ),
                severity: Some(DiagnosticSeverity::WARNING),
                source: Some("hurl-lsp".into()),
                message: format!("Duplicate section `{trimmed}`"),
                ..Default::default()
            });
        }

        if is_probable_method_line(trimmed) {
            let method = trimmed.split_whitespace().next().unwrap_or_default();
            if !HTTP_METHODS.contains(&method) {
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
            if known_variables.contains(variable) {
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
    status.len() == 3 && status.chars().all(|ch| ch.is_ascii_digit())
}

fn collect_known_variables(text: &str) -> BTreeSet<String> {
    let mut known = BTreeSet::new();
    let mut in_captures = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_captures = trimmed == "[Captures]";
            continue;
        }
        if !in_captures || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((name, _)) = trimmed.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        if name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        {
            known.insert(name.to_string());
        }
    }

    known
}

fn variable_placeholders(line: &str) -> Vec<(usize, usize, &str)> {
    let mut result = Vec::new();
    let mut offset = 0;

    while let Some(start) = line[offset..].find("{{") {
        let abs_start = offset + start;
        let content_start = abs_start + 2;
        let Some(end_rel) = line[content_start..].find("}}") else {
            break;
        };
        let content_end = content_start + end_rel;
        let variable = line[content_start..content_end].trim();
        if !variable.is_empty() {
            result.push((abs_start, content_end + 2, variable));
        }
        offset = content_end + 2;
    }

    result
}

fn is_probable_method_line(line: &str) -> bool {
    if line.is_empty()
        || line.starts_with('#')
        || line.starts_with('[')
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
}
