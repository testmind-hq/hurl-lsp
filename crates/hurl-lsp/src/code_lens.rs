use crate::diagnostics::parse_document;
use tower_lsp::lsp_types::{CodeLens, Command, Position, Range, Url};

pub const RUN_ENTRY_COMMAND: &str = "hurl.runEntry";
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
            [summary, run]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_summary_and_run_lenses() {
        let uri = Url::parse("file:///tmp/test.hurl").expect("uri");
        let text = "GET /users\nHTTP 200\n[Headers]\na: b\n[Asserts]\nstatus == 200\n";
        let lenses = code_lenses(&uri, text);
        assert_eq!(lenses.len(), 2);
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
    }
}
