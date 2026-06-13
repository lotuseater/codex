use codex_tool_registry_api::ToolSpec;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub(crate) struct HostedNamespaceAliasPolicy {
    reserved_model_namespaces: BTreeSet<String>,
    alias_allocator: NamespaceAliasAllocator,
    aliases_by_source_namespace: BTreeMap<String, String>,
}

impl HostedNamespaceAliasPolicy {
    pub(crate) fn for_hosted_specs(
        hosted_specs: &[ToolSpec],
        occupied_model_tool_names: impl IntoIterator<Item = String>,
    ) -> Self {
        let reserved_model_namespaces = hosted_reserved_model_namespaces(hosted_specs);
        let mut occupied_model_tool_names_for_aliases = BTreeSet::new();
        if !reserved_model_namespaces.is_empty() {
            occupied_model_tool_names_for_aliases.extend(occupied_model_tool_names);
            occupied_model_tool_names_for_aliases
                .extend(hosted_specs.iter().map(|spec| spec.name().to_string()));
            occupied_model_tool_names_for_aliases.extend(reserved_model_namespaces.iter().cloned());
        }

        Self {
            reserved_model_namespaces,
            alias_allocator: NamespaceAliasAllocator::new(occupied_model_tool_names_for_aliases),
            aliases_by_source_namespace: BTreeMap::new(),
        }
    }

    pub(crate) fn has_reserved_namespaces(&self) -> bool {
        !self.reserved_model_namespaces.is_empty()
    }

    pub(crate) fn alias_for_source_namespace(&mut self, source_namespace: &str) -> Option<String> {
        if !self.reserved_model_namespaces.contains(source_namespace) {
            return None;
        }

        if let Some(alias) = self.aliases_by_source_namespace.get(source_namespace) {
            return Some(alias.clone());
        }

        let alias = self.alias_allocator.allocate(source_namespace);
        self.aliases_by_source_namespace
            .insert(source_namespace.to_string(), alias.clone());
        Some(alias)
    }
}

pub(crate) struct NamespaceAliasAllocator {
    occupied_model_tool_names: BTreeSet<String>,
}

impl NamespaceAliasAllocator {
    pub(crate) fn new(occupied_model_tool_names: BTreeSet<String>) -> Self {
        Self {
            occupied_model_tool_names,
        }
    }

    pub(crate) fn allocate(&mut self, source_namespace: &str) -> String {
        let sanitized = sanitize_namespace_for_alias(source_namespace);
        let base = format!("codex_ext_{sanitized}");
        if self.occupied_model_tool_names.insert(base.clone()) {
            return base;
        }

        for suffix in 2.. {
            let candidate = format!("{base}_{suffix}");
            if self.occupied_model_tool_names.insert(candidate.clone()) {
                return candidate;
            }
        }
        unreachable!("unbounded alias allocation should always return")
    }
}

fn hosted_reserved_model_namespaces(hosted_specs: &[ToolSpec]) -> BTreeSet<String> {
    let mut reserved_namespaces = BTreeSet::new();
    for spec in hosted_specs {
        match spec {
            // The Responses API exposes hosted web search through the
            // `web_search` tool type, but rejects user-defined namespace `web`
            // in the same request.
            ToolSpec::WebSearch { .. } => {
                reserved_namespaces.insert("web".to_string());
            }
            ToolSpec::Function(_)
            | ToolSpec::Namespace(_)
            | ToolSpec::ToolSearch { .. }
            | ToolSpec::LocalShell {}
            | ToolSpec::ImageGeneration { .. }
            | ToolSpec::Freeform(_) => {}
        }
    }
    reserved_namespaces
}

fn sanitize_namespace_for_alias(namespace: &str) -> String {
    let mut sanitized = namespace
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    while sanitized.contains("__") {
        sanitized = sanitized.replace("__", "_");
    }
    sanitized = sanitized.trim_matches('_').to_string();
    if sanitized.is_empty() {
        "namespace".to_string()
    } else {
        sanitized
    }
}
