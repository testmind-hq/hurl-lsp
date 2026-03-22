use std::{
    collections::BTreeMap,
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

pub fn load_openapi_request_body_fields_with_roots(
    document_uri: &Url,
    workspace_roots: &[PathBuf],
) -> BTreeMap<String, BTreeSet<String>> {
    let Some(base_dir) = base_dir_from_uri(document_uri) else {
        return BTreeMap::new();
    };
    let dirs = bounded_ancestor_dirs(base_dir, workspace_roots);
    let mut fields = BTreeMap::<String, BTreeSet<String>>::new();

    for dir in dirs {
        for name in OPENAPI_FILES {
            let file_path = dir.join(name);
            if !file_path.exists() || !file_path.is_file() {
                continue;
            }
            let parsed = parse_openapi_request_body_fields(&file_path);
            for (key, props) in parsed {
                fields.entry(key).or_default().extend(props);
            }
        }
    }
    fields
}

pub fn load_openapi_response_fields_with_roots(
    document_uri: &Url,
    workspace_roots: &[PathBuf],
) -> BTreeMap<String, BTreeSet<String>> {
    let Some(base_dir) = base_dir_from_uri(document_uri) else {
        return BTreeMap::new();
    };
    let dirs = bounded_ancestor_dirs(base_dir, workspace_roots);
    let mut fields = BTreeMap::<String, BTreeSet<String>>::new();

    for dir in dirs {
        for name in OPENAPI_FILES {
            let file_path = dir.join(name);
            if !file_path.exists() || !file_path.is_file() {
                continue;
            }
            let parsed = parse_openapi_response_fields(&file_path);
            for (key, props) in parsed {
                fields.entry(key).or_default().extend(props);
            }
        }
    }
    fields
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

fn parse_openapi_request_body_fields(path: &Path) -> BTreeMap<String, BTreeSet<String>> {
    let Ok(content) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let value = if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        serde_json::from_str::<serde_json::Value>(&content).ok()
    } else {
        serde_yaml::from_str::<serde_yaml::Value>(&content)
            .ok()
            .and_then(|yaml| serde_json::to_value(yaml).ok())
    };
    extract_request_body_fields(value.as_ref())
}

fn parse_openapi_response_fields(path: &Path) -> BTreeMap<String, BTreeSet<String>> {
    let Ok(content) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let value = if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        serde_json::from_str::<serde_json::Value>(&content).ok()
    } else {
        serde_yaml::from_str::<serde_yaml::Value>(&content)
            .ok()
            .and_then(|yaml| serde_json::to_value(yaml).ok())
    };
    extract_response_fields(value.as_ref())
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

fn extract_request_body_fields(
    value: Option<&serde_json::Value>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();
    let Some(value) = value else {
        return out;
    };
    let Some(paths) = value.get("paths").and_then(|item| item.as_object()) else {
        return out;
    };

    for (path, operations) in paths {
        let Some(ops) = operations.as_object() else {
            continue;
        };
        for (method, op) in ops {
            let method_upper = method.to_ascii_uppercase();
            if !matches!(
                method_upper.as_str(),
                "GET"
                    | "POST"
                    | "PUT"
                    | "PATCH"
                    | "DELETE"
                    | "HEAD"
                    | "OPTIONS"
                    | "CONNECT"
                    | "TRACE"
            ) {
                continue;
            }
            let schema = op
                .get("requestBody")
                .and_then(|rb| rb.get("content"))
                .and_then(|c| c.get("application/json"))
                .and_then(|j| j.get("schema"));
            let Some(schema) = schema else {
                continue;
            };
            let mut properties = BTreeSet::new();
            collect_schema_property_names(schema, value, &mut properties, 0);
            if properties.is_empty() {
                continue;
            }

            let key = format!("{} {}", method_upper, path);
            let entry = out.entry(key).or_insert_with(BTreeSet::new);
            entry.extend(properties);
        }
    }

    out
}

fn extract_response_fields(
    value: Option<&serde_json::Value>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();
    let Some(value) = value else {
        return out;
    };
    let Some(paths) = value.get("paths").and_then(|item| item.as_object()) else {
        return out;
    };

    for (path, operations) in paths {
        let Some(ops) = operations.as_object() else {
            continue;
        };
        for (method, op) in ops {
            let method_upper = method.to_ascii_uppercase();
            if !matches!(
                method_upper.as_str(),
                "GET"
                    | "POST"
                    | "PUT"
                    | "PATCH"
                    | "DELETE"
                    | "HEAD"
                    | "OPTIONS"
                    | "CONNECT"
                    | "TRACE"
            ) {
                continue;
            }
            let Some(responses) = op.get("responses").and_then(|item| item.as_object()) else {
                continue;
            };
            for (status, response_raw) in responses {
                let response = resolve_local_ref(response_raw, value).unwrap_or(response_raw);
                let schema = response
                    .get("content")
                    .and_then(|c| c.get("application/json"))
                    .and_then(|j| j.get("schema"));
                let Some(schema) = schema else {
                    continue;
                };
                let mut properties = BTreeSet::new();
                collect_schema_property_names(schema, value, &mut properties, 0);
                if properties.is_empty() {
                    continue;
                }
                let key = format!("{method_upper} {path} {status}");
                out.entry(key).or_default().extend(properties);
            }
        }
    }

    out
}

fn resolve_local_ref<'a>(
    value: &'a serde_json::Value,
    root: &'a serde_json::Value,
) -> Option<&'a serde_json::Value> {
    let reference = value.get("$ref").and_then(|item| item.as_str())?;
    let pointer = reference.strip_prefix('#')?;
    root.pointer(pointer)
}

fn collect_schema_property_names(
    schema: &serde_json::Value,
    root: &serde_json::Value,
    out: &mut BTreeSet<String>,
    depth: usize,
) {
    if depth > 8 {
        return;
    }

    if let Some(properties) = schema.get("properties").and_then(|props| props.as_object()) {
        out.extend(properties.keys().cloned());
    }

    if let Some(reference) = schema.get("$ref").and_then(|item| item.as_str()) {
        if let Some(pointer) = reference.strip_prefix('#') {
            if let Some(resolved) = root.pointer(pointer) {
                collect_schema_property_names(resolved, root, out, depth + 1);
            }
        }
    }

    for keyword in ["allOf", "oneOf", "anyOf"] {
        if let Some(items) = schema.get(keyword).and_then(|item| item.as_array()) {
            for item in items {
                collect_schema_property_names(item, root, out, depth + 1);
            }
        }
    }
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

    #[test]
    fn loads_request_body_fields_from_yaml() {
        let base = tmp_dir("hurl-lsp-openapi-body-yaml");
        fs::create_dir_all(&base).expect("mkdir");
        fs::write(
            base.join("openapi.yaml"),
            "openapi: 3.0.0\npaths:\n  /users:\n    post:\n      requestBody:\n        content:\n          application/json:\n            schema:\n              type: object\n              properties:\n                email:\n                  type: string\n                age:\n                  type: integer\n",
        )
        .expect("write yaml");
        let uri = Url::from_file_path(base.join("test.hurl")).expect("uri");

        let fields = load_openapi_request_body_fields_with_roots(&uri, std::slice::from_ref(&base));
        let key = "POST /users".to_string();
        let props = fields.get(&key).expect("POST /users fields");
        assert!(props.contains("email"));
        assert!(props.contains("age"));

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn loads_request_body_fields_from_ref_schema() {
        let base = tmp_dir("hurl-lsp-openapi-body-ref");
        fs::create_dir_all(&base).expect("mkdir");
        fs::write(
            base.join("openapi.yaml"),
            "openapi: 3.0.0\npaths:\n  /users:\n    post:\n      requestBody:\n        content:\n          application/json:\n            schema:\n              $ref: '#/components/schemas/CreateUser'\ncomponents:\n  schemas:\n    CreateUser:\n      type: object\n      properties:\n        email:\n          type: string\n        age:\n          type: integer\n",
        )
        .expect("write yaml");
        let uri = Url::from_file_path(base.join("test.hurl")).expect("uri");

        let fields = load_openapi_request_body_fields_with_roots(&uri, std::slice::from_ref(&base));
        let key = "POST /users".to_string();
        let props = fields.get(&key).expect("POST /users fields");
        assert!(props.contains("email"));
        assert!(props.contains("age"));

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn loads_response_fields_from_ref_schema() {
        let base = tmp_dir("hurl-lsp-openapi-response-ref");
        fs::create_dir_all(&base).expect("mkdir");
        fs::write(
            base.join("openapi.yaml"),
            "openapi: 3.0.0\npaths:\n  /users:\n    post:\n      responses:\n        '201':\n          content:\n            application/json:\n              schema:\n                $ref: '#/components/schemas/UserCreated'\ncomponents:\n  schemas:\n    UserCreated:\n      type: object\n      properties:\n        id:\n          type: string\n        status:\n          type: string\n",
        )
        .expect("write yaml");
        let uri = Url::from_file_path(base.join("test.hurl")).expect("uri");

        let fields = load_openapi_response_fields_with_roots(&uri, std::slice::from_ref(&base));
        let key = "POST /users 201".to_string();
        let props = fields.get(&key).expect("POST /users 201 fields");
        assert!(props.contains("id"));
        assert!(props.contains("status"));

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn loads_response_fields_from_response_object_ref() {
        let base = tmp_dir("hurl-lsp-openapi-response-object-ref");
        fs::create_dir_all(&base).expect("mkdir");
        fs::write(
            base.join("openapi.yaml"),
            "openapi: 3.0.0\npaths:\n  /users:\n    post:\n      responses:\n        '201':\n          $ref: '#/components/responses/UserCreated'\ncomponents:\n  responses:\n    UserCreated:\n      description: user created\n      content:\n        application/json:\n          schema:\n            type: object\n            properties:\n              id:\n                type: string\n              status:\n                type: string\n",
        )
        .expect("write yaml");
        let uri = Url::from_file_path(base.join("test.hurl")).expect("uri");

        let fields = load_openapi_response_fields_with_roots(&uri, std::slice::from_ref(&base));
        let key = "POST /users 201".to_string();
        let props = fields.get(&key).expect("POST /users 201 fields");
        assert!(props.contains("id"));
        assert!(props.contains("status"));

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
