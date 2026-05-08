use std::path::PathBuf;

use super::ReplacementCandidate;

const SHELL_CONTROL_MARKERS: &[&str] = &["\n", "\r", "&&", "||", "|", ";", ">", "<", "$("];

pub(super) fn classify_shell_replacement(command: &str) -> Option<ReplacementCandidate> {
    if has_shell_control(command) {
        return classify_shell_control_shadow(command);
    }
    let tokens = shell_tokens(command)?;
    classify_promoted_git_diff_candidate(&tokens)
        .or_else(|| classify_git_candidate(&tokens))
        .or_else(|| classify_rg_files_candidate(&tokens))
        .or_else(|| classify_rg_candidate(&tokens))
        .or_else(|| classify_file_outline_candidate(&tokens))
        .or_else(|| classify_file_excerpt_digest_candidate(&tokens))
        .or_else(|| classify_select_string_digest_candidate(&tokens))
        .or_else(|| classify_generic_search_digest_candidate(&tokens))
        .or_else(|| classify_run_check_digest_candidate(&tokens))
        .or_else(|| classify_rg_expansion_candidate(&tokens))
        .or_else(|| classify_file_inventory_candidate(&tokens))
        .or_else(|| classify_directory_listing_candidate(&tokens))
        .or_else(|| classify_process_table_candidate(&tokens))
}

fn classify_shell_control_shadow(command: &str) -> Option<ReplacementCandidate> {
    classify_git_filtered_diff_shell_control(command)
        .or_else(|| classify_file_excerpt_shell_control(command))
        .or_else(|| classify_rg_file_set_shell_control(command))
        .or_else(|| classify_file_inventory_shell_control(command))
        .or_else(|| classify_run_check_shell_control(command))
        .or_else(|| classify_select_string_shell_control(command))
        .or_else(|| classify_directory_listing_shell_control(command))
        .or_else(|| {
            command_looks_like_check(command).then_some(ReplacementCandidate::RunCheckDigest)
        })
}

pub(super) fn classify_promoted_replacement(command: &str) -> Option<ReplacementCandidate> {
    if has_shell_control(command) {
        return None;
    }
    let tokens = shell_tokens(command)?;
    classify_promoted_git_diff_candidate(&tokens)
}

fn has_shell_control(command: &str) -> bool {
    SHELL_CONTROL_MARKERS
        .iter()
        .any(|marker| command.contains(marker))
}

fn shell_tokens(command: &str) -> Option<Vec<String>> {
    let tokens = if command.contains('\\') && !command.contains('"') && !command.contains('\'') {
        command
            .split_whitespace()
            .map(ToString::to_string)
            .collect()
    } else {
        shlex::split(command).unwrap_or_else(|| {
            command
                .split_whitespace()
                .map(ToString::to_string)
                .collect()
        })
    };
    (!tokens.is_empty()).then_some(tokens)
}

fn classify_git_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    if tokens.first()? != "git" {
        return None;
    }

    let mut index = 1;
    if tokens.get(index).is_some_and(|token| token == "-C") {
        tokens.get(index + 1)?;
        index += 2;
    }

    match tokens.get(index).map(String::as_str)? {
        "diff" if git_diff_stat_args_are_shadowable(&tokens[index + 1..]) => {
            Some(ReplacementCandidate::GitDiffStatCompact)
        }
        "diff" if git_diff_name_only_args_are_shadowable(&tokens[index + 1..]) => {
            Some(ReplacementCandidate::GitChangedFiles)
        }
        "diff" if git_diff_name_status_args_are_shadowable(&tokens[index + 1..]) => {
            Some(ReplacementCandidate::GitNameStatusCompact)
        }
        "diff" if git_diff_numstat_args_are_shadowable(&tokens[index + 1..]) => {
            Some(ReplacementCandidate::GitNumstatCompact)
        }
        "diff" if git_diff_hunk_args_are_shadowable(&tokens[index + 1..]) => {
            Some(ReplacementCandidate::DiffHunkSummary)
        }
        "show" | "log" => Some(ReplacementCandidate::GitHistoryDigest),
        "ls-files" => Some(ReplacementCandidate::DirectoryListingCompact),
        _ => None,
    }
}

fn classify_promoted_git_diff_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    if tokens.first()? != "git" {
        return None;
    }

    let mut index = 1;
    if tokens.get(index).is_some_and(|token| token == "-C") {
        tokens.get(index + 1)?;
        index += 2;
    }

    match tokens.get(index).map(String::as_str)? {
        "diff" if git_diff_stat_args_are_replaceable(&tokens[index + 1..]) => {
            Some(ReplacementCandidate::GitDiffStatCompact)
        }
        _ => None,
    }
}

fn git_diff_stat_args_are_shadowable(args: &[String]) -> bool {
    !args.is_empty()
        && args.iter().all(|arg| {
            matches!(
                arg.as_str(),
                "--stat" | "--shortstat" | "--cached" | "--staged" | "--" | "."
            )
        })
        && args
            .iter()
            .any(|arg| matches!(arg.as_str(), "--stat" | "--shortstat"))
}

fn git_diff_name_only_args_are_shadowable(args: &[String]) -> bool {
    !args.is_empty()
        && args.iter().all(|arg| {
            matches!(
                arg.as_str(),
                "--name-only" | "--cached" | "--staged" | "--" | "."
            ) || !arg.starts_with('-')
        })
        && args.iter().any(|arg| arg == "--name-only")
}

fn git_diff_hunk_args_are_shadowable(args: &[String]) -> bool {
    args.iter().all(|arg| {
        matches!(arg.as_str(), "--cached" | "--staged" | "--" | ".") || !arg.starts_with('-')
    }) && !args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--stat" | "--shortstat" | "--name-only" | "--name-status" | "--numstat" | "--check"
        )
    })
}

fn git_diff_name_status_args_are_shadowable(args: &[String]) -> bool {
    !args.is_empty()
        && args.iter().all(|arg| {
            matches!(
                arg.as_str(),
                "--name-status" | "--cached" | "--staged" | "--" | "."
            ) || !arg.starts_with('-')
        })
        && args.iter().any(|arg| arg == "--name-status")
}

fn git_diff_numstat_args_are_shadowable(args: &[String]) -> bool {
    !args.is_empty()
        && args.iter().all(|arg| {
            matches!(
                arg.as_str(),
                "--numstat" | "--cached" | "--staged" | "--" | "."
            ) || !arg.starts_with('-')
        })
        && args.iter().any(|arg| arg == "--numstat")
}

fn git_diff_stat_args_are_replaceable(args: &[String]) -> bool {
    !args.is_empty()
        && args.iter().all(|arg| {
            matches!(
                arg.as_str(),
                "--stat" | "--shortstat" | "--cached" | "--staged"
            )
        })
        && args
            .iter()
            .any(|arg| matches!(arg.as_str(), "--stat" | "--shortstat"))
}

fn classify_rg_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    if tokens.first()? != "rg" {
        return None;
    }

    let mut pattern = None;
    let mut paths = Vec::new();
    let mut globs = Vec::new();
    let mut index = 1;
    while let Some(token) = tokens.get(index) {
        if token == "--" {
            index += 1;
            pattern = tokens.get(index).cloned();
            paths.extend(tokens.iter().skip(index + 1).cloned());
            break;
        }
        if token.starts_with('-') {
            match parse_rg_option(tokens, &mut index, &mut pattern, &mut globs) {
                RgOptionParse::Continue => {}
                RgOptionParse::Reject => return None,
            }
        } else if pattern.is_none() {
            pattern = Some(token.clone());
        } else {
            paths.push(token.clone());
        }
        index += 1;
    }

    pattern.and_then(|pattern| {
        (!pattern.trim().is_empty()).then_some(ReplacementCandidate::SearchText {
            pattern,
            globs,
            paths,
        })
    })
}

enum RgOptionParse {
    Continue,
    Reject,
}

fn parse_rg_option(
    tokens: &[String],
    index: &mut usize,
    pattern: &mut Option<String>,
    globs: &mut Vec<String>,
) -> RgOptionParse {
    let Some(token) = tokens.get(*index) else {
        return RgOptionParse::Reject;
    };
    match token.as_str() {
        "--files"
        | "-l"
        | "--files-with-matches"
        | "--files-without-match"
        | "--count"
        | "--count-matches"
        | "--replace"
        | "--json"
        | "-A"
        | "-B"
        | "-C"
        | "-i"
        | "-F"
        | "-t"
        | "-T"
        | "--after-context"
        | "--before-context"
        | "--context"
        | "--fixed-strings"
        | "--hidden"
        | "--ignore-case"
        | "--type"
        | "--type-not" => RgOptionParse::Reject,
        "-g" | "--glob" => {
            *index += 1;
            if let Some(value) = tokens.get(*index) {
                globs.push(value.clone());
                RgOptionParse::Continue
            } else {
                RgOptionParse::Reject
            }
        }
        "-e" | "--regexp" => {
            *index += 1;
            if let Some(value) = tokens.get(*index) {
                *pattern = Some(value.clone());
                RgOptionParse::Continue
            } else {
                RgOptionParse::Reject
            }
        }
        "-m" | "--max-count" | "--max-columns" | "--color" => {
            *index += 1;
            if tokens.get(*index).is_some() {
                RgOptionParse::Continue
            } else {
                RgOptionParse::Reject
            }
        }
        "-n" | "--line-number" | "--column" | "--heading" | "--no-heading" | "--with-filename" => {
            RgOptionParse::Continue
        }
        _ if token.starts_with("--glob=") => {
            if let Some(value) = token.strip_prefix("--glob=") {
                globs.push(value.to_string());
            }
            RgOptionParse::Continue
        }
        _ if token.starts_with("--max-count=")
            || token.starts_with("--max-columns=")
            || token.starts_with("--color=") =>
        {
            RgOptionParse::Continue
        }
        _ => RgOptionParse::Reject,
    }
}

fn classify_rg_files_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    if tokens.first()? != "rg" || !tokens.iter().any(|token| token == "--files") {
        return None;
    }

    let mut index = 1;
    while let Some(token) = tokens.get(index) {
        match token.as_str() {
            "--files" | "--no-ignore" | "--hidden" => {}
            "-g" | "--glob" => {
                index += 1;
                tokens.get(index)?;
            }
            _ if token.starts_with("--glob=") => {}
            _ if token.starts_with('-') => return None,
            _ => {}
        }
        index += 1;
    }
    Some(ReplacementCandidate::RgFilesCompact)
}

fn classify_file_outline_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    let command = tokens.first()?.to_ascii_lowercase();
    if !matches!(
        command.as_str(),
        "cat" | "type" | "gc" | "get-content" | "get-content.exe"
    ) {
        return None;
    }

    let mut path = None;
    let mut index = 1;
    while let Some(token) = tokens.get(index) {
        let lower = token.to_ascii_lowercase();
        match lower.as_str() {
            "-tail" | "-totalcount" | "-first" | "-last" | "-head" => return None,
            "-path" | "-literalpath" => {
                index += 1;
                path = tokens.get(index).cloned();
            }
            _ if token.starts_with('-') => {}
            _ if path.is_none() => path = Some(token.clone()),
            _ => return None,
        }
        index += 1;
    }

    path.and_then(|path| {
        (!path.contains('*') && !path.contains('?') && !path.contains('[')).then_some(
            ReplacementCandidate::FileOutline {
                path: PathBuf::from(path),
            },
        )
    })
}

fn classify_run_check_digest_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    let command = tokens.first()?.to_ascii_lowercase();
    if command_allows_embedded_check_scan(&command) && command_looks_like_check(&tokens.join(" ")) {
        return Some(ReplacementCandidate::RunCheckDigest);
    }

    let is_check = match command.as_str() {
        "cargo" => tokens.get(1).is_some_and(|subcommand| {
            matches!(
                subcommand.as_str(),
                "test" | "check" | "build" | "clippy" | "fmt"
            )
        }),
        "git" => tokens
            .windows(2)
            .any(|pair| pair[0] == "diff" && pair[1] == "--check"),
        "just" => tokens.get(1).is_some_and(|recipe| {
            matches!(
                recipe.as_str(),
                "fmt" | "fix" | "test" | "nextest" | "argument-comment-lint"
            )
        }),
        "pytest" => true,
        "python" | "python.exe" | "py" => tokens
            .iter()
            .any(|token| matches!(token.as_str(), "test" | "pytest" | "py_compile")),
        "npm" | "pnpm" | "yarn" => tokens.iter().any(|token| {
            matches!(
                token.as_str(),
                "test" | "build" | "lint" | "typecheck" | "check"
            )
        }),
        "go" => tokens
            .get(1)
            .is_some_and(|subcommand| matches!(subcommand.as_str(), "test" | "build" | "vet")),
        "dotnet" => tokens
            .get(1)
            .is_some_and(|subcommand| matches!(subcommand.as_str(), "test" | "build")),
        "mvn" | "mvn.cmd" | "mvnw" | "mvnw.cmd" => tokens.iter().any(|token| {
            matches!(
                token.as_str(),
                "test" | "verify" | "package" | "install" | "compile"
            )
        }),
        "gradle" | "gradle.bat" | "gradlew" | "gradlew.bat" => tokens
            .iter()
            .any(|token| matches!(token.as_str(), "test" | "build" | "check" | "assemble")),
        "make" | "mingw32-make" | "nmake" => tokens
            .iter()
            .any(|token| matches!(token.as_str(), "test" | "check" | "build" | "all")),
        "cmake" => tokens.iter().any(|token| token == "--build"),
        "ctest" | "ninja" | "msbuild" | "msbuild.exe" | "xcodebuild" => true,
        "invoke-bounded" => {
            let has_build_tool = tokens.iter().any(|token| {
                let lower = token.to_ascii_lowercase();
                matches!(
                    lower.as_str(),
                    "cargo"
                        | "cmake"
                        | "ctest"
                        | "npm"
                        | "pnpm"
                        | "yarn"
                        | "go"
                        | "dotnet"
                        | "mvn"
                        | "gradle"
                        | "gradlew"
                        | "make"
                        | "ninja"
                        | "msbuild"
                        | "xcodebuild"
                )
            });
            let has_check_arg = tokens.iter().any(|token| {
                token.contains("--build")
                    || matches!(
                        token.to_ascii_lowercase().as_str(),
                        "test" | "check" | "build" | "lint" | "verify"
                    )
            });
            has_build_tool && has_check_arg
        }
        "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe" => tokens
            .iter()
            .any(|token| token.ends_with("build-local-codex.ps1")),
        _ => false,
    };
    is_check.then_some(ReplacementCandidate::RunCheckDigest)
}

fn command_allows_embedded_check_scan(command: &str) -> bool {
    matches!(
        command,
        "powershell"
            | "powershell.exe"
            | "pwsh"
            | "pwsh.exe"
            | "cmd"
            | "cmd.exe"
            | "invoke-bounded"
            | "start-process"
    )
}

fn classify_file_excerpt_digest_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    let command = tokens.first()?.to_ascii_lowercase();
    match command.as_str() {
        "head" | "tail" => Some(ReplacementCandidate::FileExcerptDigest),
        "gc" | "get-content" | "get-content.exe" => tokens
            .iter()
            .skip(1)
            .map(|token| token.to_ascii_lowercase())
            .any(|token| {
                matches!(
                    token.as_str(),
                    "-tail" | "-totalcount" | "-first" | "-last" | "-head"
                ) || token.starts_with("-tail:")
                    || token.starts_with("-totalcount:")
                    || token.starts_with("-first:")
                    || token.starts_with("-last:")
            })
            .then_some(ReplacementCandidate::FileExcerptDigest),
        _ => None,
    }
}

fn classify_select_string_digest_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    let command = tokens.first()?.to_ascii_lowercase();
    matches!(
        command.as_str(),
        "select-string" | "select-string.exe" | "sls"
    )
    .then_some(ReplacementCandidate::SelectStringDigest)
}

fn classify_generic_search_digest_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    let command = tokens.first()?.to_ascii_lowercase();
    if matches!(
        command.as_str(),
        "grep" | "grep.exe" | "findstr" | "findstr.exe"
    ) {
        return Some(ReplacementCandidate::SelectStringDigest);
    }
    (command == "git" && tokens.get(1).is_some_and(|subcommand| subcommand == "grep"))
        .then_some(ReplacementCandidate::SelectStringDigest)
}

fn classify_rg_expansion_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    if tokens.first()? != "rg" {
        return None;
    }
    if tokens
        .iter()
        .any(|token| token == "--json" || token.starts_with("--json="))
    {
        return Some(ReplacementCandidate::RgJsonDigest);
    }
    if tokens.iter().any(|token| {
        matches!(token.as_str(), "-c" | "--count" | "--count-matches")
            || token.starts_with("--count=")
            || token.starts_with("--count-matches=")
    }) {
        return Some(ReplacementCandidate::RgCountDigest);
    }
    tokens
        .iter()
        .any(|token| {
            matches!(
                token.as_str(),
                "-l" | "--files-with-matches" | "--files-without-match"
            )
        })
        .then_some(ReplacementCandidate::RgFileSetDigest)
}

fn classify_directory_listing_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    let command = tokens.first()?.to_ascii_lowercase();
    matches!(
        command.as_str(),
        "get-childitem" | "get-childitem.exe" | "gci" | "ls" | "dir"
    )
    .then_some(ReplacementCandidate::DirectoryListingCompact)
}

fn classify_file_inventory_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    let command = tokens.first()?.to_ascii_lowercase();
    match command.as_str() {
        "git"
            if tokens
                .get(1)
                .is_some_and(|subcommand| subcommand == "ls-files") =>
        {
            Some(ReplacementCandidate::DirectoryListingCompact)
        }
        "find" | "find.exe" => tokens
            .iter()
            .any(|token| token == "-type" || token == "-name" || token == "-maxdepth")
            .then_some(ReplacementCandidate::DirectoryListingCompact),
        "fd" | "fd.exe" | "fdfind" | "fdfind.exe" | "tree" | "tree.exe" => {
            Some(ReplacementCandidate::DirectoryListingCompact)
        }
        _ => None,
    }
}

fn classify_process_table_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    let command = tokens.first()?.to_ascii_lowercase();
    match command.as_str() {
        "get-process" | "get-process.exe" | "gps" | "ps" | "tasklist" | "tasklist.exe" => {
            Some(ReplacementCandidate::ProcessTableCompact)
        }
        "get-ciminstance"
        | "get-ciminstance.exe"
        | "gcim"
        | "get-wmiobject"
        | "get-wmiobject.exe"
        | "gwmi" => tokens
            .iter()
            .any(|token| token.eq_ignore_ascii_case("win32_process"))
            .then_some(ReplacementCandidate::ProcessTableCompact),
        _ => None,
    }
}

fn classify_git_filtered_diff_shell_control(command: &str) -> Option<ReplacementCandidate> {
    let lower = command.to_ascii_lowercase();
    (lower.contains("git diff")
        && lower.contains('|')
        && (lower.contains("| rg ")
            || lower.contains("|rg ")
            || lower.contains("| select-string")
            || lower.contains("|select-string")
            || lower.contains("| sls ")
            || lower.contains("|sls ")))
    .then_some(ReplacementCandidate::GitFilteredDiffDigest)
}

fn classify_file_excerpt_shell_control(command: &str) -> Option<ReplacementCandidate> {
    let lower = command.to_ascii_lowercase();
    (lower.contains('|')
        && (lower.starts_with("cat ")
            || lower.starts_with("type ")
            || lower.starts_with("gc ")
            || lower.starts_with("get-content ")
            || lower.contains(" get-content ")
            || lower.contains("; get-content ")
            || lower.contains(" gc ")
            || lower.contains("; gc "))
        && (lower.contains("| head")
            || lower.contains("|head")
            || lower.contains("| tail")
            || lower.contains("|tail")
            || lower.contains("select-object -first")
            || lower.contains("select-object -skip")
            || lower.contains("select-object -last")))
    .then_some(ReplacementCandidate::FileExcerptDigest)
}

fn classify_rg_file_set_shell_control(command: &str) -> Option<ReplacementCandidate> {
    let lower = command.to_ascii_lowercase();
    (lower.contains("rg --files")
        && lower.contains('|')
        && (lower.contains("| head")
            || lower.contains("|head")
            || lower.contains("| tail")
            || lower.contains("|tail")
            || lower.contains("select-object -first")
            || lower.contains("select-object -last")))
    .then_some(ReplacementCandidate::RgFilesCompact)
}

fn classify_select_string_shell_control(command: &str) -> Option<ReplacementCandidate> {
    let lower = command.to_ascii_lowercase();
    (lower.contains("select-string")
        || lower.contains("| sls ")
        || lower.contains("|sls ")
        || lower.starts_with("grep ")
        || lower.contains("; grep ")
        || lower.contains("| grep ")
        || lower.contains("|grep ")
        || lower.starts_with("findstr ")
        || lower.contains("; findstr ")
        || lower.contains("| findstr ")
        || lower.contains("|findstr ")
        || lower.contains("git grep "))
    .then_some(ReplacementCandidate::SelectStringDigest)
}

fn classify_file_inventory_shell_control(command: &str) -> Option<ReplacementCandidate> {
    let lower = command.to_ascii_lowercase();
    (lower.contains('|')
        && (lower.starts_with("git ls-files")
            || lower.contains("; git ls-files")
            || lower.starts_with("find ")
            || lower.contains("; find ")
            || lower.starts_with("fd ")
            || lower.contains("; fd ")
            || lower.starts_with("fdfind ")
            || lower.contains("; fdfind ")
            || lower.starts_with("tree ")
            || lower == "tree"
            || lower.contains("; tree "))
        && (lower.contains("| head")
            || lower.contains("|head")
            || lower.contains("| tail")
            || lower.contains("|tail")
            || lower.contains("select-object -first")
            || lower.contains("select-object -skip")
            || lower.contains("select-object -last")
            || lower.contains("| sort")
            || lower.contains("|sort")))
    .then_some(ReplacementCandidate::DirectoryListingCompact)
}

fn classify_directory_listing_shell_control(command: &str) -> Option<ReplacementCandidate> {
    let lower = command.to_ascii_lowercase();
    (lower.contains('|')
        && (lower.starts_with("ls ")
            || lower == "ls"
            || lower.starts_with("dir ")
            || lower == "dir"
            || lower.starts_with("gci ")
            || lower.starts_with("get-childitem ")
            || lower.contains(" get-childitem ")
            || lower.contains("; get-childitem ")
            || lower.contains(" gci ")
            || lower.contains("; gci "))
        && (lower.contains("| head")
            || lower.contains("|head")
            || lower.contains("| tail")
            || lower.contains("|tail")
            || lower.contains("format-table")
            || lower.contains("format-list")
            || lower.contains("select-object -first")
            || lower.contains("select-object -skip")
            || lower.contains("select-object -last")))
    .then_some(ReplacementCandidate::DirectoryListingCompact)
}

fn classify_run_check_shell_control(command: &str) -> Option<ReplacementCandidate> {
    let primary = primary_shell_segment(command);
    let tokens = shell_tokens(primary)?;
    classify_run_check_digest_candidate(&tokens)
}

fn primary_shell_segment(command: &str) -> &str {
    let mut end = command.len();
    for marker in ["&&", "||", "|", ";", "\n", "\r"] {
        if let Some(index) = command.find(marker) {
            end = end.min(index);
        }
    }
    command[..end].trim()
}

fn command_looks_like_check(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    lower.contains("cargo test")
        || lower.contains("cargo check")
        || lower.contains("cargo build")
        || lower.contains("cargo clippy")
        || lower.contains("cargo fmt")
        || lower.contains("git diff --check")
        || lower.contains("just fmt")
        || lower.contains("just fix")
        || lower.contains("just test")
        || lower.contains("cmake --build")
        || lower.contains("ctest")
        || lower.contains("get-winevent")
        || lower.contains("py_compile")
        || lower.contains("pytest")
        || lower.contains("npm test")
        || lower.contains("pnpm test")
        || lower.contains("yarn test")
        || lower.contains("build-local-codex.ps1")
        || lower.contains("npm run build")
        || lower.contains("npm run lint")
        || lower.contains("pnpm build")
        || lower.contains("pnpm run build")
        || lower.contains("pnpm lint")
        || lower.contains("yarn build")
        || lower.contains("yarn lint")
        || lower.contains("go test")
        || lower.contains("go build")
        || lower.contains("dotnet test")
        || lower.contains("dotnet build")
        || lower.contains("mvn test")
        || lower.contains("mvn verify")
        || lower.contains("gradle test")
        || lower.contains("gradlew test")
        || lower.contains("make test")
        || lower.contains("make check")
        || lower.contains("msbuild")
        || lower.contains("ninja")
        || lower.contains("xcodebuild")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn classifies_git_summary_commands() {
        assert_eq!(classify_shell_replacement("git status --short"), None);
        assert_eq!(
            classify_shell_replacement("git diff --stat"),
            Some(ReplacementCandidate::GitDiffStatCompact)
        );
        assert_eq!(
            classify_shell_replacement("git -C codex-rs diff --stat"),
            Some(ReplacementCandidate::GitDiffStatCompact)
        );
        assert_eq!(
            classify_shell_replacement("git diff --name-only"),
            Some(ReplacementCandidate::GitChangedFiles)
        );
        assert_eq!(
            classify_shell_replacement("git diff -- codex-rs/core/src/tools/mod.rs"),
            Some(ReplacementCandidate::DiffHunkSummary)
        );
        assert_eq!(
            classify_shell_replacement("git ls-files codex-rs/replacement-shadow"),
            Some(ReplacementCandidate::DirectoryListingCompact)
        );
    }

    #[test]
    fn promoted_replacements_keep_status_and_search_in_shadow() {
        assert_eq!(
            classify_promoted_replacement("git status --short --branch"),
            None
        );
        assert_eq!(
            classify_promoted_replacement("rg -n --glob '*.rs' context_ops codex-rs/core"),
            None
        );
    }

    #[test]
    fn promoted_replacements_allow_only_diff_stat() {
        assert_eq!(
            classify_promoted_replacement("git diff --stat"),
            Some(ReplacementCandidate::GitDiffStatCompact)
        );
        assert_eq!(
            classify_promoted_replacement("git diff --cached --stat"),
            Some(ReplacementCandidate::GitDiffStatCompact)
        );
        assert_eq!(
            classify_promoted_replacement("git -C .. diff --stat"),
            Some(ReplacementCandidate::GitDiffStatCompact)
        );
        assert_eq!(classify_promoted_replacement("git diff --name-only"), None);
        assert_eq!(
            classify_promoted_replacement("Get-Content -Path codex-rs/core/src/tools/mod.rs"),
            None
        );
    }

    #[test]
    fn classifies_rg_search_without_shell_control() {
        assert_eq!(
            classify_shell_replacement("rg -n --glob '*.rs' context_ops codex-rs/core"),
            Some(ReplacementCandidate::SearchText {
                pattern: "context_ops".to_string(),
                globs: vec!["*.rs".to_string()],
                paths: vec!["codex-rs/core".to_string()]
            })
        );
        assert_eq!(
            classify_shell_replacement(
                r#"rg -n "bias_numeric_series\(" c_core/src/c_core_analysis_bias_facades.cpp"#
            ),
            Some(ReplacementCandidate::SearchText {
                pattern: "bias_numeric_series\\(".to_string(),
                globs: Vec::new(),
                paths: vec!["c_core/src/c_core_analysis_bias_facades.cpp".to_string()]
            })
        );
        assert_eq!(
            classify_shell_replacement("rg --files | head -n 20"),
            Some(ReplacementCandidate::RgFilesCompact)
        );
    }

    #[test]
    fn classifies_rg_search_options_after_pattern_as_options_not_paths() {
        assert_eq!(
            classify_shell_replacement("rg context_ops -g '*.rs' codex-rs/core"),
            Some(ReplacementCandidate::SearchText {
                pattern: "context_ops".to_string(),
                globs: vec!["*.rs".to_string()],
                paths: vec!["codex-rs/core".to_string()]
            })
        );
        assert_eq!(
            classify_shell_replacement("rg context_ops --max-count 3 codex-rs/core"),
            Some(ReplacementCandidate::SearchText {
                pattern: "context_ops".to_string(),
                globs: Vec::new(),
                paths: vec!["codex-rs/core".to_string()]
            })
        );
    }

    #[test]
    fn classifies_rg_search_with_repeated_globs() {
        assert_eq!(
            classify_shell_replacement(
                "rg -n --glob '*.rs' --glob '!target/**' context_ops codex-rs/core"
            ),
            Some(ReplacementCandidate::SearchText {
                pattern: "context_ops".to_string(),
                globs: vec!["*.rs".to_string(), "!target/**".to_string()],
                paths: vec!["codex-rs/core".to_string()]
            })
        );
    }

    #[test]
    fn classifies_rg_files_and_run_checks_as_baseline_summaries() {
        assert_eq!(
            classify_shell_replacement("rg --files -g '*.rs' codex-rs/core"),
            Some(ReplacementCandidate::RgFilesCompact)
        );
        assert_eq!(
            classify_shell_replacement("cargo test -p codex-core --release --lib context_ops -j 1"),
            Some(ReplacementCandidate::RunCheckDigest)
        );
        assert_eq!(
            classify_shell_replacement(
                r"powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode FastRelease"
            ),
            Some(ReplacementCandidate::RunCheckDigest)
        );
        assert_eq!(
            classify_shell_replacement(
                "$stamp = Get-Date; cargo test -p codex-core --release --lib search_text -j 1 2>&1 | Tee-Object -FilePath logs\\core-search-text.log"
            ),
            Some(ReplacementCandidate::RunCheckDigest)
        );
        assert_eq!(
            classify_shell_replacement("git diff --check"),
            Some(ReplacementCandidate::RunCheckDigest)
        );
        assert_eq!(
            classify_shell_replacement(
                "Invoke-Bounded -FilePath cmake -ArgumentList @('--build','build','--target','wizard_team_app')"
            ),
            Some(ReplacementCandidate::RunCheckDigest)
        );
        assert_eq!(
            classify_shell_replacement("ctest --test-dir build --output-on-failure"),
            Some(ReplacementCandidate::RunCheckDigest)
        );
        assert_eq!(
            classify_shell_replacement("python -m py_compile src/app.py"),
            Some(ReplacementCandidate::RunCheckDigest)
        );
        assert_eq!(
            classify_shell_replacement("npm run build"),
            Some(ReplacementCandidate::RunCheckDigest)
        );
        assert_eq!(
            classify_shell_replacement("go test ./..."),
            Some(ReplacementCandidate::RunCheckDigest)
        );
        assert_eq!(
            classify_shell_replacement("dotnet test MySolution.sln"),
            Some(ReplacementCandidate::RunCheckDigest)
        );
        assert_eq!(
            classify_shell_replacement("make check"),
            Some(ReplacementCandidate::RunCheckDigest)
        );
        assert_eq!(
            classify_shell_replacement(
                "Get-WinEvent -FilterHashtable @{LogName='Application'} | Select-Object -First 20"
            ),
            Some(ReplacementCandidate::RunCheckDigest)
        );
    }

    #[test]
    fn rejects_rg_flags_that_change_search_semantics() {
        assert_eq!(classify_shell_replacement("rg -i foo"), None);
        assert_eq!(classify_shell_replacement("rg -F 'a.b'"), None);
        assert_eq!(classify_shell_replacement("rg --type rust foo"), None);
        assert_eq!(classify_shell_replacement("rg --hidden foo"), None);
        assert_eq!(classify_shell_replacement("rg foo --type rust src"), None);
        assert_eq!(classify_shell_replacement("rg foo --hidden src"), None);
    }

    #[test]
    fn classifies_whole_file_reads_only() {
        assert_eq!(
            classify_shell_replacement("Get-Content -Path codex-rs/core/src/tools/mod.rs"),
            Some(ReplacementCandidate::FileOutline {
                path: PathBuf::from("codex-rs/core/src/tools/mod.rs")
            })
        );
        assert_eq!(
            classify_shell_replacement(
                "Get-Content -Path codex-rs/core/src/tools/mod.rs -TotalCount 40"
            ),
            Some(ReplacementCandidate::FileExcerptDigest)
        );
        assert_eq!(
            classify_shell_replacement(r"Get-Content -Path codex-rs\core\src\tools\mod.rs"),
            Some(ReplacementCandidate::FileOutline {
                path: PathBuf::from(r"codex-rs\core\src\tools\mod.rs")
            })
        );
    }

    #[test]
    fn classifies_expansion_shadow_candidates_after_first_pack() {
        assert_eq!(
            classify_shell_replacement(
                "Get-Content -Path codex-rs/core/src/tools/mod.rs -TotalCount 40"
            ),
            Some(ReplacementCandidate::FileExcerptDigest)
        );
        assert_eq!(
            classify_shell_replacement("Select-String -Path *.rs -Pattern replacement_shadow"),
            Some(ReplacementCandidate::SelectStringDigest)
        );
        assert_eq!(
            classify_shell_replacement("grep -R replacement_shadow codex-rs"),
            Some(ReplacementCandidate::SelectStringDigest)
        );
        assert_eq!(
            classify_shell_replacement("findstr /S /N replacement_shadow *.rs"),
            Some(ReplacementCandidate::SelectStringDigest)
        );
        assert_eq!(
            classify_shell_replacement("git grep -n replacement_shadow -- codex-rs"),
            Some(ReplacementCandidate::SelectStringDigest)
        );
        assert_eq!(
            classify_shell_replacement("rg --count replacement_shadow codex-rs/core"),
            Some(ReplacementCandidate::RgCountDigest)
        );
        assert_eq!(
            classify_shell_replacement("rg -l replacement_shadow codex-rs/core"),
            Some(ReplacementCandidate::RgFileSetDigest)
        );
        assert_eq!(
            classify_shell_replacement("rg --json replacement_shadow codex-rs/core"),
            Some(ReplacementCandidate::RgJsonDigest)
        );
        assert_eq!(
            classify_shell_replacement("git diff --name-status --cached"),
            Some(ReplacementCandidate::GitNameStatusCompact)
        );
        assert_eq!(
            classify_shell_replacement("git diff --numstat"),
            Some(ReplacementCandidate::GitNumstatCompact)
        );
        assert_eq!(
            classify_shell_replacement("git log --stat -- codex-rs/core"),
            Some(ReplacementCandidate::GitHistoryDigest)
        );
        assert_eq!(
            classify_shell_replacement("Get-ChildItem -Recurse codex-rs/core/src/tools"),
            Some(ReplacementCandidate::DirectoryListingCompact)
        );
        assert_eq!(
            classify_shell_replacement("find . -type f -name '*.rs'"),
            Some(ReplacementCandidate::DirectoryListingCompact)
        );
        assert_eq!(
            classify_shell_replacement("fd replacement codex-rs"),
            Some(ReplacementCandidate::DirectoryListingCompact)
        );
        assert_eq!(
            classify_shell_replacement("tree -L 2 codex-rs"),
            Some(ReplacementCandidate::DirectoryListingCompact)
        );
        assert_eq!(
            classify_shell_replacement("Get-Process"),
            Some(ReplacementCandidate::ProcessTableCompact)
        );
        assert_eq!(
            classify_shell_replacement("git diff --name-only"),
            Some(ReplacementCandidate::GitChangedFiles)
        );
        assert_eq!(
            classify_shell_replacement("rg --files -g '*.rs' codex-rs/core"),
            Some(ReplacementCandidate::RgFilesCompact)
        );
    }

    #[test]
    fn classifies_baseline_only_shell_control_expansions() {
        assert_eq!(
            classify_shell_replacement("git diff -- codex-rs/core | rg replacement_shadow"),
            Some(ReplacementCandidate::GitFilteredDiffDigest)
        );
        assert_eq!(
            classify_shell_replacement("cat codex-rs/core/src/lib.rs | head -n 40"),
            Some(ReplacementCandidate::FileExcerptDigest)
        );
        assert_eq!(
            classify_shell_replacement(
                "$i=1; Get-Content -Path scripts\\build-local-codex.ps1 | Select-Object -Skip 20 -First 40"
            ),
            Some(ReplacementCandidate::FileExcerptDigest)
        );
        assert_eq!(
            classify_shell_replacement("rg --files codex-rs/core | head -n 20"),
            Some(ReplacementCandidate::RgFilesCompact)
        );
        assert_eq!(
            classify_shell_replacement("git ls-files codex-rs | head -n 40"),
            Some(ReplacementCandidate::DirectoryListingCompact)
        );
        assert_eq!(
            classify_shell_replacement("find . -type f | sort | head -n 40"),
            Some(ReplacementCandidate::DirectoryListingCompact)
        );
        assert_eq!(
            classify_shell_replacement("Get-ChildItem codex-rs | Select-Object -First 20"),
            Some(ReplacementCandidate::DirectoryListingCompact)
        );
        assert_eq!(
            classify_shell_replacement(
                "$root='reports'; Get-ChildItem $root -Recurse | Select-Object -First 20"
            ),
            Some(ReplacementCandidate::DirectoryListingCompact)
        );
        assert_eq!(
            classify_shell_replacement("Get-ChildItem codex-rs | Select-String replacement_shadow"),
            Some(ReplacementCandidate::SelectStringDigest)
        );
        assert_eq!(
            classify_shell_replacement("grep -R replacement_shadow codex-rs | head -n 20"),
            Some(ReplacementCandidate::SelectStringDigest)
        );
        assert_eq!(
            classify_shell_replacement("cargo test 2>&1 | Select-String error"),
            Some(ReplacementCandidate::RunCheckDigest)
        );
        assert_eq!(
            classify_shell_replacement("cargo test 2>&1 | grep failed"),
            Some(ReplacementCandidate::RunCheckDigest)
        );
    }

    #[test]
    fn search_patterns_that_look_like_checks_stay_search_shadows() {
        assert_eq!(
            classify_shell_replacement("Select-String -Path *.rs -Pattern 'cargo test'"),
            Some(ReplacementCandidate::SelectStringDigest)
        );
        assert_eq!(
            classify_shell_replacement(
                "Get-Content build.log | Select-String -Pattern 'cargo test'"
            ),
            Some(ReplacementCandidate::SelectStringDigest)
        );
        assert_eq!(
            classify_shell_replacement("grep -R 'go test' codex-rs"),
            Some(ReplacementCandidate::SelectStringDigest)
        );
    }

    #[test]
    fn promoted_replacements_reject_expansion_candidates() {
        assert_eq!(
            classify_promoted_replacement("git diff --name-status"),
            None
        );
        assert_eq!(classify_promoted_replacement("git diff --numstat"), None);
        assert_eq!(
            classify_promoted_replacement("rg --count replacement_shadow codex-rs/core"),
            None
        );
        assert_eq!(
            classify_promoted_replacement("rg -l replacement_shadow codex-rs/core"),
            None
        );
        assert_eq!(
            classify_promoted_replacement("rg --json replacement_shadow codex-rs/core"),
            None
        );
        assert_eq!(
            classify_promoted_replacement("rg --files -g '*.rs' codex-rs/core"),
            None
        );
        assert_eq!(
            classify_promoted_replacement("Select-String -Path *.rs -Pattern replacement_shadow"),
            None
        );
        assert_eq!(
            classify_promoted_replacement("Get-ChildItem -Recurse codex-rs/core/src/tools"),
            None
        );
        assert_eq!(
            classify_promoted_replacement("git log --stat -- codex-rs/core"),
            None
        );
        assert_eq!(
            classify_promoted_replacement("git diff -- codex-rs/core | rg replacement_shadow"),
            None
        );
        assert_eq!(classify_promoted_replacement("Get-Process"), None);
        assert_eq!(
            classify_promoted_replacement(
                "Get-Content -Path codex-rs/core/src/lib.rs -TotalCount 40"
            ),
            None
        );
    }
}
