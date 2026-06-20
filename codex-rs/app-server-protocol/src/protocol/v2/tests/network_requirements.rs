use super::*;
use pretty_assertions::assert_eq;

#[test]
fn network_requirements_deserializes_legacy_fields() {
    let requirements: NetworkRequirements = serde_json::from_value(json!({
        "allowedDomains": ["api.openai.com"],
        "deniedDomains": ["blocked.example.com"],
        "allowUnixSockets": ["/tmp/proxy.sock"]
    }))
    .expect("legacy network requirements should deserialize");

    assert_eq!(
        requirements,
        NetworkRequirements {
            enabled: None,
            http_port: None,
            socks_port: None,
            allow_upstream_proxy: None,
            dangerously_allow_non_loopback_proxy: None,
            dangerously_allow_all_unix_sockets: None,
            domains: None,
            managed_allowed_domains_only: None,
            allowed_domains: Some(vec!["api.openai.com".to_string()]),
            denied_domains: Some(vec!["blocked.example.com".to_string()]),
            unix_sockets: None,
            allow_unix_sockets: Some(vec!["/tmp/proxy.sock".to_string()]),
            allow_local_binding: None,
        }
    );
}

#[test]
fn network_requirements_serializes_canonical_and_legacy_fields() {
    let requirements = NetworkRequirements {
        enabled: Some(true),
        http_port: Some(8080),
        socks_port: Some(1080),
        allow_upstream_proxy: Some(false),
        dangerously_allow_non_loopback_proxy: Some(false),
        dangerously_allow_all_unix_sockets: Some(true),
        domains: Some(BTreeMap::from([
            ("api.openai.com".to_string(), NetworkDomainPermission::Allow),
            (
                "blocked.example.com".to_string(),
                NetworkDomainPermission::Deny,
            ),
        ])),
        managed_allowed_domains_only: Some(true),
        allowed_domains: Some(vec!["api.openai.com".to_string()]),
        denied_domains: Some(vec!["blocked.example.com".to_string()]),
        unix_sockets: Some(BTreeMap::from([
            (
                "/tmp/proxy.sock".to_string(),
                NetworkUnixSocketPermission::Allow,
            ),
            (
                "/tmp/ignored.sock".to_string(),
                NetworkUnixSocketPermission::None,
            ),
        ])),
        allow_unix_sockets: Some(vec!["/tmp/proxy.sock".to_string()]),
        allow_local_binding: Some(true),
    };

    assert_eq!(
        serde_json::to_value(requirements).expect("network requirements should serialize"),
        json!({
            "enabled": true,
            "httpPort": 8080,
            "socksPort": 1080,
            "allowUpstreamProxy": false,
            "dangerouslyAllowNonLoopbackProxy": false,
            "dangerouslyAllowAllUnixSockets": true,
            "domains": {
                "api.openai.com": "allow",
                "blocked.example.com": "deny"
            },
            "managedAllowedDomainsOnly": true,
            "allowedDomains": ["api.openai.com"],
            "deniedDomains": ["blocked.example.com"],
            "unixSockets": {
                "/tmp/ignored.sock": "none",
                "/tmp/proxy.sock": "allow"
            },
            "allowUnixSockets": ["/tmp/proxy.sock"],
            "allowLocalBinding": true
        })
    );
}
