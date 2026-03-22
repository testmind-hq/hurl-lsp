use crate::{
    diagnostics::parse_document,
    metadata::{infer_entry_dependencies, CaseKind, HurlMetaParser, Priority, StepType},
};
use std::collections::{BTreeMap, BTreeSet};
use tower_lsp::lsp_types::{DocumentSymbol, Position, Range, SymbolKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutlineGroupMode {
    Hierarchical,
    Flat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutlineSortMode {
    Source,
    Priority,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OutlineConfig {
    group_mode: OutlineGroupMode,
    sort_mode: OutlineSortMode,
}

impl OutlineConfig {
    fn from_env() -> Self {
        let group_mode = match std::env::var("HURL_OUTLINE_GROUP_MODE")
            .unwrap_or_else(|_| "hierarchical".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "flat" => OutlineGroupMode::Flat,
            _ => OutlineGroupMode::Hierarchical,
        };
        let sort_mode = match std::env::var("HURL_OUTLINE_SORT_MODE")
            .unwrap_or_else(|_| "source".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "priority" => OutlineSortMode::Priority,
            _ => OutlineSortMode::Source,
        };
        Self {
            group_mode,
            sort_mode,
        }
    }
}

#[derive(Clone)]
struct EntryView {
    line: u32,
    method: String,
    path: String,
    meta: crate::metadata::EntryMeta,
}

#[allow(deprecated)]
pub fn document_symbols(text: &str) -> Vec<DocumentSymbol> {
    document_symbols_with_config(text, OutlineConfig::from_env())
}

#[allow(deprecated)]
fn document_symbols_with_config(text: &str, config: OutlineConfig) -> Vec<DocumentSymbol> {
    let parsed = parse_document(text);
    let meta = HurlMetaParser::parse(text);
    let len = parsed.entries.len().min(meta.entries.len());
    let mut entry_views = Vec::with_capacity(len);
    for idx in 0..len {
        let entry = &parsed.entries[idx];
        let entry_meta = &meta.entries[idx];
        entry_views.push(EntryView {
            line: entry.line,
            method: entry.method.clone(),
            path: entry.path.clone(),
            meta: entry_meta.clone(),
        });
    }

    let inferred = infer_entry_dependencies(text, &meta);
    let chain_lines: BTreeSet<u32> = meta
        .entries
        .iter()
        .filter(|entry| entry.case_kind == Some(CaseKind::Chain))
        .map(|entry| entry.line)
        .collect();
    let auto_chain_lines: BTreeSet<u32> = inferred
        .iter()
        .filter(|dep| !chain_lines.contains(&dep.from_line) && !chain_lines.contains(&dep.to_line))
        .flat_map(|dep| [dep.from_line, dep.to_line])
        .collect();

    match config.group_mode {
        OutlineGroupMode::Flat => {
            build_flat_symbols(&entry_views, &auto_chain_lines, config.sort_mode)
        }
        OutlineGroupMode::Hierarchical => {
            build_hierarchical_symbols(&entry_views, &auto_chain_lines, config.sort_mode)
        }
    }
}

#[allow(deprecated)]
fn build_flat_symbols(
    entry_views: &[EntryView],
    auto_chain_lines: &BTreeSet<u32>,
    sort_mode: OutlineSortMode,
) -> Vec<DocumentSymbol> {
    let mut leaves: Vec<(u32, Option<Priority>, Option<StepType>, DocumentSymbol)> = entry_views
        .iter()
        .map(|view| {
            let name = flat_entry_name(view, auto_chain_lines.contains(&view.line));
            (
                view.line,
                view.meta.priority.clone(),
                view.meta.step_type.clone(),
                leaf_symbol(view.line, name),
            )
        })
        .collect();
    sort_leafs(&mut leaves, sort_mode);
    leaves.into_iter().map(|(_, _, _, symbol)| symbol).collect()
}

#[allow(deprecated)]
fn build_hierarchical_symbols(
    entry_views: &[EntryView],
    auto_chain_lines: &BTreeSet<u32>,
    sort_mode: OutlineSortMode,
) -> Vec<DocumentSymbol> {
    let mut chains: BTreeMap<String, Vec<DocumentSymbol>> = BTreeMap::new();
    let mut chain_meta: BTreeMap<String, (Option<String>, Option<Priority>)> = BTreeMap::new();
    let mut chain_sort_keys: BTreeMap<String, u32> = BTreeMap::new();
    let mut single_groups: BTreeMap<&'static str, Vec<DocumentSymbol>> = BTreeMap::new();
    let mut fallback: Vec<(u32, Option<Priority>, Option<StepType>, DocumentSymbol)> = Vec::new();
    let mut auto_chain_children: Vec<(u32, Option<Priority>, Option<StepType>, DocumentSymbol)> =
        Vec::new();

    for view in entry_views {
        match view.meta.case_kind {
            Some(CaseKind::Chain) => {
                let case_id = view
                    .meta
                    .case_id
                    .clone()
                    .unwrap_or_else(|| "CHAIN".to_string());
                let step_name = chain_step_name(&view.meta, &view.method, &view.path);
                chains
                    .entry(case_id.clone())
                    .or_default()
                    .push(leaf_symbol(view.line, step_name));
                chain_meta
                    .entry(case_id)
                    .or_insert((view.meta.title.clone(), view.meta.priority.clone()));
                chain_sort_keys
                    .entry(
                        view.meta
                            .case_id
                            .clone()
                            .unwrap_or_else(|| "CHAIN".to_string()),
                    )
                    .and_modify(|line| *line = (*line).min(view.line))
                    .or_insert(view.line);
            }
            _ => {
                if auto_chain_lines.contains(&view.line) {
                    auto_chain_children.push((
                        view.line,
                        view.meta.priority.clone(),
                        view.meta.step_type.clone(),
                        leaf_symbol(view.line, format!("🔗 {} {}", view.method, view.path)),
                    ));
                    continue;
                }
                if let Some(priority) = &view.meta.priority {
                    let label = priority_label(priority);
                    let case_name = single_case_name(&view.meta, &view.method, &view.path);
                    single_groups
                        .entry(label)
                        .or_default()
                        .push(leaf_symbol(view.line, case_name));
                } else {
                    fallback.push((
                        view.line,
                        None,
                        None,
                        leaf_symbol(view.line, format!("○ {} {}", view.method, view.path)),
                    ));
                }
            }
        }
    }

    let mut root: Vec<(u8, u32, DocumentSymbol)> = Vec::new();
    for (case_id, mut children) in chains {
        if children.is_empty() {
            continue;
        }
        sort_group_children(&mut children, sort_mode);
        let (title, priority) = chain_meta.remove(&case_id).unwrap_or((None, None));
        let title = title.unwrap_or_default();
        let priority_suffix = priority
            .as_ref()
            .map(|p| format!(" [{}]", priority_plain(p)))
            .unwrap_or_default();
        let group_name = if title.is_empty() {
            format!("🔗 {case_id}{priority_suffix}")
        } else {
            format!("🔗 {case_id} {title}{priority_suffix}")
        };
        let start_line = chain_sort_keys.get(&case_id).copied().unwrap_or_else(|| {
            children
                .first()
                .map(|child| child.range.start.line)
                .unwrap_or_default()
        });
        root.push((
            group_priority_rank(priority.as_ref()),
            start_line,
            group_symbol(group_name, children),
        ));
    }

    if !auto_chain_children.is_empty() {
        sort_leafs(&mut auto_chain_children, sort_mode);
        let children: Vec<DocumentSymbol> = auto_chain_children
            .into_iter()
            .map(|(_, _, _, symbol)| symbol)
            .collect();
        let start_line = children
            .iter()
            .map(|child| child.range.start.line)
            .min()
            .unwrap_or_default();
        root.push((
            group_priority_rank(None),
            start_line,
            group_symbol("🔗 自动识别".to_string(), children),
        ));
    }

    for label in ["🟥 P0", "🟧 P1", "🟨 P2"] {
        if let Some(mut children) = single_groups.remove(label) {
            if !children.is_empty() {
                sort_group_children(&mut children, sort_mode);
                let group_priority = match label {
                    "🟥 P0" => Some(Priority::P0),
                    "🟧 P1" => Some(Priority::P1),
                    "🟨 P2" => Some(Priority::P2),
                    _ => None,
                };
                let start_line = children
                    .first()
                    .map(|child| child.range.start.line)
                    .unwrap_or_default();
                root.push((
                    group_priority_rank(group_priority.as_ref()),
                    start_line,
                    group_symbol(label.to_string(), children),
                ));
            }
        }
    }

    sort_leafs(&mut fallback, sort_mode);
    for (line, priority, _, symbol) in fallback {
        root.push((group_priority_rank(priority.as_ref()), line, symbol));
    }

    match sort_mode {
        OutlineSortMode::Source => root.sort_by_key(|(_, line, _)| *line),
        OutlineSortMode::Priority => root.sort_by_key(|(rank, line, _)| (*rank, *line)),
    }

    root.into_iter().map(|(_, _, symbol)| symbol).collect()
}

fn flat_entry_name(view: &EntryView, auto_chain: bool) -> String {
    if view.meta.case_kind == Some(CaseKind::Chain) {
        let case_id = view
            .meta
            .case_id
            .clone()
            .unwrap_or_else(|| "CHAIN".to_string());
        let step = chain_step_name(&view.meta, &view.method, &view.path);
        let priority = view
            .meta
            .priority
            .as_ref()
            .map(|p| format!(" [{}]", priority_plain(p)))
            .unwrap_or_default();
        return format!("🔗 {case_id} {step}{priority}");
    }
    if auto_chain {
        return format!("🔗 {} {}", view.method, view.path);
    }
    if let Some(priority) = &view.meta.priority {
        let case_name = single_case_name(&view.meta, &view.method, &view.path);
        return format!("{} {}", priority_label(priority), case_name);
    }
    format!("○ {} {}", view.method, view.path)
}

fn sort_group_children(children: &mut [DocumentSymbol], sort_mode: OutlineSortMode) {
    match sort_mode {
        OutlineSortMode::Source => children.sort_by_key(|child| child.range.start.line),
        OutlineSortMode::Priority => {
            children.sort_by_key(|child| (step_rank_from_name(&child.name), child.range.start.line))
        }
    }
}

fn sort_leafs(
    leaves: &mut [(u32, Option<Priority>, Option<StepType>, DocumentSymbol)],
    sort_mode: OutlineSortMode,
) {
    match sort_mode {
        OutlineSortMode::Source => leaves.sort_by_key(|(line, _, _, _)| *line),
        OutlineSortMode::Priority => leaves.sort_by_key(|(line, priority, step, _)| {
            (
                group_priority_rank(priority.as_ref()),
                step_rank(step.as_ref()),
                *line,
            )
        }),
    }
}

fn group_priority_rank(priority: Option<&Priority>) -> u8 {
    match priority {
        Some(Priority::P0) => 0,
        Some(Priority::P1) => 1,
        Some(Priority::P2) => 2,
        None => 3,
    }
}

fn step_rank(step: Option<&StepType>) -> u8 {
    match step {
        Some(StepType::Setup) => 0,
        Some(StepType::Test) => 1,
        Some(StepType::Teardown) => 2,
        None => 3,
    }
}

fn step_rank_from_name(name: &str) -> u8 {
    if name.starts_with("🔧") {
        return 0;
    }
    if name.starts_with("🧪") {
        return 1;
    }
    if name.starts_with("🧹") {
        return 2;
    }
    3
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
        .iter()
        .map(|child| child.range.start)
        .min_by_key(|pos| (pos.line, pos.character))
        .unwrap_or(Position::new(0, 0));
    let end = children
        .iter()
        .map(|child| child.range.end)
        .max_by_key(|pos| (pos.line, pos.character))
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
        let symbols = document_symbols_with_config(
            text,
            OutlineConfig {
                group_mode: OutlineGroupMode::Hierarchical,
                sort_mode: OutlineSortMode::Source,
            },
        );
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
        let symbols = document_symbols_with_config(
            text,
            OutlineConfig {
                group_mode: OutlineGroupMode::Hierarchical,
                sort_mode: OutlineSortMode::Source,
            },
        );
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
        let symbols = document_symbols_with_config(
            text,
            OutlineConfig {
                group_mode: OutlineGroupMode::Hierarchical,
                sort_mode: OutlineSortMode::Source,
            },
        );
        assert_eq!(symbols.len(), 1);
        assert!(symbols[0].name.starts_with("🔗 TC-CHAIN-001"));
        let children = symbols[0].children.as_ref().expect("chain children");
        assert_eq!(children[0].name, "🔧 Create user setup");
    }

    #[test]
    fn groups_inferred_chain_entries_under_auto_group() {
        let text = r#"
# step_id=setup
POST /users
HTTP 201
[Captures]
user_id: jsonpath "$.id"

# step_id=test
GET /users/{{user_id}}
HTTP 200
"#;
        let symbols = document_symbols_with_config(
            text,
            OutlineConfig {
                group_mode: OutlineGroupMode::Hierarchical,
                sort_mode: OutlineSortMode::Source,
            },
        );
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "🔗 自动识别");
        let children = symbols[0].children.as_ref().expect("auto children");
        assert_eq!(children.len(), 2);
        assert!(children[0].name.contains("POST /users"));
        assert!(children[1].name.contains("GET /users/{{user_id}}"));
    }

    #[test]
    fn flat_mode_returns_leaf_symbols_only() {
        let text = r#"
# case_id=TC-CHAIN-001
# case_kind=chain
# priority=P0
# step_id=setup
# step_type=setup
POST /users
HTTP 201
"#;
        let symbols = document_symbols_with_config(
            text,
            OutlineConfig {
                group_mode: OutlineGroupMode::Flat,
                sort_mode: OutlineSortMode::Source,
            },
        );
        assert_eq!(symbols.len(), 1);
        assert!(symbols[0].name.starts_with("🔗 TC-CHAIN-001"));
        assert!(symbols[0].children.is_none());
    }

    #[test]
    fn priority_sort_orders_p0_before_p2() {
        let text = r#"
# priority=P2
GET /slow
HTTP 200

# priority=P0
GET /critical
HTTP 200
"#;
        let symbols = document_symbols_with_config(
            text,
            OutlineConfig {
                group_mode: OutlineGroupMode::Hierarchical,
                sort_mode: OutlineSortMode::Priority,
            },
        );
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "🟥 P0");
        assert_eq!(symbols[1].name, "🟨 P2");
    }
}
