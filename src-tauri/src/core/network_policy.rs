use std::collections::HashSet;

use reqwest::Url;

use crate::core::config_manager::ConfigManager;

pub const DEFAULT_NETWORK_ALLOWLIST: &str =
    "api.anthropic.com,api.openai.com,translation.googleapis.com,translate.googleapis.com,api.tavily.com,duckduckgo.com,github.com";

pub fn allowlist_from_config(config: &ConfigManager) -> HashSet<String> {
    parse_allowlist(
        &config
            .get("security.network_allowlist")
            .unwrap_or_else(|| DEFAULT_NETWORK_ALLOWLIST.to_string()),
    )
}

pub fn allowlist_from_getter<F>(mut get: F) -> HashSet<String>
where
    F: FnMut(&str) -> Option<String>,
{
    parse_allowlist(
        &get("security.network_allowlist").unwrap_or_else(|| DEFAULT_NETWORK_ALLOWLIST.to_string()),
    )
}

pub fn configured_url(
    config: &ConfigManager,
    key: &str,
    default_url: &str,
    label: &str,
) -> Result<String, String> {
    let raw = config
        .get(key)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default_url.to_string());
    enforce_outbound_url(&raw, &allowlist_from_config(config), label)
}

pub fn enforce_outbound_url(
    raw_url: &str,
    allowed_hosts: &HashSet<String>,
    label: &str,
) -> Result<String, String> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} URL is empty"));
    }

    let parsed = Url::parse(trimmed).map_err(|e| format!("{label} URL is invalid: {e}"))?;
    let scheme = parsed.scheme().to_ascii_lowercase();
    if scheme != "https" && scheme != "http" {
        return Err(format!(
            "{label} must use http:// or https://; '{trimmed}' is not allowed"
        ));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| format!("{label} URL is missing a host"))?
        .to_ascii_lowercase();

    if scheme == "http" && !is_loopback_host(&host) {
        return Err(format!(
            "{label} must use HTTPS or explicit localhost HTTP; '{trimmed}' is not allowed"
        ));
    }

    if !is_loopback_host(&host) && !allowed_hosts.contains(&host) {
        return Err(format!(
            "{label} host '{host}' is not in security.network_allowlist"
        ));
    }

    Ok(trimmed.trim_end_matches('/').to_string())
}

pub fn enforce_known_endpoint(
    raw_url: &str,
    allowed_hosts: &HashSet<String>,
    label: &str,
) -> Result<(), String> {
    enforce_outbound_url(raw_url, allowed_hosts, label).map(|_| ())
}

pub fn parse_allowlist(raw: &str) -> HashSet<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_allowlist_normalizes_hosts() {
        let hosts = parse_allowlist(" api.openai.com , GITHUB.com ,,localhost ");
        assert!(hosts.contains("api.openai.com"));
        assert!(hosts.contains("github.com"));
        assert!(hosts.contains("localhost"));
        assert_eq!(hosts.len(), 3);
    }

    #[test]
    fn enforce_outbound_url_allows_https_allowlisted_hosts() {
        let hosts = parse_allowlist("api.openai.com");
        let url = enforce_outbound_url("https://api.openai.com/v1/", &hosts, "OpenAI base URL")
            .expect("allowlisted host");
        assert_eq!(url, "https://api.openai.com/v1");
    }

    #[test]
    fn enforce_outbound_url_allows_loopback_http() {
        let hosts = HashSet::new();
        assert_eq!(
            enforce_outbound_url("http://localhost:11434/", &hosts, "Ollama base URL",).unwrap(),
            "http://localhost:11434"
        );
    }

    #[test]
    fn enforce_outbound_url_rejects_remote_http() {
        let hosts = parse_allowlist("api.openai.com");
        let error = enforce_outbound_url("http://api.openai.com/v1", &hosts, "OpenAI base URL")
            .unwrap_err();
        assert!(error.contains("must use HTTPS or explicit localhost HTTP"));
    }

    #[test]
    fn enforce_outbound_url_rejects_non_allowlisted_remote_host() {
        let hosts = parse_allowlist("api.openai.com");
        let error = enforce_outbound_url("https://evil.example.com/v1", &hosts, "OpenAI base URL")
            .unwrap_err();
        assert!(error.contains("security.network_allowlist"));
    }
}
