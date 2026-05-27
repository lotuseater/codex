use std::collections::BTreeMap;

use codex_network_proxy_config::InjectedHeaderConfig;
use codex_network_proxy_config::MitmHookActionsConfig;
use codex_network_proxy_config::MitmHookBodyConfig;
use codex_network_proxy_config::MitmHookConfig;
use codex_network_proxy_config::MitmHookMatchConfig;
use codex_network_proxy_config::NetworkDomainPermission as ProxyNetworkDomainPermission;
use codex_network_proxy_config::NetworkMode;
use codex_network_proxy_config::NetworkProxyConfig;
use codex_network_proxy_config::NetworkUnixSocketPermission as ProxyNetworkUnixSocketPermission;
use codex_network_proxy_config::normalize_host;
use codex_permission_types::FileSystemAccessMode;
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde::de;

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
pub struct PermissionsToml {
    #[serde(flatten)]
    pub entries: BTreeMap<String, PermissionProfileToml>,
}

impl PermissionsToml {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn resolve_profile(
        &self,
        profile_name: &str,
        built_in_profile: impl Fn(&str) -> Option<PermissionProfileToml>,
    ) -> Result<ResolvedPermissionProfileToml, ResolvePermissionProfileError> {
        if let Some(profile) = self.entries.get(profile_name) {
            return Ok(ResolvedPermissionProfileToml {
                profile: profile.clone(),
                inherited_profile_names: Vec::new(),
            });
        }

        if let Some(profile) = built_in_profile(profile_name) {
            return Ok(ResolvedPermissionProfileToml {
                profile,
                inherited_profile_names: vec![profile_name.to_string()],
            });
        }

        Err(ResolvePermissionProfileError {
            profile_name: profile_name.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPermissionProfileToml {
    pub profile: PermissionProfileToml,
    pub inherited_profile_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvePermissionProfileError {
    profile_name: String,
}

impl std::fmt::Display for ResolvePermissionProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "permission profile `{}` is not defined",
            self.profile_name
        )
    }
}

impl std::error::Error for ResolvePermissionProfileError {}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct PermissionProfileToml {
    pub description: Option<String>,
    pub workspace_roots: Option<WorkspaceRootsToml>,
    pub filesystem: Option<FilesystemPermissionsToml>,
    pub network: Option<NetworkToml>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
pub struct WorkspaceRootsToml {
    #[serde(flatten)]
    pub entries: BTreeMap<String, bool>,
}

impl WorkspaceRootsToml {
    pub fn enabled_roots(&self) -> impl Iterator<Item = &String> {
        self.entries
            .iter()
            .filter_map(|(path, enabled)| (*enabled).then_some(path))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
pub struct FilesystemPermissionsToml {
    /// Optional maximum depth for expanding unreadable glob patterns on
    /// platforms that snapshot glob matches before sandbox startup.
    #[schemars(range(min = 1))]
    pub glob_scan_max_depth: Option<usize>,
    #[serde(flatten)]
    pub entries: BTreeMap<String, FilesystemPermissionToml>,
}

impl FilesystemPermissionsToml {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(untagged)]
pub enum FilesystemPermissionToml {
    Access(FileSystemAccessMode),
    Scoped(BTreeMap<String, FileSystemAccessMode>),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
pub struct NetworkDomainPermissionsToml {
    #[serde(flatten)]
    pub entries: BTreeMap<String, NetworkDomainPermissionToml>,
}

impl NetworkDomainPermissionsToml {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn allowed_domains(&self) -> Option<Vec<String>> {
        let allowed_domains: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, permission)| matches!(permission, NetworkDomainPermissionToml::Allow))
            .map(|(pattern, _)| pattern.clone())
            .collect();
        (!allowed_domains.is_empty()).then_some(allowed_domains)
    }

    pub fn denied_domains(&self) -> Option<Vec<String>> {
        let denied_domains: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, permission)| matches!(permission, NetworkDomainPermissionToml::Deny))
            .map(|(pattern, _)| pattern.clone())
            .collect();
        (!denied_domains.is_empty()).then_some(denied_domains)
    }
}

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum NetworkDomainPermissionToml {
    Allow,
    Deny,
}

impl std::fmt::Display for NetworkDomainPermissionToml {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let permission = match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        };
        f.write_str(permission)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
pub struct NetworkUnixSocketPermissionsToml {
    #[serde(flatten)]
    pub entries: BTreeMap<String, NetworkUnixSocketPermissionToml>,
}

impl NetworkUnixSocketPermissionsToml {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn allow_unix_sockets(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|(_, permission)| matches!(permission, NetworkUnixSocketPermissionToml::Allow))
            .map(|(path, _)| path.clone())
            .collect()
    }
}

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum NetworkUnixSocketPermissionToml {
    Allow,
    None,
}

impl std::fmt::Display for NetworkUnixSocketPermissionToml {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let permission = match self {
            Self::Allow => "allow",
            Self::None => "none",
        };
        f.write_str(permission)
    }
}

#[derive(Serialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct NetworkMitmToml {
    #[schemars(with = "Option<BTreeMap<String, NetworkMitmHookToml>>")]
    pub hooks: Option<IndexMap<String, NetworkMitmHookToml>>,
    #[schemars(with = "Option<BTreeMap<String, NetworkMitmActionToml>>")]
    pub actions: Option<IndexMap<String, NetworkMitmActionToml>>,
}

impl<'de> Deserialize<'de> for NetworkMitmToml {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawNetworkMitmToml {
            hooks: Option<IndexMap<String, NetworkMitmHookToml>>,
            actions: Option<IndexMap<String, NetworkMitmActionToml>>,
        }

        let raw = RawNetworkMitmToml::deserialize(deserializer)?;
        let toml = Self {
            hooks: raw.hooks,
            actions: raw.actions,
        };
        toml.validate().map_err(de::Error::custom)?;
        Ok(toml)
    }
}

impl NetworkMitmToml {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(actions) = self.actions.as_ref() {
            for (name, action) in actions {
                if action.strip_request_headers.is_empty()
                    && action.inject_request_headers.is_empty()
                {
                    return Err(format!(
                        "network.mitm.actions.{name} must define at least one operation"
                    ));
                }
            }
        }

        if let Some(hooks) = self.hooks.as_ref() {
            for (name, hook) in hooks {
                if hook.action.is_empty() {
                    return Err(format!(
                        "network.mitm.hooks.{name}.action must not be empty"
                    ));
                }

                for action_name in &hook.action {
                    if !self
                        .actions
                        .as_ref()
                        .is_some_and(|actions| actions.contains_key(action_name))
                    {
                        return Err(format!(
                            "network.mitm.hooks.{name}.action references undefined action {action_name}"
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    pub fn to_mitm_hooks(&self) -> Vec<MitmHookConfig> {
        let Some(hooks) = self.hooks.as_ref() else {
            return Vec::new();
        };

        hooks
            .values()
            .map(|hook| MitmHookConfig {
                host: hook.host.clone(),
                matcher: MitmHookMatchConfig {
                    methods: hook.methods.clone(),
                    path_prefixes: hook.path_prefixes.clone(),
                    query: hook.query.clone(),
                    headers: hook.headers.clone(),
                    body: hook
                        .body
                        .as_ref()
                        .map(|body| MitmHookBodyConfig(body.0.clone())),
                },
                actions: self.actions_for_hook(hook),
            })
            .collect()
    }

    fn actions_for_hook(&self, hook: &NetworkMitmHookToml) -> MitmHookActionsConfig {
        let mut actions = MitmHookActionsConfig::default();
        let Some(definitions) = self.actions.as_ref() else {
            return actions;
        };

        for action_name in &hook.action {
            let Some(action) = definitions.get(action_name) else {
                continue;
            };

            actions
                .strip_request_headers
                .extend(action.strip_request_headers.iter().cloned());
            actions.inject_request_headers.extend(
                action
                    .inject_request_headers
                    .iter()
                    .map(InjectedHeaderConfig::from),
            );
        }

        actions
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
#[serde(default, deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct NetworkMitmHookToml {
    pub host: String,
    pub methods: Vec<String>,
    pub path_prefixes: Vec<String>,
    pub query: BTreeMap<String, Vec<String>>,
    pub headers: BTreeMap<String, Vec<String>>,
    pub body: Option<NetworkMitmHookBodyToml>,
    pub action: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
#[serde(default, deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct NetworkMitmActionToml {
    pub strip_request_headers: Vec<String>,
    pub inject_request_headers: Vec<NetworkMitmInjectedHeaderToml>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
#[serde(default, deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct NetworkMitmInjectedHeaderToml {
    pub name: String,
    pub secret_env_var: Option<String>,
    pub secret_file: Option<String>,
    pub prefix: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(transparent)]
pub struct NetworkMitmHookBodyToml(pub serde_json::Value);

impl From<&NetworkMitmInjectedHeaderToml> for InjectedHeaderConfig {
    fn from(value: &NetworkMitmInjectedHeaderToml) -> Self {
        Self {
            name: value.name.clone(),
            secret_env_var: value.secret_env_var.clone(),
            secret_file: value.secret_file.clone(),
            prefix: value.prefix.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct NetworkToml {
    pub enabled: Option<bool>,
    pub proxy_url: Option<String>,
    pub enable_socks5: Option<bool>,
    pub socks_url: Option<String>,
    pub enable_socks5_udp: Option<bool>,
    pub allow_upstream_proxy: Option<bool>,
    pub dangerously_allow_non_loopback_proxy: Option<bool>,
    pub dangerously_allow_all_unix_sockets: Option<bool>,
    #[schemars(with = "Option<NetworkModeSchema>")]
    pub mode: Option<NetworkMode>,
    pub domains: Option<NetworkDomainPermissionsToml>,
    pub unix_sockets: Option<NetworkUnixSocketPermissionsToml>,
    pub allow_local_binding: Option<bool>,
    pub mitm: Option<NetworkMitmToml>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum NetworkModeSchema {
    Limited,
    Full,
}

impl NetworkToml {
    pub fn apply_to_network_proxy_config(&self, config: &mut NetworkProxyConfig) {
        if let Some(enabled) = self.enabled {
            config.network.enabled = enabled;
        }
        if let Some(proxy_url) = self.proxy_url.as_ref() {
            config.network.proxy_url = proxy_url.clone();
        }
        if let Some(enable_socks5) = self.enable_socks5 {
            config.network.enable_socks5 = enable_socks5;
        }
        if let Some(socks_url) = self.socks_url.as_ref() {
            config.network.socks_url = socks_url.clone();
        }
        if let Some(enable_socks5_udp) = self.enable_socks5_udp {
            config.network.enable_socks5_udp = enable_socks5_udp;
        }
        if let Some(allow_upstream_proxy) = self.allow_upstream_proxy {
            config.network.allow_upstream_proxy = allow_upstream_proxy;
        }
        if let Some(dangerously_allow_non_loopback_proxy) =
            self.dangerously_allow_non_loopback_proxy
        {
            config.network.dangerously_allow_non_loopback_proxy =
                dangerously_allow_non_loopback_proxy;
        }
        if let Some(dangerously_allow_all_unix_sockets) = self.dangerously_allow_all_unix_sockets {
            config.network.dangerously_allow_all_unix_sockets = dangerously_allow_all_unix_sockets;
        }
        if let Some(mode) = self.mode {
            config.network.mode = mode;
        }
        if let Some(domains) = self.domains.as_ref() {
            overlay_network_domain_permissions(config, domains);
        }
        if let Some(unix_sockets) = self.unix_sockets.as_ref() {
            let mut proxy_unix_sockets = config.network.unix_sockets.take().unwrap_or_default();
            for (path, permission) in &unix_sockets.entries {
                let permission = match permission {
                    NetworkUnixSocketPermissionToml::Allow => {
                        ProxyNetworkUnixSocketPermission::Allow
                    }
                    NetworkUnixSocketPermissionToml::None => ProxyNetworkUnixSocketPermission::None,
                };
                proxy_unix_sockets.entries.insert(path.clone(), permission);
            }
            config.network.unix_sockets =
                (!proxy_unix_sockets.entries.is_empty()).then_some(proxy_unix_sockets);
        }
        if let Some(allow_local_binding) = self.allow_local_binding {
            config.network.allow_local_binding = allow_local_binding;
        }
        if let Some(mitm) = self.mitm.as_ref() {
            config.network.mitm = true;
            config.network.mitm_hooks = mitm.to_mitm_hooks();
        }
    }

    pub fn to_network_proxy_config(&self) -> NetworkProxyConfig {
        let mut config = NetworkProxyConfig::default();
        self.apply_to_network_proxy_config(&mut config);
        config
    }
}

pub fn overlay_network_domain_permissions(
    config: &mut NetworkProxyConfig,
    domains: &NetworkDomainPermissionsToml,
) {
    for (pattern, permission) in &domains.entries {
        let permission = match permission {
            NetworkDomainPermissionToml::Allow => ProxyNetworkDomainPermission::Allow,
            NetworkDomainPermissionToml::Deny => ProxyNetworkDomainPermission::Deny,
        };
        config
            .network
            .upsert_domain_permission(pattern.clone(), permission, normalize_host);
    }
}
