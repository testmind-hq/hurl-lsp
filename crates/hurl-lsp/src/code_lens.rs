use crate::diagnostics::parse_document;
use tower_lsp::lsp_types::{CodeLens, Command, Position, Range, Url};

pub const RUN_ENTRY_COMMAND: &str = "hurl.runEntry";
pub const RUN_ENTRY_WITH_VARS_COMMAND: &str = "hurl.runEntryWithVars";
pub const COPY_AS_CURL_COMMAND: &str = "hurl.copyAsCurl";
pub const NOOP_COMMAND: &str = "hurl.noop";

pub fn code_lenses(uri: &Url, text: &str) -> Vec<CodeLens> {
    let parsed = parse_document(text);
    let lines: Vec<&str> = text.lines().collect();

    parsed
        .entries
        .iter()
        .flat_map(|entry| {
            let start = Position::new(entry.line, 0);
            let range = Range::new(start, start);
            let (headers, asserts, captures) =
                count_sections_after_entry(text, entry.line as usize);
            let summary = CodeLens {
                range,
                command: Some(Command {
                    title: format!(
                        "📋 {} {}  │ {} headers │ {} asserts │ {} captures",
                        entry.method, entry.path, headers, asserts, captures
                    ),
                    command: NOOP_COMMAND.to_string(),
                    arguments: None,
                }),
                data: None,
            };
            let run = CodeLens {
                range,
                command: Some(Command {
                    title: "▶ Run".to_string(),
                    command: RUN_ENTRY_COMMAND.to_string(),
                    arguments: Some(vec![
                        serde_json::Value::String(uri.to_string()),
                        serde_json::Value::Number((entry.line as u64).into()),
                    ]),
                }),
                data: None,
            };
            let run_with_vars = CodeLens {
                range,
                command: Some(Command {
                    title: "⚡ Run with vars".to_string(),
                    command: RUN_ENTRY_WITH_VARS_COMMAND.to_string(),
                    arguments: Some(vec![
                        serde_json::Value::String(uri.to_string()),
                        serde_json::Value::Number((entry.line as u64).into()),
                    ]),
                }),
                data: None,
            };
            let copy_as_curl = CodeLens {
                range,
                command: Some(Command {
                    title: "📋 Copy as curl".to_string(),
                    command: COPY_AS_CURL_COMMAND.to_string(),
                    arguments: Some(vec![
                        serde_json::Value::String(uri.to_string()),
                        serde_json::Value::Number((entry.line as u64).into()),
                    ]),
                }),
                data: None,
            };
            [summary, run, run_with_vars, copy_as_curl]
        })
        .collect::<Vec<_>>()
        .into_iter()
        .filter(|lens| lens.range.start.line as usize <= lines.len())
        .collect()
}

fn count_sections_after_entry(text: &str, entry_line: usize) -> (usize, usize, usize) {
    let mut headers = 0;
    let mut asserts = 0;
    let mut captures = 0;
    let mut in_current_entry = false;

    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if idx == entry_line {
            in_current_entry = true;
            continue;
        }
        if !in_current_entry {
            continue;
        }
        if crate::syntax::method_from_line(trimmed).is_some() {
            break;
        }
        match trimmed {
            "[Headers]" => headers += 1,
            "[Asserts]" => asserts += 1,
            "[Captures]" => captures += 1,
            _ => {}
        }
    }

    (headers, asserts, captures)
}

pub fn build_curl_for_entry(text: &str, entry_line: usize) -> Option<String> {
    let parsed = parse_document(text);
    let entry = parsed
        .entries
        .iter()
        .find(|item| item.line as usize == entry_line)?;

    let mut headers = Vec::new();
    let mut in_headers = false;
    for (idx, raw) in text.lines().enumerate() {
        if idx <= entry_line {
            continue;
        }
        let line = raw.trim();
        if crate::syntax::method_from_line(line).is_some() {
            break;
        }
        if let Some(section) = crate::syntax::section_name_from_line(line) {
            in_headers = section == "Headers";
            continue;
        }
        if !in_headers || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push(format!("{}: {}", k.trim(), v.trim()));
        }
    }

    let mut command = format!(
        "curl -X {} '{}'",
        entry.method,
        shell_single_quote(&entry.path)
    );
    for header in headers {
        command.push_str(&format!(" -H '{}'", shell_single_quote(&header)));
    }
    Some(command)
}

fn shell_single_quote(value: &str) -> String {
    value.replace('\'', "'\"'\"'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_summary_and_run_lenses() {
        let uri = Url::parse("file:///tmp/test.hurl").expect("uri");
        let text = "GET /users\nHTTP 200\n[Headers]\na: b\n[Asserts]\nstatus == 200\n";
        let lenses = code_lenses(&uri, text);
        assert_eq!(lenses.len(), 4);
        assert!(lenses[0]
            .command
            .as_ref()
            .expect("summary")
            .title
            .contains("GET /users"));
        assert_eq!(
            lenses[1].command.as_ref().expect("run").command,
            RUN_ENTRY_COMMAND
        );
        assert_eq!(
            lenses[2].command.as_ref().expect("run vars").command,
            RUN_ENTRY_WITH_VARS_COMMAND
        );
        assert_eq!(
            lenses[3].command.as_ref().expect("copy").command,
            COPY_AS_CURL_COMMAND
        );
    }

    #[test]
    fn builds_curl_from_entry_line_and_headers() {
        let text = "POST https://example.com/users\nHTTP 201\n[Headers]\nContent-Type: application/json\nAuthorization: Bearer xxx\n";
        let curl = build_curl_for_entry(text, 0).expect("curl");
        assert!(curl.contains("curl -X POST"));
        assert!(curl.contains("-H 'Content-Type: application/json'"));
    }
}
