use crate::{
    syntax::variable_placeholders,
    variables::{is_sensitive_name, ResolvedVariable},
};
use hurl_core::{
    ast::{Bytes, MultipartParam},
    error::DisplaySourceError,
    parser::parse_hurl_file,
    types::ToSource,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurlCommand {
    pub command: String,
    pub display_command: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CurlBuildError {
    UnresolvedVariables(Vec<String>),
    Unsupported(String),
    Parse(String),
}

#[derive(Default)]
struct RequestParts {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    query: Vec<(String, String)>,
    auth: Option<(String, String)>,
    body: Option<String>,
    form: Vec<(String, String)>,
    multipart: Vec<(String, String)>,
    sensitive_values: Vec<String>,
}

pub fn build_curl_for_entry(
    text: &str,
    entry_line: usize,
    vars: &BTreeMap<String, ResolvedVariable>,
) -> Result<CurlCommand, CurlBuildError> {
    let file = parse_hurl_file(text).map_err(|error| CurlBuildError::Parse(error.description()))?;
    let parsed = crate::diagnostics::parse_document(text);
    let entry_index = parsed
        .entries
        .iter()
        .position(|entry| entry.line as usize == entry_line)
        .ok_or_else(|| {
            CurlBuildError::Unsupported("Unable to locate the selected request entry.".into())
        })?;
    let entry = file.entries.get(entry_index).ok_or_else(|| {
        CurlBuildError::Unsupported("Unable to locate the selected request entry.".into())
    })?;
    let request = &entry.request;
    let mut parts = RequestParts {
        method: request.method.to_string(),
        url: request.url.to_string(),
        ..Default::default()
    };
    for header in &request.headers {
        parts
            .headers
            .push((header.key.to_string(), header.value.to_string()));
    }
    for item in request.querystring_params() {
        parts
            .query
            .push((item.key.to_string(), item.value.to_string()));
    }
    for item in request.form_params() {
        parts
            .form
            .push((item.key.to_string(), item.value.to_string()));
    }
    for item in request.multipart_form_data() {
        match item {
            MultipartParam::Param(item) => parts
                .multipart
                .push((item.key.to_string(), item.value.to_string())),
            MultipartParam::FilenameParam(item) => parts
                .multipart
                .push((item.key.to_string(), format!("@{}", item.value.filename))),
        }
    }
    if let Some(auth) = request.basic_auth() {
        parts.auth = Some((auth.key.to_string(), auth.value.to_string()));
    }
    if let Some(body) = &request.body {
        parts.body = Some(match &body.value {
            Bytes::Json(value) => value.to_source().as_str().to_string(),
            Bytes::Xml(value) => value.clone(),
            Bytes::MultilineString(value) => value.value().to_string(),
            Bytes::OnelineString(value) => value.to_string(),
            Bytes::File(value) => format!("\0FILE:{}", value.filename),
            Bytes::Base64(_) | Bytes::Hex(_) => {
                return Err(CurlBuildError::Unsupported(
                    "Base64 and hex request bodies are not supported by static cURL export.".into(),
                ))
            }
        });
    }
    resolve_parts(&mut parts, vars)?;
    let command = render(&parts, false);
    let display_command = render(&parts, true);
    Ok(CurlCommand {
        command,
        display_command,
    })
}

fn resolve_parts(
    parts: &mut RequestParts,
    vars: &BTreeMap<String, ResolvedVariable>,
) -> Result<(), CurlBuildError> {
    let mut unresolved = BTreeSet::new();
    {
        let mut resolve = |value: &mut String| {
            let original = value.clone();
            let placeholders = variable_placeholders(&original);
            let mut output = original.clone();
            for (start, end, name) in placeholders.into_iter().rev() {
                if let Some(variable) = vars.get(name) {
                    output.replace_range(start..end, &variable.value);
                } else {
                    unresolved.insert(name.to_string());
                }
            }
            *value = output;
        };
        resolve(&mut parts.url);
        for (name, value) in &mut parts.headers {
            resolve(name);
            resolve(value);
        }
        for (name, value) in &mut parts.query {
            resolve(name);
            resolve(value);
        }
        for (name, value) in &mut parts.form {
            resolve(name);
            resolve(value);
        }
        for (name, value) in &mut parts.multipart {
            resolve(name);
            resolve(value);
        }
        if let Some((user, pass)) = &mut parts.auth {
            resolve(user);
            resolve(pass);
        }
        if let Some(body) = &mut parts.body {
            resolve(body);
        }
    }
    if !unresolved.is_empty() {
        return Err(CurlBuildError::UnresolvedVariables(
            unresolved.into_iter().collect(),
        ));
    }
    parts.sensitive_values = vars
        .values()
        .filter(|v| v.sensitive)
        .map(|v| v.value.clone())
        .filter(|v| !v.is_empty())
        .collect();
    Ok(())
}

fn render(parts: &RequestParts, mask: bool) -> String {
    let mut args = vec![
        "curl".to_string(),
        "-X".into(),
        quote(&parts.method),
        quote(&masked(&parts.url, parts, mask)),
    ];
    for (name, value) in &parts.headers {
        let sensitive = is_sensitive_name(name)
            || matches!(name.to_ascii_lowercase().as_str(), "cookie" | "set-cookie");
        let shown = if mask && sensitive {
            "••••••".into()
        } else {
            masked(value, parts, mask)
        };
        args.extend(["-H".into(), quote(&format!("{name}: {shown}"))]);
    }
    if !parts.query.is_empty() {
        args.push("--get".into());
        for (k, v) in &parts.query {
            args.extend([
                "--data-urlencode".into(),
                quote(&format!("{k}={}", masked(v, parts, mask))),
            ]);
        }
    }
    if let Some((user, pass)) = &parts.auth {
        args.extend([
            "--user".into(),
            quote(&format!(
                "{}:{}",
                masked(user, parts, mask),
                if mask {
                    "••••••".into()
                } else {
                    pass.clone()
                }
            )),
        ]);
    }
    for (k, v) in &parts.form {
        args.extend([
            "--data-urlencode".into(),
            quote(&format!("{k}={}", masked(v, parts, mask))),
        ]);
    }
    for (k, v) in &parts.multipart {
        args.extend([
            "--form".into(),
            quote(&format!("{k}={}", masked(v, parts, mask))),
        ]);
    }
    if let Some(body) = &parts.body {
        if let Some(filename) = body.strip_prefix("\0FILE:") {
            args.extend([
                "--data-binary".into(),
                quote(&format!("@{}", masked(filename, parts, mask))),
            ]);
        } else {
            args.extend(["--data-raw".into(), quote(&masked(body, parts, mask))]);
        }
    }
    args.join(" \\\n  ")
}

fn masked(value: &str, parts: &RequestParts, mask: bool) -> String {
    if !mask {
        return value.to_string();
    }
    parts
        .sensitive_values
        .iter()
        .fold(value.to_string(), |text, secret| {
            text.replace(secret, "••••••")
        })
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Url;
    fn var(name: &str, value: &str, sensitive: bool) -> ResolvedVariable {
        ResolvedVariable {
            name: name.into(),
            value: value.into(),
            uri: Url::parse("file:///tmp/vars.env").unwrap(),
            line: 0,
            start: 0,
            end: name.len() as u32,
            sensitive,
        }
    }

    #[test]
    fn builds_headers_body_variables_and_masks_secrets() {
        let text = "POST {{base_url}}/users\nX-Account-Email: {{email}}\nAuthorization: Bearer {{token}}\n{\"name\":\"O'Brien\"}\nHTTP 201\n";
        let vars = BTreeMap::from([
            (
                "base_url".into(),
                var("base_url", "https://example.com", false),
            ),
            ("email".into(), var("email", "user@example.com", false)),
            ("token".into(), var("token", "real-token", true)),
        ]);
        let curl = build_curl_for_entry(text, 0, &vars).expect("curl");
        assert!(curl.command.contains("'https://example.com/users'"));
        assert!(curl.command.contains("X-Account-Email: user@example.com"));
        assert!(curl.command.contains("O'\"'\"'Brien"));
        assert!(!curl.display_command.contains("real-token"));
        assert!(!curl.command.lines().any(|line| line.starts_with('+')));
        assert!(curl
            .command
            .lines()
            .skip(1)
            .all(|line| line.starts_with("  ")));
    }

    #[test]
    fn supports_query_form_basic_auth_and_multipart() {
        let text = "POST https://example.com/upload\n[Query]\npage: 2\n[BasicAuth]\nalice: secret\n[Multipart]\nlabel: avatar\nfile: file,photo.png;\nHTTP 200\n";
        let curl = build_curl_for_entry(text, 0, &BTreeMap::new()).expect("curl");
        assert!(curl.command.contains("--get"));
        assert!(curl.command.contains("page=2"));
        assert!(curl.command.contains("--user"));
        assert!(curl.command.contains("alice:secret"));
        assert!(curl.command.contains("--form"));
    }

    #[test]
    fn rejects_unresolved_variables() {
        let err = build_curl_for_entry("GET {{base}}/{{token}}\nHTTP 200\n", 0, &BTreeMap::new())
            .unwrap_err();
        assert_eq!(
            err,
            CurlBuildError::UnresolvedVariables(vec!["base".into(), "token".into()])
        );
    }

    #[test]
    fn locates_request_after_metadata_comments_and_resolves_spaced_placeholder() {
        let vars = BTreeMap::from([("host".into(), var("host", "example.com", false))]);
        let curl = build_curl_for_entry(
            "# title=Health\nGET https://{{ host }}/health\nHTTP 200\n",
            1,
            &vars,
        )
        .expect("curl");
        assert!(curl.command.contains("https://example.com/health"));
    }
}
