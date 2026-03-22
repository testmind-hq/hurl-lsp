use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, Position};

const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
const SECTIONS: &[&str] = &[
    "[Asserts]",
    "[Captures]",
    "[Cookies]",
    "[FormParams]",
    "[Headers]",
    "[Options]",
    "[QueryStringParameters]",
];
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

pub fn completions(text: &str, position: Position) -> Vec<CompletionItem> {
    let line = text.lines().nth(position.line as usize).unwrap_or_default();
    let prefix = &line[..(position.character as usize).min(line.len())];
    let trimmed = prefix.trim_start();

    if let Some(var_prefix) = variable_prefix(prefix) {
        let vars = known_variables(text);
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
        return SECTIONS
            .iter()
            .map(|section| keyword_item(section))
            .collect();
    }

    if in_asserts_block(text, position.line as usize) {
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

    METHODS.iter().map(|method| keyword_item(method)).collect()
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
    let mut current_section = "";

    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed;
        }
        if idx == line_idx {
            break;
        }
    }

    current_section == "[Asserts]"
}

fn variable_prefix(prefix: &str) -> Option<&str> {
    let idx = prefix.rfind("{{")?;
    let content = &prefix[(idx + 2)..];
    if content.contains("}}") {
        return None;
    }
    Some(content)
}

fn known_variables(text: &str) -> Vec<String> {
    let mut in_captures = false;
    let mut vars = Vec::new();

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
        if name.is_empty()
            || !name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        {
            continue;
        }
        if !vars.iter().any(|existing| existing == name) {
            vars.push(name.to_string());
        }
    }

    vars
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
}
