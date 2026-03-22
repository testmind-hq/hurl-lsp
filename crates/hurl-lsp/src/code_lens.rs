use crate::{
    diagnostics::parse_document,
    execution::RunSummary,
    metadata::{infer_entry_dependencies, HurlMetaParser},
};
use std::collections::{BTreeMap, BTreeSet};
use tower_lsp::lsp_types::{CodeLens, Command, Position, Range, Url};

pub const RUN_ENTRY_COMMAND: &str = "hurl.runEntry";
pub const RUN_ENTRY_WITH_VARS_COMMAND: &str = "hurl.runEntryWithVars";
pub const RUN_CHAIN_COMMAND: &str = "hurl.runChain";
pub const RUN_FILE_COMMAND: &str = "hurl.runFile";
pub const COPY_AS_CURL_COMMAND: &str = "hurl.copyAsCurl";
pub const NOOP_COMMAND: &str = "hurl.noop";

#[cfg(test)]
pub fn code_lenses(uri: &Url, text: &str) -> Vec<CodeLens> {
    code_lenses_with_context(uri, text, &BTreeMap::new())
}

pub fn code_lenses_with_context(
    uri: &Url,
    text: &str,
    run_summaries: &BTreeMap<u32, RunSummary>,
) -> Vec<CodeLens> {
    let parsed = parse_document(text);
    let meta = HurlMetaParser::parse(text);
    let deps = infer_entry_dependencies(text, &meta);
    let mut deps_in = BTreeMap::<u32, BTreeSet<String>>::new();
    let mut deps_out = BTreeMap::<u32, BTreeSet<String>>::new();
    for dep in deps {
        let in_edge = if dep.variables.is_empty() {
            dep.from_step.clone()
        } else {
            format!("{} ← {}", dep.variables.join(", "), dep.from_step)
        };
        let out_edge = if dep.variables.is_empty() {
            dep.to_step.clone()
        } else {
            format!("{} → {}", dep.variables.join(", "), dep.to_step)
        };
        deps_in.entry(dep.to_line).or_default().insert(in_edge);
        deps_out.entry(dep.from_line).or_default().insert(out_edge);
    }
    let lines: Vec<&str> = text.lines().collect();

    parsed
        .entries
        .iter()
        .flat_map(|entry| {
            let start = Position::new(entry.line, 0);
            let range = Range::new(start, start);
            let (headers, asserts, captures) =
                count_sections_after_entry(text, entry.line as usize);
            let status = run_summaries.get(&entry.line).map(format_run_status_suffix);
            let title = if let Some(status) = status {
                format!(
                    "📋 {} {}  │ {} headers │ {} asserts │ {} captures │ {}",
                    entry.method, entry.path, headers, asserts, captures, status
                )
            } else {
                format!(
                    "📋 {} {}  │ {} headers │ {} asserts │ {} captures",
                    entry.method, entry.path, headers, asserts, captures
                )
            };
            let summary = CodeLens {
                range,
                command: Some(Command {
                    title,
                    command: NOOP_COMMAND.to_string(),
                    arguments: None,
                }),
                data: None,
            };
            let dep_title =
                dependency_title(entry.line, &deps_in, &deps_out).map(|title| CodeLens {
                    range,
                    command: Some(Command {
                        title,
                        command: NOOP_COMMAND.to_string(),
                        arguments: None,
                    }),
                    data: None,
                });
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
            let run_chain = CodeLens {
                range,
                command: Some(Command {
                    title: "⛓ Run chain".to_string(),
                    command: RUN_CHAIN_COMMAND.to_string(),
                    arguments: Some(vec![
                        serde_json::Value::String(uri.to_string()),
                        serde_json::Value::Number((entry.line as u64).into()),
                    ]),
                }),
                data: None,
            };
            let run_file = CodeLens {
                range,
                command: Some(Command {
                    title: "📄 Run file".to_string(),
                    command: RUN_FILE_COMMAND.to_string(),
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
            let mut items = vec![summary];
            if let Some(dep) = dep_title {
                items.push(dep);
            }
            items.push(run);
            items.push(run_with_vars);
            items.push(run_chain);
            items.push(run_file);
            items.push(copy_as_curl);
            items
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

pub fn extract_entry_text(text: &str, entry_line: usize) -> Option<String> {
    let parsed = parse_document(text);
    let mut entry_lines: Vec<usize> = parsed
        .entries
        .iter()
        .map(|entry| entry.line as usize)
        .collect();
    entry_lines.sort_unstable();
    let start = entry_lines
        .iter()
        .copied()
        .find(|line| *line == entry_line)?;
    let end = entry_lines
        .iter()
        .copied()
        .find(|line| *line > start)
        .unwrap_or_else(|| text.lines().count());
    let lines: Vec<&str> = text.lines().collect();
    Some(lines[start..end].join("\n"))
}

fn shell_single_quote(value: &str) -> String {
    value.replace('\'', "'\"'\"'")
}

fn format_run_status_suffix(summary: &RunSummary) -> String {
    let duration = summary
        .duration_ms
        .map(|value| format!("{value}ms"))
        .unwrap_or_else(|| "unknown".to_string());
    if summary.success {
        format!("✅ 上次执行通过 ({duration})")
    } else if summary.failed_asserts > 0 {
        format!(
            "❌ 上次执行失败 ({} assert failed · {duration})",
            summary.failed_asserts
        )
    } else {
        format!("❌ 上次执行失败 ({duration})")
    }
}

fn dependency_title(
    line: u32,
    deps_in: &BTreeMap<u32, BTreeSet<String>>,
    deps_out: &BTreeMap<u32, BTreeSet<String>>,
) -> Option<String> {
    let incoming = deps_in.get(&line).map(|items| {
        format!(
            "📥 依赖: {}",
            items.iter().cloned().collect::<Vec<_>>().join(" | ")
        )
    });
    let outgoing = deps_out.get(&line).map(|items| {
        format!(
            "📤 输出: {}",
            items.iter().cloned().collect::<Vec<_>>().join(" | ")
        )
    });
    match (incoming, outgoing) {
        (Some(left), Some(right)) => Some(format!("{left}  │  {right}")),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_summary_and_run_lenses() {
        let uri = Url::parse("file:///tmp/test.hurl").expect("uri");
        let text = "GET /users\nHTTP 200\n[Headers]\na: b\n[Asserts]\nstatus == 200\n";
        let lenses = code_lenses(&uri, text);
        assert_eq!(lenses.len(), 6);
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
            lenses[3].command.as_ref().expect("run chain").command,
            RUN_CHAIN_COMMAND
        );
        assert_eq!(
            lenses[4].command.as_ref().expect("run file").command,
            RUN_FILE_COMMAND
        );
        assert_eq!(
            lenses[5].command.as_ref().expect("copy").command,
            COPY_AS_CURL_COMMAND
        );
    }

    #[test]
    fn chain_entry_lens_shows_depends_on_annotation() {
        let uri = Url::parse("file:///tmp/test.hurl").expect("uri");
        let text = "# step_id=step-setup-user\nPOST /users\nHTTP 201\n[Captures]\nuser_id: jsonpath \"$.id\"\n\n# step_id=step-test-get\nGET /users/{{user_id}}\nHTTP 200\n";
        let lenses = code_lenses(&uri, text);
        assert!(lenses.iter().any(|item| {
            item.command
                .as_ref()
                .map(|cmd| cmd.title.contains("📥 依赖: user_id ← step-setup-user"))
                .unwrap_or(false)
        }));
    }

    #[test]
    fn chain_entry_lens_shows_explicit_depends_on_without_variables() {
        let uri = Url::parse("file:///tmp/test.hurl").expect("uri");
        let text = "# step_id=step-a\nPOST /users\nHTTP 201\n\n# step_id=step-b\n# depends_on=step-a\nGET /health\nHTTP 200\n";
        let lenses = code_lenses(&uri, text);
        assert!(lenses.iter().any(|item| {
            item.command
                .as_ref()
                .map(|cmd| cmd.title.contains("📥 依赖: step-a"))
                .unwrap_or(false)
        }));
    }

    #[test]
    fn summary_lens_shows_last_run_status() {
        let uri = Url::parse("file:///tmp/test.hurl").expect("uri");
        let text = "GET /users\nHTTP 200\n";
        let mut summaries = BTreeMap::new();
        summaries.insert(
            0,
            RunSummary {
                success: false,
                failed_asserts: 1,
                duration_ms: Some(230),
            },
        );
        let lenses = code_lenses_with_context(&uri, text, &summaries);
        assert!(lenses[0]
            .command
            .as_ref()
            .expect("summary")
            .title
            .contains("❌ 上次执行失败 (1 assert failed · 230ms)"));
    }

    #[test]
    fn builds_curl_from_entry_line_and_headers() {
        let text = "POST https://example.com/users\nHTTP 201\n[Headers]\nContent-Type: application/json\nAuthorization: Bearer xxx\n";
        let curl = build_curl_for_entry(text, 0).expect("curl");
        assert!(curl.contains("curl -X POST"));
        assert!(curl.contains("-H 'Content-Type: application/json'"));
    }

    #[test]
    fn extracts_only_target_entry_text() {
        let text = "GET /users\nHTTP 200\n\nPOST /orders\nHTTP 201\n";
        let entry = extract_entry_text(text, 3).expect("entry");
        assert!(entry.starts_with("POST /orders"));
        assert!(!entry.contains("GET /users"));
    }
}
