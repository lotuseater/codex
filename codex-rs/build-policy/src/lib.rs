#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugCargoInvocation {
    pub subcommand: String,
}

pub const CODEX_REPO_DEBUG_CARGO_REJECTION: &str = "Direct debug-profile Cargo build/test/check/doc commands are blocked in this Codex checkout because they repeatedly exhaust local disk and memory. Use scripts\\test-local-codex-release.ps1, scripts\\build-local-codex.ps1 -Mode FastRelease or -Mode LowMemRelease, or add --release to a direct Cargo command when a direct command is genuinely needed.";

const SAFE_CARGO_SUBCOMMANDS: &[&str] = &[
    "add",
    "clean",
    "fetch",
    "fmt",
    "generate-lockfile",
    "help",
    "locate-project",
    "login",
    "logout",
    "metadata",
    "owner",
    "package",
    "pkgid",
    "publish",
    "remove",
    "rm",
    "search",
    "tree",
    "update",
    "vendor",
    "verify-project",
    "version",
    "yank",
];

pub fn debug_cargo_invocation(command: &str) -> Option<DebugCargoInvocation> {
    debug_cargo_invocation_inner(command, 0)
}

fn debug_cargo_invocation_inner(command: &str, depth: usize) -> Option<DebugCargoInvocation> {
    if depth > 4 {
        return None;
    }

    for segment in split_shell_segments(command) {
        let tokens = tokenize_shell_segment(&segment);
        if let Some(invocation) = debug_cargo_invocation_from_tokens(&tokens, depth) {
            return Some(invocation);
        }
    }
    None
}

fn debug_cargo_invocation_from_tokens(
    tokens: &[String],
    depth: usize,
) -> Option<DebugCargoInvocation> {
    let start = command_start(tokens)?;
    let executable = executable_name(&tokens[start]);

    if is_shell_executable(&executable)
        && let Some(inner) = shell_command_argument(tokens, start)
    {
        return debug_cargo_invocation_inner(&inner, depth + 1);
    }

    if executable == "cmd"
        && let Some(inner) = cmd_command_argument(tokens, start)
    {
        return debug_cargo_invocation_inner(&inner, depth + 1);
    }

    let cargo_index = if executable == "cargo" {
        Some(start)
    } else if executable == "rustup" {
        rustup_cargo_index(tokens, start)
    } else {
        None
    }?;

    debug_cargo_subcommand(tokens, cargo_index).map(|subcommand| DebugCargoInvocation {
        subcommand: subcommand.to_string(),
    })
}

fn command_start(tokens: &[String]) -> Option<usize> {
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index].trim();
        if token.is_empty() || token == "&" || token.eq_ignore_ascii_case("command") {
            index += 1;
            continue;
        }
        if looks_like_env_assignment(token) {
            index += 1;
            continue;
        }
        return Some(index);
    }
    None
}

fn looks_like_env_assignment(token: &str) -> bool {
    let Some((name, _)) = token.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn executable_name(token: &str) -> String {
    let normalized = token
        .trim_matches('"')
        .trim_matches('\'')
        .replace('\\', "/")
        .to_ascii_lowercase();
    let basename = normalized.rsplit('/').next().unwrap_or(&normalized);
    basename
        .strip_suffix(".exe")
        .unwrap_or(basename)
        .to_string()
}

fn is_shell_executable(executable: &str) -> bool {
    matches!(executable, "powershell" | "pwsh" | "pwsh-preview")
}

fn shell_command_argument(tokens: &[String], start: usize) -> Option<String> {
    let mut index = start + 1;
    while index < tokens.len() {
        let token = tokens[index].to_ascii_lowercase();
        if matches!(token.as_str(), "-command" | "-c") {
            let command = tokens[index + 1..].join(" ");
            return (!command.is_empty()).then_some(command);
        }
        index += 1;
    }
    None
}

fn cmd_command_argument(tokens: &[String], start: usize) -> Option<String> {
    let mut index = start + 1;
    while index < tokens.len() {
        if tokens[index].eq_ignore_ascii_case("/c") {
            return Some(tokens[index + 1..].join(" "));
        }
        index += 1;
    }
    None
}

fn rustup_cargo_index(tokens: &[String], start: usize) -> Option<usize> {
    if !tokens
        .get(start + 1)
        .is_some_and(|token| token.eq_ignore_ascii_case("run"))
    {
        return None;
    }
    let cargo_index = start + 3;
    tokens
        .get(cargo_index)
        .filter(|token| executable_name(token) == "cargo")
        .map(|_| cargo_index)
}

fn debug_cargo_subcommand(tokens: &[String], cargo_index: usize) -> Option<&str> {
    let mut index = cargo_index + 1;
    if tokens
        .get(index)
        .is_some_and(|token| token.starts_with('+') && token.len() > 1)
    {
        index += 1;
    }

    while index < tokens.len() {
        let token = tokens[index].to_ascii_lowercase();
        if !token.starts_with('-') {
            let rest = &tokens[index + 1..];
            if SAFE_CARGO_SUBCOMMANDS
                .iter()
                .any(|subcommand| token == *subcommand)
            {
                return None;
            }
            return (!uses_release_profile(rest, tokens[index].as_str()))
                .then_some(tokens[index].as_str());
        }
        index += cargo_global_option_width(&token);
    }
    None
}

fn cargo_global_option_width(token: &str) -> usize {
    match token {
        "--config" | "-Z" | "--manifest-path" | "--color" | "--target-dir" => 2,
        _ => 1,
    }
}

fn uses_release_profile(tokens: &[String], cargo_subcommand: &str) -> bool {
    let cargo_profile_flag_applies = matches!(
        cargo_subcommand,
        "bench" | "build" | "check" | "clippy" | "doc" | "run" | "test"
    );
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index].to_ascii_lowercase();
        if token == "--release" {
            return true;
        }
        if token == "--profile" {
            return cargo_profile_flag_applies
                && tokens
                    .get(index + 1)
                    .is_some_and(|profile| profile.eq_ignore_ascii_case("release"));
        }
        if let Some(profile) = token.strip_prefix("--profile=") {
            return cargo_profile_flag_applies && profile.eq_ignore_ascii_case("release");
        }
        if token == "--cargo-profile" {
            return tokens
                .get(index + 1)
                .is_some_and(|profile| profile.eq_ignore_ascii_case("release"));
        }
        if let Some(profile) = token.strip_prefix("--cargo-profile=") {
            return profile.eq_ignore_ascii_case("release");
        }
        index += 1;
    }
    false
}

fn split_shell_segments(command: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut chars = command.chars().peekable();
    while let Some(ch) = chars.next() {
        match quote {
            Some(active) if ch == active => {
                quote = None;
                current.push(ch);
            }
            Some(_) => current.push(ch),
            None if ch == '\'' || ch == '"' => {
                quote = Some(ch);
                current.push(ch);
            }
            None if matches!(ch, ';' | '\n' | '\r') => {
                push_segment(&mut segments, &mut current);
            }
            None if ch == '&' || ch == '|' => {
                if chars.peek().is_some_and(|next| *next == ch) {
                    let _ = chars.next();
                }
                push_segment(&mut segments, &mut current);
            }
            None => current.push(ch),
        }
    }
    push_segment(&mut segments, &mut current);
    segments
}

fn push_segment(segments: &mut Vec<String>, current: &mut String) {
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        segments.push(trimmed.to_string());
    }
    current.clear();
}

fn tokenize_shell_segment(segment: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut chars = segment.chars().peekable();

    while let Some(ch) = chars.next() {
        match quote {
            Some(active) if ch == active => {
                quote = None;
            }
            Some(_) => current.push(ch),
            None if ch == '\'' || ch == '"' => {
                quote = Some(ch);
            }
            None if ch.is_whitespace() => {
                push_token(&mut tokens, &mut current);
            }
            None if ch == '&' && current.is_empty() => {
                tokens.push("&".to_string());
            }
            None if ch == '`' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            None => current.push(ch),
        }
    }
    push_token(&mut tokens, &mut current);
    tokens
}

fn push_token(tokens: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        tokens.push(std::mem::take(current));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocked(command: &str) -> bool {
        debug_cargo_invocation(command).is_some()
    }

    #[test]
    fn blocks_direct_debug_cargo_work() {
        assert!(blocked("cargo test -p codex-agent-policy"));
        assert!(blocked("cargo check"));
        assert!(blocked("cargo build && echo ok"));
        assert!(blocked("& cargo.exe clippy -p codex-core"));
        assert!(blocked("cargo doc -p codex-core"));
        assert!(blocked("cargo nextest run -p codex-core"));
        assert!(blocked("cargo nextest run -p codex-core --profile release"));
        assert!(blocked("cargo llvm-cov nextest"));
        assert!(blocked("cargo +1.93.0 test -p codex-tools"));
        assert!(blocked("rustup run stable cargo test -p codex-core"));
    }

    #[test]
    fn allows_release_cargo_work_and_non_build_commands() {
        assert!(!blocked("cargo test -p codex-core --release"));
        assert!(!blocked("cargo build -p codex-cli --release --bin codex"));
        assert!(!blocked("cargo test -p codex-core --profile release"));
        assert!(!blocked("cargo nextest run -p codex-core --release"));
        assert!(!blocked("cargo llvm-cov nextest --cargo-profile release"));
        assert!(!blocked("cargo metadata --no-deps --format-version 1"));
        assert!(!blocked("cargo fmt -p codex-core"));
        assert!(!blocked("cargo tree -p codex-core"));
        assert!(!blocked(
            "powershell -File scripts\\test-local-codex-release.ps1 -Package codex-core"
        ));
    }

    #[test]
    fn does_not_block_searching_for_cargo_text() {
        assert!(!blocked("rg -n \"cargo test\" codex-rs/core/src"));
        assert!(!blocked(
            "Select-String -Path scripts\\*.ps1 -Pattern 'cargo build'"
        ));
    }

    #[test]
    fn blocks_nested_shell_debug_commands() {
        assert!(blocked("pwsh -Command \"cargo test -p codex-core\""));
        assert!(blocked("pwsh -Command cargo test -p codex-core"));
        assert!(blocked("powershell -c cargo check -p codex-core"));
        assert!(blocked("cmd /c cargo test -p codex-core"));
    }
}
