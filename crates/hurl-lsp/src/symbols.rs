use crate::{
    diagnostics::parse_document,
    metadata::{CaseKind, HurlMetaParser, Priority, StepType},
};
use std::collections::BTreeMap;
use tower_lsp::lsp_types::{DocumentSymbol, Position, Range, SymbolKind};

#[allow(deprecated)]
pub fn document_symbols(text: &str) -> Vec<DocumentSymbol> {
    let parsed = parse_document(text);
    let meta = HurlMetaParser::parse(text);
    let len = parsed.entries.len().min(meta.entries.len());
    let mut chains: BTreeMap<String, Vec<DocumentSymbol>> = BTreeMap::new();
    let mut chain_meta: BTreeMap<String, (Option<String>, Option<Priority>)> = BTreeMap::new();
    let mut single_groups: BTreeMap<&'static str, Vec<DocumentSymbol>> = BTreeMap::new();
    let mut fallback = Vec::new();

    for idx in 0..len {
        let entry = &parsed.entries[idx];
        let entry_meta = &meta.entries[idx];
        match entry_meta.case_kind {
            Some(CaseKind::Chain) => {
                let case_id = entry_meta
                    .case_id
                    .clone()
                    .unwrap_or_else(|| "CHAIN".to_string());
                let step_name = chain_step_name(entry_meta, &entry.method, &entry.path);
                chains
                    .entry(case_id.clone())
                    .or_default()
                    .push(leaf_symbol(entry.line, step_name));
                chain_meta
                    .entry(case_id)
                    .or_insert((entry_meta.title.clone(), entry_meta.priority.clone()));
            }
            _ => {
                if let Some(priority) = &entry_meta.priority {
                    let label = priority_label(priority);
                    let case_name = single_case_name(entry_meta, &entry.method, &entry.path);
                    single_groups
                        .entry(label)
                        .or_default()
                        .push(leaf_symbol(entry.line, case_name));
                } else {
                    fallback.push(leaf_symbol(
                        entry.line,
                        format!("○ {} {}", entry.method, entry.path),
                    ));
                }
            }
        }
    }

    let mut root = Vec::new();
    for (case_id, children) in chains {
        if children.is_empty() {
            continue;
        }
        let (title, priority) = chain_meta.remove(&case_id).unwrap_or((None, None));
        let title = title.unwrap_or_default();
        let priority_suffix = priority
            .map(|p| format!(" [{}]", priority_plain(&p)))
            .unwrap_or_default();
        let group_name = if title.is_empty() {
            format!("🔗 {case_id}{priority_suffix}")
        } else {
            format!("🔗 {case_id} {title}{priority_suffix}")
        };
        root.push(group_symbol(group_name, children));
    }

    for label in ["🟥 P0", "🟧 P1", "🟨 P2"] {
        if let Some(children) = single_groups.remove(label) {
            if !children.is_empty() {
                root.push(group_symbol(label.to_string(), children));
            }
        }
    }

    root.extend(fallback);
    root
}

fn single_case_name(meta: &crate::metadata::EntryMeta, method: &str, path: &str) -> String {
    let mut name = match (&meta.case_id, &meta.title) {
        (Some(case_id), Some(title)) => format!("{case_id} {title}"),
        (Some(case_id), None) => format!("{case_id} {method} {path}"),
        (None, Some(title)) => title.clone(),
        (None, None) => format!("{method} {path}"),
    };
    if let Some(technique) = &meta.technique {
        name.push_str(&format!(" [{technique}]"));
    }
    name
}

fn chain_step_name(meta: &crate::metadata::EntryMeta, method: &str, path: &str) -> String {
    let icon = match meta.step_type {
        Some(StepType::Setup) => "🔧",
        Some(StepType::Test) => "🧪",
        Some(StepType::Teardown) => "🧹",
        None => "•",
    };
    let title = meta
        .title
        .clone()
        .unwrap_or_else(|| format!("{method} {path}"));
    match &meta.step_id {
        Some(step_id) => format!("{icon} {title} {step_id}"),
        None => format!("{icon} {title}"),
    }
}

fn priority_label(priority: &Priority) -> &'static str {
    match priority {
        Priority::P0 => "🟥 P0",
        Priority::P1 => "🟧 P1",
        Priority::P2 => "🟨 P2",
    }
}

fn priority_plain(priority: &Priority) -> &'static str {
    match priority {
        Priority::P0 => "P0",
        Priority::P1 => "P1",
        Priority::P2 => "P2",
    }
}

#[allow(deprecated)]
fn leaf_symbol(line: u32, name: String) -> DocumentSymbol {
    let range = Range::new(Position::new(line, 0), Position::new(line, 1));
    DocumentSymbol {
        name,
        detail: None,
        kind: SymbolKind::OBJECT,
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: None,
    }
}

#[allow(deprecated)]
fn group_symbol(name: String, children: Vec<DocumentSymbol>) -> DocumentSymbol {
    let start = children
        .first()
        .map(|child| child.range.start)
        .unwrap_or(Position::new(0, 0));
    let end = children
        .last()
        .map(|child| child.range.end)
        .unwrap_or(Position::new(0, 0));
    DocumentSymbol {
        name,
        detail: None,
        kind: SymbolKind::NAMESPACE,
        tags: None,
        deprecated: None,
        range: Range::new(start, end),
        selection_range: Range::new(start, start),
        children: Some(children),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_fallback_symbol_without_meta() {
        let text = "GET /users\nHTTP 200\n";
        let symbols = document_symbols(text);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "○ GET /users");
        assert_eq!(symbols[0].range.start.line, 0);
    }

    #[test]
    fn groups_single_cases_by_priority() {
        let text = r#"
# case_id=TC-0042
# case_kind=single
# priority=P1
# title=Invalid email
POST /users
HTTP 422
"#;
        let symbols = document_symbols(text);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "🟧 P1");
        let children = symbols[0].children.as_ref().expect("group children");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "TC-0042 Invalid email");
    }

    #[test]
    fn groups_chain_steps_under_case() {
        let text = r#"
# case_id=TC-CHAIN-001
# case_kind=chain
# priority=P0
# step_id=setup
# step_type=setup
# title=Create user
POST /users
HTTP 201
"#;
        let symbols = document_symbols(text);
        assert_eq!(symbols.len(), 1);
        assert!(symbols[0].name.starts_with("🔗 TC-CHAIN-001"));
        let children = symbols[0].children.as_ref().expect("chain children");
        assert_eq!(children[0].name, "🔧 Create user setup");
    }
}
