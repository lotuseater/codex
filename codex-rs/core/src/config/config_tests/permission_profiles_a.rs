use super::*;
use pretty_assertions::assert_eq;

#[test]
fn config_toml_deserializes_permission_profiles() {
    let toml = r#"
default_permissions = "dev"

[permissions.dev]
description = "Day-to-day workspace access."

[permissions.dev.workspace_roots]
"~/code/openai" = true
"~/code/ignored" = false

[permissions.dev.filesystem]
":minimal" = "read"
"/tmp/secret.env" = "deny"

[permissions.dev.filesystem.":workspace_roots"]
"." = "write"
"docs" = "read"

[permissions.dev.network]
enabled = true
proxy_url = "http://127.0.0.1:43128"
enable_socks5 = false
allow_upstream_proxy = false
mode = "full"

[permissions.dev.network.domains]
"openai.com" = "allow"

[permissions.dev.network.mitm.hooks.github_write]
host = "api.github.com"
methods = ["POST", "PUT"]
path_prefixes = ["/repos/openai/"]
action = ["strip_auth"]

[permissions.dev.network.mitm.actions.strip_auth]
strip_request_headers = ["authorization"]
"#;
    let cfg: ConfigToml =
        toml::from_str(toml).expect("TOML deserialization should succeed for permissions profiles");

    assert_eq!(cfg.default_permissions.as_deref(), Some("dev"));
    assert_eq!(
        cfg.permissions.expect("[permissions] should deserialize"),
        PermissionsToml {
            entries: BTreeMap::from([(
                "dev".to_string(),
                PermissionProfileToml {
                    description: Some("Day-to-day workspace access.".to_string()),
                    extends: None,
                    workspace_roots: Some(WorkspaceRootsToml {
                        entries: BTreeMap::from([
                            ("~/code/ignored".to_string(), false),
                            ("~/code/openai".to_string(), true),
                        ]),
                    }),
                    filesystem: Some(FilesystemPermissionsToml {
                        glob_scan_max_depth: None,
                        entries: BTreeMap::from([
                            (
                                ":minimal".to_string(),
                                FilesystemPermissionToml::Access(FileSystemAccessMode::Read),
                            ),
                            (
                                "/tmp/secret.env".to_string(),
                                FilesystemPermissionToml::Access(FileSystemAccessMode::Deny),
                            ),
                            (
                                ":workspace_roots".to_string(),
                                FilesystemPermissionToml::Scoped(BTreeMap::from([
                                    (".".to_string(), FileSystemAccessMode::Write),
                                    ("docs".to_string(), FileSystemAccessMode::Read),
                                ])),
                            ),
                        ]),
                    }),
                    network: Some(NetworkToml {
                        enabled: Some(true),
                        proxy_url: Some("http://127.0.0.1:43128".to_string()),
                        enable_socks5: Some(false),
                        socks_url: None,
                        enable_socks5_udp: None,
                        allow_upstream_proxy: Some(false),
                        dangerously_allow_non_loopback_proxy: None,
                        dangerously_allow_all_unix_sockets: None,
                        mode: Some(NetworkMode::Full),
                        domains: Some(NetworkDomainPermissionsToml {
                            entries: BTreeMap::from([(
                                "openai.com".to_string(),
                                NetworkDomainPermissionToml::Allow,
                            )]),
                        }),
                        unix_sockets: None,
                        allow_local_binding: None,
                        mitm: Some(NetworkMitmToml {
                            hooks: Some(IndexMap::from([(
                                "github_write".to_string(),
                                NetworkMitmHookToml {
                                    host: "api.github.com".to_string(),
                                    methods: vec!["POST".to_string(), "PUT".to_string()],
                                    path_prefixes: vec!["/repos/openai/".to_string()],
                                    query: BTreeMap::new(),
                                    headers: BTreeMap::new(),
                                    body: None,
                                    action: vec!["strip_auth".to_string()],
                                },
                            )])),
                            actions: Some(IndexMap::from([(
                                "strip_auth".to_string(),
                                NetworkMitmActionToml {
                                    strip_request_headers: vec!["authorization".to_string()],
                                    inject_request_headers: Vec::new(),
                                },
                            )])),
                        }),
                    }),
                },
            )]),
        }
    );
}

#[test]
fn config_toml_rejects_empty_mitm_action_reference_list() {
    let toml = r#"
default_permissions = "workspace"

[permissions.workspace.network.mitm.hooks.github_write]
host = "api.github.com"
methods = ["POST"]
path_prefixes = ["/repos/openai/"]
action = []

[permissions.workspace.network.mitm.actions.strip_auth]
strip_request_headers = ["authorization"]
"#;

    let err =
        toml::from_str::<ConfigToml>(toml).expect_err("empty MITM action refs should fail closed");

    assert!(
        err.to_string()
            .contains("network.mitm.hooks.github_write.action must not be empty"),
        "{err}"
    );
}

#[test]
fn config_toml_rejects_empty_mitm_action_definition() {
    let toml = r#"
default_permissions = "workspace"

[permissions.workspace.network.mitm.hooks.github_write]
host = "api.github.com"
methods = ["POST"]
path_prefixes = ["/repos/openai/"]
action = ["strip_auth"]

[permissions.workspace.network.mitm.actions.strip_auth]
"#;

    let err = toml::from_str::<ConfigToml>(toml)
        .expect_err("empty MITM action definitions should fail closed");

    assert!(
        err.to_string()
            .contains("network.mitm.actions.strip_auth must define at least one operation"),
        "{err}"
    );
}

#[test]
fn permissions_profile_network_to_proxy_config_preserves_mitm_hooks() {
    let network = NetworkToml {
        mode: Some(NetworkMode::Full),
        mitm: Some(NetworkMitmToml {
            hooks: Some(IndexMap::from([(
                "github_write".to_string(),
                NetworkMitmHookToml {
                    host: "api.github.com".to_string(),
                    methods: vec!["POST".to_string()],
                    path_prefixes: vec!["/repos/openai/".to_string()],
                    action: vec!["strip_auth".to_string()],
                    ..NetworkMitmHookToml::default()
                },
            )])),
            actions: Some(IndexMap::from([(
                "strip_auth".to_string(),
                NetworkMitmActionToml {
                    strip_request_headers: vec!["authorization".to_string()],
                    inject_request_headers: Vec::new(),
                },
            )])),
        }),
        ..NetworkToml::default()
    };

    let config = network.to_network_proxy_config();

    assert_eq!(config.network.mode, NetworkMode::Full);
    assert!(config.network.mitm);
    assert_eq!(config.network.mitm_hooks.len(), 1);
    assert_eq!(config.network.mitm_hooks[0].host, "api.github.com");
    assert_eq!(
        config.network.mitm_hooks[0].matcher.methods,
        vec!["POST".to_string()]
    );
    assert_eq!(
        config.network.mitm_hooks[0].actions.strip_request_headers,
        vec!["authorization".to_string()]
    );
}

#[test]
fn permissions_profile_network_to_proxy_config_preserves_mitm_hook_declaration_order() {
    let toml = r#"
default_permissions = "workspace"

[permissions.workspace.network.mitm.actions.noop]
strip_request_headers = ["authorization"]

[permissions.workspace.network.mitm.hooks.z_first]
host = "api.github.com"
methods = ["POST"]
path_prefixes = ["/repos/openai/"]
action = ["noop"]

[permissions.workspace.network.mitm.hooks.a_second]
host = "api.github.com"
methods = ["POST"]
path_prefixes = ["/repos/"]
action = ["noop"]
"#;
    let cfg: ConfigToml = toml::from_str(toml).expect("permissions profile should deserialize");
    let permissions = cfg.permissions.expect("permissions should deserialize");
    let network = permissions
        .entries
        .get("workspace")
        .expect("workspace profile should exist")
        .network
        .as_ref()
        .expect("network profile should exist");

    let config = network.to_network_proxy_config();

    assert_eq!(config.network.mitm_hooks.len(), 2);
    assert_eq!(
        config.network.mitm_hooks[0].matcher.path_prefixes,
        vec!["/repos/openai/".to_string()]
    );
    assert_eq!(
        config.network.mitm_hooks[1].matcher.path_prefixes,
        vec!["/repos/".to_string()]
    );
}
