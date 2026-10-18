//! Local model detection — identifies local model endpoints (Ollama, LM Studio, vLLM)
//! so pseudonymization can be skipped for them.

/// Check if a base URL points to a local/private endpoint.
/// Returns true for localhost, RFC1918 private IPs, Docker hostnames, etc.
pub fn is_local_model_base_url(base_url: &str) -> bool {
    let Ok(url) = url::Url::parse(base_url) else {
        return false;
    };
    let Some(hostname) = url.host_str() else {
        return false;
    };
    let hostname = hostname.to_lowercase();

    // Explicit localhost
    if matches!(hostname.as_str(), "localhost" | "127.0.0.1" | "::1") {
        return true;
    }

    // Private IPv4 ranges (RFC 1918)
    if let Ok(ip) = hostname.parse::<std::net::Ipv4Addr>() {
        let octets = ip.octets();
        if octets[0] == 10 {
            return true; // 10.0.0.0/8
        }
        if octets[0] == 172 && (16..=31).contains(&octets[1]) {
            return true; // 172.16.0.0/12
        }
        if octets[0] == 192 && octets[1] == 168 {
            return true; // 192.168.0.0/16
        }
        if octets[0] == 169 && octets[1] == 254 {
            return true; // 169.254.0.0/16 (link-local)
        }
    }

    // Docker/container hostnames
    if hostname == "host.docker.internal" || hostname.ends_with(".local") {
        return true;
    }

    false
}

/// Get the active model's base URL for a session.
/// Returns None if the session or model config is not available.
#[cfg(feature = "basemind")]
pub async fn get_active_model_base_url(session_id: &str) -> Option<String> {
    // This would need access to the session config to get the model's base_url
    // For now, return None to default to pseudonymization
    None
}

/// Determine if pseudonymization should run for a session.
/// Defaults to true (pseudonymize) if the model/base_url cannot be determined.
#[cfg(feature = "basemind")]
pub async fn should_pseudonymize(session_id: &str) -> bool {
    let base_url = get_active_model_base_url(session_id).await;
    match base_url {
        Some(url) => !is_local_model_base_url(&url),
        None => true, // Default to pseudonymize if unknown
    }
}