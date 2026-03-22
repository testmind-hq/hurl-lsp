use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use tower_lsp::lsp_types::Url;

const OPENAPI_FILES: &[&str] = &[
    "openapi.yaml",
    "openapi.yml",
    "swagger.yaml",
    "swagger.yml",
    "swagger.json",
];

pub fn load_openapi_paths_with_roots(
    document_uri: &Url,
    workspace_roots: &[PathBuf],
) -> BTreeSet<String> {
    let Some(base_dir) = base_dir_from_uri(document_uri) else {
        return BTreeSet::new();
    };
    let dirs = bounded_ancestor_dirs(base_dir, workspace_roots);
    let mut paths = BTreeSet::new();

    for dir in dirs {
        for name in OPENAPI_FILES {
            let file_path = dir.join(name);
            if !file_path.exists() || !file_path.is_file() {
                continue;
            }
            for path in parse_openapi_paths(&file_path) {
                paths.insert(path);
            }
        }
    }
    paths
}

fn parse_openapi_paths(path: &Path) -> BTreeSet<String> {
    let Ok(content) = fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    let value = if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        serde_json::from_str::<serde_json::Value>(&content).ok()
    } else {
        serde_yaml::from_str::<serde_yaml::Value>(&content)
            .ok()
            .and_then(|yaml| serde_json::to_value(yaml).ok())
    };
    extract_paths(value.as_ref())
}

fn extract_paths(value: Option<&serde_json::Value>) -> BTreeSet<String> {
    let Some(value) = value else {
        return BTreeSet::new();
    };
    let Some(paths) = value.get("paths").and_then(|item| item.as_object()) else {
        return BTreeSet::new();
    };

    paths.keys().cloned().collect()
}

fn base_dir_from_uri(uri: &Url) -> Option<PathBuf> {
    if uri.scheme() != "file" {
        return None;
    }
    uri.to_file_path().ok()?.parent().map(Path::to_path_buf)
}

fn bounded_ancestor_dirs(base_dir: PathBuf, workspace_roots: &[PathBuf]) -> Vec<PathBuf> {
    let normalized_roots: Vec<PathBuf> = workspace_roots
        .iter()
        .map(|root| root.canonicalize().unwrap_or_else(|_| root.clone()))
        .collect();
    let normalized_base = base_dir.canonicalize().unwrap_or_else(|_| base_dir.clone());

    let selected_root = normalized_roots
        .iter()
        .filter(|root| normalized_base.starts_with(root))
        .max_by_key(|root| root.components().count())
        .cloned();

    let mut dirs = Vec::new();
    let mut current = Some(normalized_base);
    while let Some(dir) = current {
        if let Some(root) = &selected_root {
            if !dir.starts_with(root) {
                break;
            }
        }
        dirs.push(dir.clone());
        if let Some(root) = &selected_root {
            if dir == *root {
                break;
            }
        } else {
            break;
        }
        current = dir.parent().map(Path::to_path_buf);
    }
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn loads_paths_from_yaml() {
        let base = tmp_dir("hurl-lsp-openapi-yaml");
        fs::create_dir_all(&base).expect("mkdir");
        fs::write(
            base.join("openapi.yaml"),
            "openapi: 3.0.0\npaths:\n  /users: {}\n  /health: {}\n",
        )
        .expect("write yaml");
        let uri = Url::from_file_path(base.join("test.hurl")).expect("uri");

        let paths = load_openapi_paths_with_roots(&uri, std::slice::from_ref(&base));
        assert!(paths.contains("/users"));
        assert!(paths.contains("/health"));
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn loads_paths_from_json() {
        let base = tmp_dir("hurl-lsp-openapi-json");
        fs::create_dir_all(&base).expect("mkdir");
        fs::write(
            base.join("swagger.json"),
            r#"{"openapi":"3.0.0","paths":{"/orders":{},"/orders/{id}":{}}}"#,
        )
        .expect("write json");
        let uri = Url::from_file_path(base.join("test.hurl")).expect("uri");

        let paths = load_openapi_paths_with_roots(&uri, std::slice::from_ref(&base));
        assert!(paths.contains("/orders"));
        assert!(paths.contains("/orders/{id}"));
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn does_not_read_openapi_outside_workspace_root() {
        let base = tmp_dir("hurl-lsp-openapi-bounded");
        let workspace = base.join("workspace");
        let nested = workspace.join("api");
        fs::create_dir_all(&nested).expect("mkdir");
        fs::write(
            base.join("openapi.yaml"),
            "openapi: 3.0.0\npaths:\n  /outer: {}\n",
        )
        .expect("write outer");
        fs::write(
            workspace.join("openapi.yaml"),
            "openapi: 3.0.0\npaths:\n  /inner: {}\n",
        )
        .expect("write inner");
        let uri = Url::from_file_path(nested.join("test.hurl")).expect("uri");

        let paths = load_openapi_paths_with_roots(&uri, std::slice::from_ref(&workspace));
        assert!(paths.contains("/inner"));
        assert!(!paths.contains("/outer"));

        let _ = fs::remove_dir_all(base);
    }

    fn tmp_dir(prefix: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{now}"))
    }
}
