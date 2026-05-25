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
    let statements = split_powershell_statements(command);
    let mut string_assignments = Vec::<(String, String)>::new();

    for (index, statement) in statements.iter().enumerate() {
        if let Some((name, value)) = powershell_string_assignment(statement) {
            string_assignments.push((name, value));
            continue;
        }

        let Some(path_arg) = powershell_get_content_path_arg(statement) else {
            continue;
        };
        let assigned_get_content = statement
            .split_once('=')
            .map(|(name, command)| {
                normalize_powershell_variable(name.trim()).is_some()
                    && command.trim_start().starts_with("Get-Content")
            })
            .unwrap_or(false);
        if !path_arg.trim().starts_with('$') && !assigned_get_content {
            continue;
        }
        let path = resolve_powershell_path_arg(path_arg, &string_assignments)?;
        let selector = statements
            .iter()
            .skip(index + 1)
            .find(|statement| powershell_string_assignment(statement).is_none())
            .copied();

        let mut label = format!(
            "shell_output:Get-Content {}",
            compact_absolute_windows_paths(path.trim())
        );
        if let Some(selector) = selector {
            label.push_str("; ");
            label.push_str(&compact_absolute_windows_paths(selector));
        }
        return Some(label);
    }

    None
}

fn split_powershell_statements(command: &str) -> Vec<&str> {
    let mut statements = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut paren_depth = 0i32;
    let mut brace_depth = 0i32;
    let mut bracket_depth = 0i32;
    let mut escaped = false;

    for (index, ch) in command.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '`' {
            escaped = true;
            continue;
        }
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '(' => paren_depth += 1,
            ')' => paren_depth = (paren_depth - 1).max(0),
            '{' => brace_depth += 1,
            '}' => brace_depth = (brace_depth - 1).max(0),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = (bracket_depth - 1).max(0),
            ';' if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                let statement = command[start..index].trim();
                if !statement.is_empty() {
                    statements.push(statement);
                }
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }

    let statement = command[start..].trim();
    if !statement.is_empty() {
        statements.push(statement);
    }
    statements
}

fn powershell_string_assignment(statement: &str) -> Option<(String, String)> {
    let (name, value) = statement.split_once('=')?;
    let name = normalize_powershell_variable(name.trim())?;
    let value = unquote_powershell_string(value.trim())?;
    Some((name, value))
}

fn powershell_get_content_path_arg(statement: &str) -> Option<&str> {
    let command = statement
        .split_once('=')
        .map(|(_, value)| value.trim())
        .unwrap_or(statement.trim());
    let mut rest = command.strip_prefix("Get-Content")?.trim_start();
    while let Some((arg, next_rest)) = next_powershell_arg(rest) {
        if let Some(path_arg) = powershell_path_option_value(arg, next_rest) {
            return Some(path_arg);
        }

        if arg.starts_with('-') {
            if powershell_get_content_option_takes_value(arg) {
                rest = if powershell_option_has_inline_value(arg) {
                    next_rest
                } else {
                    next_powershell_arg(next_rest)
                        .map(|(_, after_value)| after_value)
                        .unwrap_or("")
                };
            } else {
                rest = next_rest;
            }
            continue;
        }

        return Some(arg);
    }

    None
}

fn first_powershell_arg(text: &str) -> Option<&str> {
    next_powershell_arg(text).map(|(arg, _)| arg)
}

fn next_powershell_arg(text: &str) -> Option<(&str, &str)> {
    let text = text.trim_start();
    let first = text.chars().next()?;
    if first == '\'' || first == '"' {
        let end = text[1..].find(first)? + 2;
        return Some((&text[..end], &text[end..]));
    }
    let end = text.find(char::is_whitespace).unwrap_or(text.len());
    Some((&text[..end], &text[end..]))
}

fn powershell_path_option_value<'a>(arg: &'a str, rest: &'a str) -> Option<&'a str> {
    for option in ["-LiteralPath", "-Path"] {
        if arg.eq_ignore_ascii_case(option) {
            return first_powershell_arg(rest);
        }
        if let Some(value) = powershell_option_inline_value(arg, option) {
            return Some(value);
        }
    }
    None
}

fn powershell_option_inline_value<'a>(arg: &'a str, option: &str) -> Option<&'a str> {
    let suffix = arg.get(option.len()..)?;
    if arg[..option.len()].eq_ignore_ascii_case(option)
        && (suffix.starts_with(':') || suffix.starts_with('='))
        && suffix.len() > 1
    {
        return Some(&suffix[1..]);
    }
    None
}

fn powershell_option_has_inline_value(arg: &str) -> bool {
    arg.trim_start_matches('-')
        .find(|ch| ch == ':' || ch == '=')
        .is_some()
}

fn powershell_get_content_option_takes_value(arg: &str) -> bool {
    let Some(option_name) = arg
        .trim_start_matches('-')
        .split([':', '='])
        .next()
        .filter(|name| !name.is_empty())
    else {
        return false;
    };

    [
        "Credential",
        "Delimiter",
        "Encoding",
        "ErrorAction",
        "ErrorVariable",
        "Exclude",
        "Filter",
        "InformationAction",
        "InformationVariable",
        "Include",
        "OutBuffer",
        "OutVariable",
        "PipelineVariable",
        "ProgressAction",
        "ReadCount",
        "Stream",
        "Tail",
        "TotalCount",
        "WarningAction",
        "WarningVariable",
    ]
    .iter()
    .any(|known| option_name.eq_ignore_ascii_case(known))
}

fn resolve_powershell_path_arg(
    path_arg: &str,
    string_assignments: &[(String, String)],
) -> Option<String> {
    let path_arg = path_arg.trim();
    if let Some(name) = normalize_powershell_variable(path_arg) {
        return string_assignments
            .iter()
            .rev()
            .find(|(assignment_name, _)| assignment_name == &name)
            .map(|(_, value)| value.clone());
    }

    Some(unquote_powershell_string(path_arg).unwrap_or_else(|| path_arg.to_string()))
}

fn normalize_powershell_variable(text: &str) -> Option<String> {
    let name = text.trim().strip_prefix('$')?;
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    Some(name.to_ascii_lowercase())
}

fn unquote_powershell_string(text: &str) -> Option<String> {
    let text = text.trim();
    let quote = text.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let rest = &text[1..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
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
    fn compacts_powershell_variable_line_reads_to_resolved_path() {
        let source = concat!(
            "shell_output:$p='C:\\Users\\Oleh\\Documents\\GitHub\\open_ai\\codex\\codex-rs\\prompt-reducer\\src\\source_label.rs'; ",
            "$lines=Get-Content $p; ",
            "1..80 | ForEach-Object { '{0}: {1}' -f $_,$lines[$_-1] }"
        );

        let label = compact_source_label(source);

        assert!(label.starts_with(
            "shell_output:Get-Content C:\\...\\prompt-reducer\\src\\source_label.rs; 1..80"
        ));
        assert!(!label.contains("$p="));
        assert!(!label.contains("Get-Content $p"));
    }

    #[test]
    fn compacts_powershell_variable_literal_path_reads() {
        let source = concat!(
            "shell_output:$Path = \"C:\\Users\\Oleh\\Documents\\GitHub\\open_ai\\codex\\codex-rs\\prompt-reducer\\src\\source_label.rs\"; ",
            "$lines = Get-Content -LiteralPath $Path; ",
            "$lines[41..100]"
        );

        assert_eq!(
            compact_source_label(source),
            "shell_output:Get-Content C:\\...\\prompt-reducer\\src\\source_label.rs; $lines[41..100]"
        );
    }

    #[test]
    fn compacts_powershell_variable_raw_reads() {
        let source = concat!(
            "shell_output:$p = 'C:\\Users\\Oleh\\Documents\\GitHub\\open_ai\\codex\\codex-rs\\prompt-reducer\\src\\source_label.rs'; ",
            "$content = Get-Content -Raw $p; ",
            "$content.Length"
        );

        assert_eq!(
            compact_source_label(source),
            "shell_output:Get-Content C:\\...\\prompt-reducer\\src\\source_label.rs; $content.Length"
        );
    }

    #[test]
    fn compacts_powershell_variable_reads_after_value_options() {
        let source = concat!(
            "shell_output:$p = 'C:\\Users\\Oleh\\Documents\\GitHub\\open_ai\\codex\\codex-rs\\prompt-reducer\\src\\source_label.rs'; ",
            "$lines = Get-Content -TotalCount 40 $p; ",
            "$lines[0..10]"
        );

        assert_eq!(
            compact_source_label(source),
            "shell_output:Get-Content C:\\...\\prompt-reducer\\src\\source_label.rs; $lines[0..10]"
        );
    }

    #[test]
    fn compacts_powershell_variable_for_loop_reads_without_truncating_selector() {
        let source = concat!(
            "shell_output:$p='C:\\Users\\Oleh\\Documents\\GitHub\\open_ai\\codex\\codex-rs\\prompt-reducer\\src\\source_label.rs'; ",
            "$lines=Get-Content $p; ",
            "for($i=1;$i -le 80;$i++){ '{0}: {1}' -f $i,$lines[$i-1] }"
        );

        let label = compact_source_label(source);

        assert!(label.starts_with(
            "shell_output:Get-Content C:\\...\\prompt-reducer\\src\\source_label.rs; for($i=1;$i -le 80;$i++)"
        ));
        assert!(!label.contains("$p="));
        assert!(!label.contains("Get-Content $p"));
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
