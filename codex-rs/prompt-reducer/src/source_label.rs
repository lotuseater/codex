const MAX_SOURCE_LABEL_CHARS: usize = 160;
const MAX_PATH_COMPONENTS: usize = 3;

pub(crate) fn compact_source_label(source: &str) -> String {
    let source = source.trim();
    let compact = compact_powershell_line_read(source)
        .unwrap_or_else(|| compact_absolute_windows_paths(source));
    truncate_source_label(&compact)
}

fn compact_powershell_line_read(source: &str) -> Option<String> {
    let command = source.strip_prefix("shell_output:")?;
    let (assignment, selector) = command.split_once(';')?;
    let assignment = assignment.trim();
    let selector = selector.trim();
    let path = assignment.strip_prefix("$lines = Get-Content ")?;
    if !selector.starts_with("$lines[") {
        return None;
    }

    Some(format!(
        "shell_output:Get-Content {}; {}",
        compact_absolute_windows_paths(path.trim()),
        selector
    ))
}

fn compact_absolute_windows_paths(text: &str) -> String {
    text.split_whitespace()
        .map(compact_path_like_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn compact_path_like_token(token: &str) -> String {
    let Some((prefix, core, suffix)) = split_windows_path_token(token) else {
        return token.to_string();
    };

    if let Some(tool_name) = compact_known_tool_path(core) {
        return format!("{prefix}{tool_name}{suffix}");
    }

    let separator = if core.contains('\\') { '\\' } else { '/' };
    let components = core.split(separator).collect::<Vec<_>>();
    if components.len() <= MAX_PATH_COMPONENTS + 1 {
        return token.to_string();
    }

    let drive = components.first().copied().unwrap_or_default();
    let separator = separator.to_string();
    let tail = components[components.len() - MAX_PATH_COMPONENTS..].join(&separator);
    format!("{prefix}{drive}{separator}...{separator}{tail}{suffix}")
}

fn split_windows_path_token(token: &str) -> Option<(&str, &str, &str)> {
    let start = token.char_indices().find_map(|(index, ch)| {
        if !ch.is_ascii_alphabetic() {
            return None;
        }
        let rest = &token[index..];
        let bytes = rest.as_bytes();
        (bytes.len() >= 3 && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/'))
            .then_some(index)
    })?;
    let end = token[start..]
        .char_indices()
        .find_map(|(offset, ch)| {
            let is_path_body_char = ch.is_ascii_alphanumeric()
                || matches!(
                    ch,
                    '\\' | '/' | ':' | '.' | '_' | '-' | '+' | '=' | '$' | '~' | '#' | '@'
                );
            (!is_path_body_char).then_some(start + offset)
        })
        .unwrap_or(token.len());
    let core = &token[start..end];
    if !looks_like_absolute_windows_path(core) {
        return None;
    }
    Some((&token[..start], core, &token[end..]))
}

fn compact_known_tool_path(path: &str) -> Option<&'static str> {
    let file_name = path.rsplit(['\\', '/']).next()?;
    if file_name.eq_ignore_ascii_case("powershell.exe") {
        return Some("powershell.exe");
    }
    if file_name.eq_ignore_ascii_case("pwsh.exe") {
        return Some("pwsh.exe");
    }
    None
}

fn looks_like_absolute_windows_path(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn truncate_source_label(text: &str) -> String {
    let char_count = text.chars().count();
    if char_count <= MAX_SOURCE_LABEL_CHARS {
        return text.to_string();
    }

    let head_len = MAX_SOURCE_LABEL_CHARS.saturating_sub(1);
    let head = text.chars().take(head_len).collect::<String>();
    format!("{head}...")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compacts_powershell_line_reads() {
        let source = concat!(
            "shell_output:$lines = Get-Content ",
            "C:\\Users\\Oleh\\Documents\\GitHub\\open_ai\\codex\\codex-rs\\prompt-reducer\\src\\lib.rs",
            "; $lines[686..715]"
        );

        assert_eq!(
            compact_source_label(source),
            "shell_output:Get-Content C:\\...\\prompt-reducer\\src\\lib.rs; $lines[686..715]"
        );
    }

    #[test]
    fn compacts_absolute_paths_in_generic_shell_sources() {
        let source = concat!(
            "shell_output:Get-Content -Path ",
            "C:\\Users\\Oleh\\Documents\\GitHub\\open_ai\\codex\\codex-rs\\prompt-reducer\\src\\lib.rs ",
            "-TotalCount 220"
        );

        assert_eq!(
            compact_source_label(source),
            "shell_output:Get-Content -Path C:\\...\\prompt-reducer\\src\\lib.rs -TotalCount 220"
        );
    }

    #[test]
    fn compacts_forward_slash_absolute_paths() {
        let source = concat!(
            "shell_output:rg prompt ",
            "C:/Users/Oleh/.codex/sessions/2026/05/22/rollout.jsonl"
        );

        assert_eq!(
            compact_source_label(source),
            "shell_output:rg prompt C:/.../05/22/rollout.jsonl"
        );
    }

    #[test]
    fn compacts_known_powershell_tool_paths_to_executable_name() {
        let source = concat!(
            "shell_output:\"C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe\" ",
            "-NoProfile -File scripts\\test-local-codex-release.ps1"
        );

        assert_eq!(
            compact_source_label(source),
            "shell_output:\"powershell.exe\" -NoProfile -File scripts\\test-local-codex-release.ps1"
        );
    }

    #[test]
    fn leaves_short_non_path_sources_unchanged() {
        assert_eq!(
            compact_source_label("message:assistant"),
            "message:assistant"
        );
    }
}
