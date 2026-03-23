pub fn display_version() -> String {
    if let Some(raw) = option_env!("HURL_LSP_VERSION") {
        if let Some(normalized) = normalize_tag_version(raw) {
            return normalized;
        }
    }
    env!("CARGO_PKG_VERSION").to_string()
}

fn normalize_tag_version(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let stripped = trimmed.strip_prefix('v').unwrap_or(trimmed);
    if stripped
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_digit())
    {
        return Some(stripped.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::normalize_tag_version;

    #[test]
    fn normalize_tag_version_supports_prefixed_version() {
        assert_eq!(normalize_tag_version("v0.1.9"), Some("0.1.9".to_string()));
    }

    #[test]
    fn normalize_tag_version_supports_plain_version() {
        assert_eq!(normalize_tag_version("0.1.9"), Some("0.1.9".to_string()));
    }

    #[test]
    fn normalize_tag_version_rejects_non_version_like_values() {
        assert_eq!(normalize_tag_version("main"), None);
        assert_eq!(normalize_tag_version(""), None);
    }
}
