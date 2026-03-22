use std::collections::BTreeSet;

pub const HTTP_METHODS: &[&str] = &[
    "GET", "HEAD", "POST", "PUT", "DELETE", "CONNECT", "OPTIONS", "TRACE", "PATCH",
];

pub const SECTION_NAMES: &[&str] = &[
    "Asserts",
    "BasicAuth",
    "Captures",
    "Cookies",
    "Form",
    "FormParams",
    "Headers",
    "Multipart",
    "MultipartFormData",
    "Options",
    "Query",
    "QueryStringParams",
    "QueryStringParameters",
];

pub const BUILTIN_VARIABLES: &[&str] = &["newUuid", "newDate"];

pub fn is_http_method(method: &str) -> bool {
    HTTP_METHODS.contains(&method)
}

pub fn method_from_line(line: &str) -> Option<&str> {
    let mut parts = line.split_whitespace();
    let method = parts.next()?;
    let _path = parts.next()?;
    if is_http_method(method) {
        Some(method)
    } else {
        None
    }
}

pub fn section_name_from_line(line: &str) -> Option<&str> {
    if !(line.starts_with('[') && line.ends_with(']')) {
        return None;
    }
    let name = &line[1..line.len().saturating_sub(1)];
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    Some(name)
}

pub fn is_known_section(name: &str) -> bool {
    SECTION_NAMES.contains(&name)
}

pub fn section_label(name: &str) -> String {
    format!("[{name}]")
}

pub fn is_identifier(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

pub fn variable_placeholders(line: &str) -> Vec<(usize, usize, &str)> {
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

pub fn visible_variables_before_line(text: &str, target_line: usize) -> BTreeSet<String> {
    let mut visible = BTreeSet::new();
    for builtin in BUILTIN_VARIABLES {
        visible.insert((*builtin).to_string());
    }

    let mut in_captures = false;
    let mut captures_in_current_entry = BTreeSet::new();

    for (idx, line) in text.lines().enumerate() {
        if idx >= target_line {
            break;
        }
        let trimmed = line.trim();

        if method_from_line(trimmed).is_some() {
            visible.extend(captures_in_current_entry.iter().cloned());
            captures_in_current_entry.clear();
            in_captures = false;
            continue;
        }

        if let Some(section_name) = section_name_from_line(trimmed) {
            in_captures = section_name == "Captures";
            continue;
        }

        if !in_captures || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some((name, _)) = trimmed.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if is_identifier(name) {
            captures_in_current_entry.insert(name.to_string());
        }
    }

    if let Some(target) = text.lines().nth(target_line).map(str::trim) {
        if method_from_line(target).is_some() {
            visible.extend(captures_in_current_entry.iter().cloned());
        }
    }

    visible
}

pub fn capture_definitions_before_line(
    text: &str,
    target_line: usize,
) -> Vec<(usize, usize, usize, String)> {
    let mut defs = Vec::new();
    let mut in_captures = false;

    for (idx, line) in text.lines().enumerate() {
        if idx >= target_line {
            break;
        }
        let trimmed = line.trim();

        if method_from_line(trimmed).is_some() {
            in_captures = false;
            continue;
        }

        if let Some(section_name) = section_name_from_line(trimmed) {
            in_captures = section_name == "Captures";
            continue;
        }

        if !in_captures || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some((name, _)) = trimmed.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if !is_identifier(name) {
            continue;
        }
        let leading_ws = line.chars().take_while(|c| c.is_whitespace()).count();
        defs.push((idx, leading_ws, leading_ws + name.len(), name.to_string()));
    }

    defs
}
