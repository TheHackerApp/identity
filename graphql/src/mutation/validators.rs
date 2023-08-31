use url::Url;

/// Check if the argument is a valid identifier
pub fn identifier(raw: &str) -> bool {
    raw.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Check if the argument is a valid URL
pub fn url(raw: &str) -> bool {
    match Url::parse(raw) {
        Ok(url) => {
            let scheme = url.scheme();
            (scheme == "http" || scheme == "https") && url.has_authority()
        }
        Err(_) => false,
    }
}
