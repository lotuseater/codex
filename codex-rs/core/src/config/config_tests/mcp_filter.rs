use super::common::*;
use super::*;
use pretty_assertions::assert_eq;

#[test]
fn filter_mcp_servers_by_allowlist_enforces_identity_rules() {
    const MISMATCHED_COMMAND_SERVER: &str = "mismatched-command-should-disable";
    const MISMATCHED_URL_SERVER: &str = "mismatched-url-should-disable";
    const MATCHED_COMMAND_SERVER: &str = "matched-command-should-allow";
    const MATCHED_URL_SERVER: &str = "matched-url-should-allow";
    const DIFFERENT_NAME_SERVER: &str = "different-name-should-disable";

    const GOOD_CMD: &str = "good-cmd";
    const GOOD_URL: &str = "https://example.com/good";

    let mut servers = HashMap::from([
        (MISMATCHED_COMMAND_SERVER.to_string(), stdio_mcp("docs-cmd")),
        (
            MISMATCHED_URL_SERVER.to_string(),
            http_mcp("https://example.com/mcp"),
        ),
        (MATCHED_COMMAND_SERVER.to_string(), stdio_mcp(GOOD_CMD)),
        (MATCHED_URL_SERVER.to_string(), http_mcp(GOOD_URL)),
        (DIFFERENT_NAME_SERVER.to_string(), stdio_mcp("same-cmd")),
    ]);
    let source = RequirementSource::LegacyManagedConfigTomlFromMdm;
    let requirements = Sourced::new(
        BTreeMap::from([
            (
                MISMATCHED_URL_SERVER.to_string(),
                McpServerRequirement {
                    identity: McpServerIdentity::Url {
                        url: "https://example.com/other".to_string(),
                    },
                },
            ),
            (
                MISMATCHED_COMMAND_SERVER.to_string(),
                McpServerRequirement {
                    identity: McpServerIdentity::Command {
                        command: "other-cmd".to_string(),
                    },
                },
            ),
            (
                MATCHED_URL_SERVER.to_string(),
                McpServerRequirement {
                    identity: McpServerIdentity::Url {
                        url: GOOD_URL.to_string(),
                    },
                },
            ),
            (
                MATCHED_COMMAND_SERVER.to_string(),
                McpServerRequirement {
                    identity: McpServerIdentity::Command {
                        command: GOOD_CMD.to_string(),
                    },
                },
            ),
        ]),
        source.clone(),
    );
    filter_mcp_servers_by_requirements(&mut servers, Some(&requirements));

    let reason = Some(McpServerDisabledReason::Requirements { source });
    assert_eq!(
        servers
            .iter()
            .map(|(name, server)| (
                name.clone(),
                (server.enabled, server.disabled_reason.clone())
            ))
            .collect::<HashMap<String, (bool, Option<McpServerDisabledReason>)>>(),
        HashMap::from([
            (MISMATCHED_URL_SERVER.to_string(), (false, reason.clone())),
            (
                MISMATCHED_COMMAND_SERVER.to_string(),
                (false, reason.clone()),
            ),
            (MATCHED_URL_SERVER.to_string(), (true, None)),
            (MATCHED_COMMAND_SERVER.to_string(), (true, None)),
            (DIFFERENT_NAME_SERVER.to_string(), (false, reason)),
        ])
    );
}

#[test]
fn filter_mcp_servers_by_allowlist_allows_all_when_unset() {
    let mut servers = HashMap::from([
        ("server-a".to_string(), stdio_mcp("cmd-a")),
        ("server-b".to_string(), http_mcp("https://example.com/b")),
    ]);

    filter_mcp_servers_by_requirements(&mut servers, /*mcp_requirements*/ None);

    assert_eq!(
        servers
            .iter()
            .map(|(name, server)| (
                name.clone(),
                (server.enabled, server.disabled_reason.clone())
            ))
            .collect::<HashMap<String, (bool, Option<McpServerDisabledReason>)>>(),
        HashMap::from([
            ("server-a".to_string(), (true, None)),
            ("server-b".to_string(), (true, None)),
        ])
    );
}

#[test]
fn filter_mcp_servers_by_allowlist_blocks_all_when_empty() {
    let mut servers = HashMap::from([
        ("server-a".to_string(), stdio_mcp("cmd-a")),
        ("server-b".to_string(), http_mcp("https://example.com/b")),
    ]);

    let source = RequirementSource::LegacyManagedConfigTomlFromMdm;
    let requirements = Sourced::new(BTreeMap::new(), source.clone());
    filter_mcp_servers_by_requirements(&mut servers, Some(&requirements));

    let reason = Some(McpServerDisabledReason::Requirements { source });
    assert_eq!(
        servers
            .iter()
            .map(|(name, server)| (
                name.clone(),
                (server.enabled, server.disabled_reason.clone())
            ))
            .collect::<HashMap<String, (bool, Option<McpServerDisabledReason>)>>(),
        HashMap::from([
            ("server-a".to_string(), (false, reason.clone())),
            ("server-b".to_string(), (false, reason)),
        ])
    );
}

#[test]
fn filter_plugin_mcp_servers_by_allowlist_enforces_plugin_and_identity_rules() {
    const MATCHED_SERVER: &str = "matched-should-allow";
    const MISMATCHED_SERVER: &str = "mismatched-should-disable";
    const UNLISTED_SERVER: &str = "unlisted-should-disable";
    const GOOD_CMD: &str = "good-cmd";

    let mut servers = HashMap::from([
        (MATCHED_SERVER.to_string(), stdio_mcp(GOOD_CMD)),
        (MISMATCHED_SERVER.to_string(), stdio_mcp("bad-cmd")),
        (
            UNLISTED_SERVER.to_string(),
            http_mcp("https://example.com/mcp"),
        ),
    ]);
    let source = RequirementSource::EnterpriseManaged {
        id: "cloud_requirements".to_string(),
        name: "Cloud requirements".to_string(),
    };
    let requirements = Sourced::new(
        BTreeMap::from([(
            "sample@test".to_string(),
            codex_config::PluginRequirementsToml {
                mcp_servers: Some(BTreeMap::from([
                    (
                        MATCHED_SERVER.to_string(),
                        McpServerRequirement {
                            identity: McpServerIdentity::Command {
                                command: GOOD_CMD.to_string(),
                            },
                        },
                    ),
                    (
                        MISMATCHED_SERVER.to_string(),
                        McpServerRequirement {
                            identity: McpServerIdentity::Command {
                                command: GOOD_CMD.to_string(),
                            },
                        },
                    ),
                ])),
            },
        )]),
        source.clone(),
    );

    filter_plugin_mcp_servers_by_requirements("sample@test", &mut servers, Some(&requirements));

    let reason = Some(McpServerDisabledReason::Requirements { source });
    assert_eq!(
        servers
            .iter()
            .map(|(name, server)| (
                name.clone(),
                (server.enabled, server.disabled_reason.clone())
            ))
            .collect::<HashMap<String, (bool, Option<McpServerDisabledReason>)>>(),
        HashMap::from([
            (MATCHED_SERVER.to_string(), (true, None)),
            (MISMATCHED_SERVER.to_string(), (false, reason.clone())),
            (UNLISTED_SERVER.to_string(), (false, reason)),
        ])
    );
}

#[test]
fn filter_plugin_mcp_servers_by_allowlist_blocks_unlisted_plugin() {
    let mut servers = HashMap::from([("server-a".to_string(), stdio_mcp("cmd-a"))]);
    let source = RequirementSource::EnterpriseManaged {
        id: "cloud_requirements".to_string(),
        name: "Cloud requirements".to_string(),
    };
    let requirements = Sourced::new(
        BTreeMap::from([(
            "other@test".to_string(),
            codex_config::PluginRequirementsToml {
                mcp_servers: Some(BTreeMap::from([(
                    "server-a".to_string(),
                    McpServerRequirement {
                        identity: McpServerIdentity::Command {
                            command: "cmd-a".to_string(),
                        },
                    },
                )])),
            },
        )]),
        source.clone(),
    );

    filter_plugin_mcp_servers_by_requirements("sample@test", &mut servers, Some(&requirements));

    assert_eq!(
        servers
            .iter()
            .map(|(name, server)| (
                name.clone(),
                (server.enabled, server.disabled_reason.clone())
            ))
            .collect::<HashMap<String, (bool, Option<McpServerDisabledReason>)>>(),
        HashMap::from([(
            "server-a".to_string(),
            (
                false,
                Some(McpServerDisabledReason::Requirements { source })
            )
        )])
    );
}
