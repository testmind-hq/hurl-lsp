use crate::syntax::{
    is_identifier, method_from_line, section_label, section_name_from_line,
    visible_variables_before_line, HTTP_METHODS, SECTION_NAMES,
};
use std::collections::{BTreeMap, BTreeSet};
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, Position};

const ASSERTS: &[(&str, &str)] = &[
    ("jsonpath", "jsonpath \"$.field\" == value"),
    ("xpath", "xpath \"//node\" exists"),
    ("regex", "regex \"pattern\""),
    ("header", "header \"content-type\" == \"application/json\""),
    ("status", "status == 200"),
    ("duration", "duration < 1000"),
];
const CONTENT_TYPES: &[&str] = &[
    "application/json",
    "application/xml",
    "text/plain",
    "application/x-www-form-urlencoded",
];

#[cfg(test)]
pub fn completions(text: &str, position: Position) -> Vec<CompletionItem> {
    completions_with_external(
        text,
        position,
        &BTreeSet::new(),
        &BTreeSet::new(),
        &BTreeMap::new(),
    )
}

pub fn completions_with_external(
    text: &str,
    position: Position,
    external_variables: &BTreeSet<String>,
    openapi_paths: &BTreeSet<String>,
    openapi_body_fields: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<CompletionItem> {
    let line_idx = position.line as usize;
    let line = text.lines().nth(position.line as usize).unwrap_or_default();
    let prefix = &line[..(position.character as usize).min(line.len())];
    let trimmed = prefix.trim_start();

    if let Some(var_prefix) = variable_prefix(prefix) {
        let vars = known_variables(text, line_idx, external_variables);
        return vars
            .into_iter()
            .filter(|name| name.starts_with(var_prefix))
            .map(|name| CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some("Captured variable".into()),
                insert_text: Some([name.as_str(), "}}"].concat()),
                ..Default::default()
            })
            .collect();
    }

    if trimmed.starts_with('[') {
        return SECTION_NAMES
            .iter()
            .map(|section| keyword_item(&section_label(section)))
            .collect();
    }

    if in_asserts_block(text, line_idx) {
        return ASSERTS
            .iter()
            .map(|(label, detail)| CompletionItem {
                label: (*label).into(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some((*detail).into()),
                insert_text: Some(format!("{label} ")),
                ..Default::default()
            })
            .collect();
    }

    if prefix.contains("Content-Type:") {
        return CONTENT_TYPES
            .iter()
            .map(|item| keyword_item(item))
            .collect();
    }

    if let Some(path_prefix) = request_path_prefix(prefix) {
        return openapi_paths
            .iter()
            .filter(|path| path.starts_with(path_prefix))
            .map(|path| CompletionItem {
                label: path.clone(),
                kind: Some(CompletionItemKind::REFERENCE),
                insert_text: Some(path.clone()),
                detail: Some("OpenAPI path".into()),
                ..Default::default()
            })
            .collect();
    }

    if let Some(key_prefix) = json_body_key_prefix(prefix) {
        if let Some(operation_key) = current_operation_key(text, line_idx) {
            if let Some(fields) = openapi_body_fields.get(&operation_key) {
                return fields
                    .iter()
                    .filter(|field| field.starts_with(key_prefix))
                    .map(|field| CompletionItem {
                        label: field.clone(),
                        kind: Some(CompletionItemKind::FIELD),
                        insert_text: Some(field.clone()),
                        detail: Some("OpenAPI request body field".into()),
                        ..Default::default()
                    })
                    .collect();
            }
        }
    }

    HTTP_METHODS
        .iter()
        .map(|method| keyword_item(method))
        .collect()
}

fn keyword_item(value: &str) -> CompletionItem {
    CompletionItem {
        label: value.into(),
        kind: Some(CompletionItemKind::KEYWORD),
        insert_text: Some(value.into()),
        ..Default::default()
    }
}

fn in_asserts_block(text: &str, line_idx: usize) -> bool {
    let mut current_section = None;

    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if method_from_line(trimmed).is_some() || looks_like_request_start(trimmed) {
            current_section = None;
        } else if let Some(name) = section_name_from_line(trimmed) {
            current_section = Some(name);
        }
        if idx == line_idx {
            break;
        }
    }

    current_section == Some("Asserts")
}

fn looks_like_request_start(line: &str) -> bool {
    if line.is_empty() || line.starts_with('#') || line.starts_with('[') || line.starts_with('{') {
        return false;
    }
    let token = line.split_whitespace().next().unwrap_or_default();
    if token.is_empty() || !token.chars().all(|ch| ch.is_ascii_uppercase()) {
        return false;
    }
    HTTP_METHODS.iter().any(|method| method.starts_with(token))
}

fn variable_prefix(prefix: &str) -> Option<&str> {
    let idx = prefix.rfind("{{")?;
    let content = &prefix[(idx + 2)..];
    if content.contains("}}") {
        return None;
    }
    Some(content)
}

fn request_path_prefix(prefix: &str) -> Option<&str> {
    let trimmed = prefix.trim_start();
    let mut parts = trimmed.split_whitespace();
    let method = parts.next()?;
    if !crate::syntax::is_http_method(method) {
        return None;
    }
    let path = parts.next().unwrap_or_default();
    Some(path)
}

fn json_body_key_prefix(prefix: &str) -> Option<&str> {
    let idx = prefix.rfind('"')?;
    let tail = &prefix[(idx + 1)..];
    if tail.contains('"') {
        return None;
    }
    let before = prefix[..idx].trim_end();
    if before.trim().is_empty() {
        return Some(tail);
    }
    let prev = before.chars().last()?;
    if prev != '{' && prev != ',' {
        return None;
    }
    Some(tail)
}

fn current_operation_key(text: &str, line_idx: usize) -> Option<String> {
    let mut method = None::<String>;
    let mut path = None::<String>;
    let mut in_body = false;

    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(_m) = method_from_line(trimmed) {
            let mut parts = trimmed.split_whitespace();
            let m = parts.next().unwrap_or_default();
            let p = parts.next().unwrap_or_default();
            method = Some(m.to_string());
            path = Some(p.to_string());
            in_body = true;
        } else if trimmed.starts_with("HTTP ") || section_name_from_line(trimmed).is_some() {
            in_body = false;
        }

        if idx == line_idx {
            break;
        }
    }

    if !in_body {
        return None;
    }
    Some(format!("{} {}", method?, path?))
}

fn known_variables(
    text: &str,
    line_idx: usize,
    external_variables: &BTreeSet<String>,
) -> Vec<String> {
    let mut vars = visible_variables_before_line(text, line_idx);
    vars.extend(external_variables.iter().cloned());
    vars.into_iter()
        .filter(|name| is_identifier(name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_method_completions_by_default() {
        let items = completions("", Position::new(0, 0));
        assert!(items.iter().any(|item| item.label == "GET"));
    }

    #[test]
    fn returns_assert_completions_inside_asserts_block() {
        let items = completions("[Asserts]\njs", Position::new(1, 2));
        assert!(items.iter().any(|item| item.label == "jsonpath"));
    }

    #[test]
    fn returns_variable_completions_from_captures() {
        let text = "[Captures]\nuser_id: jsonpath \"$.id\"\n\nGET /users/{{u";
        let items = completions(text, Position::new(3, 13));
        assert!(items.iter().any(|item| item.label == "user_id"));
    }

    #[test]
    fn returns_external_variables() {
        let text = "GET /users/{{ho";
        let mut vars = BTreeSet::new();
        vars.insert("host".to_string());
        let items = completions_with_external(
            text,
            Position::new(0, 15),
            &vars,
            &BTreeSet::new(),
            &BTreeMap::new(),
        );
        assert!(items.iter().any(|item| item.label == "host"));
    }

    #[test]
    fn returns_openapi_path_completion() {
        let text = "GET /us";
        let mut paths = BTreeSet::new();
        paths.insert("/users".to_string());
        paths.insert("/orders".to_string());
        let items = completions_with_external(
            text,
            Position::new(0, 7),
            &BTreeSet::new(),
            &paths,
            &BTreeMap::new(),
        );
        assert!(items.iter().any(|item| item.label == "/users"));
        assert!(!items.iter().any(|item| item.label == "/orders"));
    }

    #[test]
    fn returns_openapi_body_field_completion() {
        let text = "POST /users\n{\n  \"e\n}\nHTTP 201\n";
        let mut fields = BTreeMap::new();
        let mut props = BTreeSet::new();
        props.insert("email".to_string());
        props.insert("age".to_string());
        fields.insert("POST /users".to_string(), props);

        let items = completions_with_external(
            text,
            Position::new(2, 4),
            &BTreeSet::new(),
            &BTreeSet::new(),
            &fields,
        );
        assert!(items.iter().any(|item| item.label == "email"));
        assert!(!items.iter().any(|item| item.label == "age"));
    }

    #[test]
    fn does_not_return_openapi_body_field_completion_inside_string_value() {
        let text = "POST /users\n{\n  \"note\": \"e\n}\nHTTP 201\n";
        let mut fields = BTreeMap::new();
        let mut props = BTreeSet::new();
        props.insert("email".to_string());
        fields.insert("POST /users".to_string(), props);

        let items = completions_with_external(
            text,
            Position::new(2, 13),
            &BTreeSet::new(),
            &BTreeSet::new(),
            &fields,
        );
        assert!(!items.iter().any(|item| item.label == "email"));
    }

    #[test]
    fn resets_assert_context_on_next_request() {
        let text = "GET /a\nHTTP 200\n[Asserts]\njsonpath \"$.id\" == 1\n\nGET /b\n";
        let items = completions(text, Position::new(5, 2));
        assert!(!items.iter().any(|item| item.label == "jsonpath"));
    }

    #[test]
    fn supports_short_section_completion() {
        let items = completions("[Q", Position::new(0, 2));
        assert!(items.iter().any(|item| item.label == "[Query]"));
        assert!(items.iter().any(|item| item.label == "[Form]"));
    }

    #[test]
    fn does_not_suggest_future_capture_variable() {
        let text = "GET /users/{{u}}\nHTTP 200\n\nGET /a\nHTTP 200\n[Captures]\nuser_id: jsonpath \"$.id\"\n";
        let items = completions(text, Position::new(0, 14));
        assert!(!items.iter().any(|item| item.label == "user_id"));
    }

    #[test]
    fn exits_assert_context_on_partial_method_line() {
        let text = "GET /a\nHTTP 200\n[Asserts]\njsonpath \"$.id\" == 1\n\nGE";
        let items = completions(text, Position::new(5, 2));
        assert!(!items.iter().any(|item| item.label == "jsonpath"));
        assert!(items.iter().any(|item| item.label == "GET"));
    }

    #[test]
    fn exits_assert_context_on_partial_connect_line() {
        let text = "GET /a\nHTTP 200\n[Asserts]\njsonpath \"$.id\" == 1\n\nCON";
        let items = completions(text, Position::new(5, 3));
        assert!(!items.iter().any(|item| item.label == "jsonpath"));
        assert!(items.iter().any(|item| item.label == "CONNECT"));
    }
}
