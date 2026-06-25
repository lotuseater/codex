use codex_connectors::AppInfo;

pub(crate) fn connector_display_label(connector: &AppInfo) -> String {
    connector.name.clone()
}

pub(crate) fn connector_mention_slug(connector: &AppInfo) -> String {
    connector_name_slug(&connector_display_label(connector))
}

fn connector_name_slug(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
        } else {
            normalized.push('-');
        }
    }
    let normalized = normalized.trim_matches('-');
    if normalized.is_empty() {
        "app".to_string()
    } else {
        normalized.to_string()
    }
}
