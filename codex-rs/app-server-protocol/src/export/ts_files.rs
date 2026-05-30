use super::*;

pub(crate) fn prepend_header_if_missing(path: &Path) -> Result<()> {
    let mut content = String::new();
    {
        let mut f = fs::File::open(path)
            .with_context(|| format!("Failed to open {} for reading", path.display()))?;
        f.read_to_string(&mut content)
            .with_context(|| format!("Failed to read {}", path.display()))?;
    }

    if content.starts_with(GENERATED_TS_HEADER) {
        return Ok(());
    }

    let mut f = fs::File::create(path)
        .with_context(|| format!("Failed to open {} for writing", path.display()))?;
    f.write_all(GENERATED_TS_HEADER.as_bytes())
        .with_context(|| format!("Failed to write header to {}", path.display()))?;
    f.write_all(content.as_bytes())
        .with_context(|| format!("Failed to write content to {}", path.display()))?;
    Ok(())
}

fn ts_files_in(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in
        fs::read_dir(dir).with_context(|| format!("Failed to read dir {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension() == Some(OsStr::new("ts")) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

pub(crate) fn ts_files_in_recursive(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in
            fs::read_dir(&d).with_context(|| format!("Failed to read dir {}", d.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() && path.extension() == Some(OsStr::new("ts")) {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

pub(crate) fn trim_trailing_whitespace_in_ts_files(paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let trimmed = trim_trailing_line_whitespace(&content);
        if trimmed != content {
            fs::write(path, trimmed)
                .with_context(|| format!("Failed to write {}", path.display()))?;
        }
    }
    Ok(())
}

pub(crate) fn trim_trailing_line_whitespace(content: &str) -> String {
    let mut trimmed = String::with_capacity(content.len());
    for line in content.split_inclusive('\n') {
        if let Some(line_without_newline) = line.strip_suffix('\n') {
            trimmed.push_str(line_without_newline.trim_end_matches([' ', '\t']));
            trimmed.push('\n');
        } else {
            trimmed.push_str(line.trim_end_matches([' ', '\t']));
        }
    }
    trimmed
}

/// Generate an index.ts file that re-exports all generated types.
/// This allows consumers to import all types from a single file.
pub(crate) fn generate_index_ts(out_dir: &Path) -> Result<PathBuf> {
    let content = generated_index_ts_with_header(index_ts_entries(
        &ts_files_in(out_dir)?
            .iter()
            .map(PathBuf::as_path)
            .collect::<Vec<_>>(),
        ts_files_in(&out_dir.join("v2"))
            .map(|v| !v.is_empty())
            .unwrap_or(false),
    ));

    let index_path = out_dir.join("index.ts");
    let mut f = fs::File::create(&index_path)
        .with_context(|| format!("Failed to create {}", index_path.display()))?;
    f.write_all(content.as_bytes())
        .with_context(|| format!("Failed to write {}", index_path.display()))?;
    Ok(index_path)
}

pub(crate) fn generate_index_ts_tree(tree: &mut BTreeMap<PathBuf, String>) {
    let root_entries = tree
        .keys()
        .filter(|path| path.components().count() == 1)
        .map(PathBuf::as_path)
        .collect::<Vec<_>>();
    let has_v2_ts = tree.keys().any(|path| {
        path.parent()
            .is_some_and(|parent| parent == Path::new("v2"))
            && path.extension() == Some(OsStr::new("ts"))
            && path.file_stem().is_some_and(|stem| stem != "index")
    });
    tree.insert(
        PathBuf::from("index.ts"),
        index_ts_entries(&root_entries, has_v2_ts),
    );

    let v2_entries = tree
        .keys()
        .filter(|path| {
            path.parent()
                .is_some_and(|parent| parent == Path::new("v2"))
        })
        .map(PathBuf::as_path)
        .collect::<Vec<_>>();
    if !v2_entries.is_empty() {
        tree.insert(
            PathBuf::from("v2").join("index.ts"),
            index_ts_entries(&v2_entries, /*has_v2_ts*/ false),
        );
    }
}

fn generated_index_ts_with_header(content: String) -> String {
    let mut with_header = String::with_capacity(GENERATED_TS_HEADER.len() + content.len());
    with_header.push_str(GENERATED_TS_HEADER);
    with_header.push_str(&content);
    with_header
}

fn index_ts_entries(paths: &[&Path], has_v2_ts: bool) -> String {
    let mut stems: Vec<String> = paths
        .iter()
        .filter(|path| path.extension() == Some(OsStr::new("ts")))
        .filter_map(|path| {
            let stem = path.file_stem()?.to_string_lossy().into_owned();
            if stem == "index" { None } else { Some(stem) }
        })
        .filter(|stem| stem != "EventMsg")
        .collect();
    stems.sort();
    stems.dedup();

    let mut entries = String::new();
    for name in stems {
        entries.push_str(&format!("export type {{ {name} }} from \"./{name}\";\n"));
    }
    if has_v2_ts {
        entries.push_str("export * as v2 from \"./v2\";\n");
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema_fixtures::read_schema_fixture_subtree;
    use anyhow::Context;
    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use std::collections::BTreeSet;
    use std::path::Path;
    use std::path::PathBuf;

    #[test]
    fn generated_ts_optional_nullable_fields_only_in_params() -> Result<()> {
        // Assert that "?: T | null" only appears in generated *Params types.
        let fixture_tree = read_schema_fixture_subtree(&schema_root()?, "typescript")?;

        let client_request_ts = std::str::from_utf8(
            fixture_tree
                .get(Path::new("ClientRequest.ts"))
                .ok_or_else(|| anyhow::anyhow!("missing ClientRequest.ts fixture"))?,
        )?;
        assert_eq!(client_request_ts.contains("mock/experimentalMethod"), false);
        assert_eq!(
            client_request_ts.contains("MockExperimentalMethodParams"),
            false
        );
        let typescript_index = std::str::from_utf8(
            fixture_tree
                .get(Path::new("index.ts"))
                .ok_or_else(|| anyhow::anyhow!("missing index.ts fixture"))?,
        )?;
        assert_eq!(typescript_index.contains("export type { EventMsg }"), false);
        let thread_start_ts = std::str::from_utf8(
            fixture_tree
                .get(Path::new("v2/ThreadStartParams.ts"))
                .ok_or_else(|| anyhow::anyhow!("missing v2/ThreadStartParams.ts fixture"))?,
        )?;
        assert_eq!(thread_start_ts.contains("mockExperimentalField"), false);
        let review_target_ts = std::str::from_utf8(
            fixture_tree
                .get(Path::new("v2/ReviewTarget.ts"))
                .ok_or_else(|| anyhow::anyhow!("missing v2/ReviewTarget.ts fixture"))?,
        )?;
        assert_eq!(review_target_ts.contains("title?: string | null"), true);
        assert_eq!(review_target_ts.contains("title: string | null"), false);
        assert_eq!(
            fixture_tree.contains_key(Path::new("v2/MockExperimentalMethodParams.ts")),
            false
        );
        assert_eq!(
            fixture_tree.contains_key(Path::new("v2/MockExperimentalMethodResponse.ts")),
            false
        );

        let mut undefined_offenders = Vec::new();
        let mut optional_nullable_offenders = BTreeSet::new();
        for (path, contents) in &fixture_tree {
            if !matches!(path.extension().and_then(|ext| ext.to_str()), Some("ts")) {
                continue;
            }

            // Only allow "?: T | null" in objects representing JSON-RPC requests,
            // which we assume are called "*Params", plus documented input union
            // shapes that accept omitted-or-null fields.
            let allow_optional_nullable = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| {
                    stem.ends_with("Params")
                        || stem == "InitializeCapabilities"
                        || matches!(
                            stem,
                            "CollabAgentRef"
                                | "CollabAgentStatusEntry"
                                | "CollabAgentSpawnEndEvent"
                                | "CollabAgentInteractionEndEvent"
                                | "CollabCloseEndEvent"
                                | "CollabResumeBeginEvent"
                                | "CollabResumeEndEvent"
                                | "CollabCompactBeginEvent"
                                | "CollabCompactEndEvent"
                                | "CollabRestartBeginEvent"
                                | "CollabRestartEndEvent"
                                | "ReviewTarget"
                        )
                });

            let contents = std::str::from_utf8(contents)?;
            if contents.contains("| undefined") {
                undefined_offenders.push(path.clone());
            }

            const SKIP_PREFIXES: &[&str] = &[
                "const ",
                "let ",
                "var ",
                "export const ",
                "export let ",
                "export var ",
            ];

            let mut search_start = 0;
            while let Some(idx) = contents[search_start..].find("| null") {
                let abs_idx = search_start + idx;
                // Find the property-colon for this field by scanning forward
                // from the start of the segment and ignoring nested braces,
                // brackets, and parens. This avoids colons inside nested
                // type literals like `{ [k in string]?: string }`.

                let line_start_idx = contents[..abs_idx].rfind('\n').map(|i| i + 1).unwrap_or(0);

                let mut segment_start_idx = line_start_idx;
                if let Some(rel_idx) = contents[line_start_idx..abs_idx].rfind(',') {
                    segment_start_idx = segment_start_idx.max(line_start_idx + rel_idx + 1);
                }
                if let Some(rel_idx) = contents[line_start_idx..abs_idx].rfind('{') {
                    segment_start_idx = segment_start_idx.max(line_start_idx + rel_idx + 1);
                }
                if let Some(rel_idx) = contents[line_start_idx..abs_idx].rfind('}') {
                    segment_start_idx = segment_start_idx.max(line_start_idx + rel_idx + 1);
                }

                // Scan forward for the colon that separates the field name from its type.
                let mut level_brace = 0_i32;
                let mut level_brack = 0_i32;
                let mut level_paren = 0_i32;
                let mut in_single = false;
                let mut in_double = false;
                let mut escape = false;
                let mut prop_colon_idx = None;
                for (i, ch) in contents[segment_start_idx..abs_idx].char_indices() {
                    let idx_abs = segment_start_idx + i;
                    if escape {
                        escape = false;
                        continue;
                    }
                    match ch {
                        '\\' => {
                            if in_single || in_double {
                                escape = true;
                            }
                        }
                        '\'' => {
                            if !in_double {
                                in_single = !in_single;
                            }
                        }
                        '"' => {
                            if !in_single {
                                in_double = !in_double;
                            }
                        }
                        '{' if !in_single && !in_double => level_brace += 1,
                        '}' if !in_single && !in_double => level_brace -= 1,
                        '[' if !in_single && !in_double => level_brack += 1,
                        ']' if !in_single && !in_double => level_brack -= 1,
                        '(' if !in_single && !in_double => level_paren += 1,
                        ')' if !in_single && !in_double => level_paren -= 1,
                        ':' if !in_single
                            && !in_double
                            && level_brace == 0
                            && level_brack == 0
                            && level_paren == 0 =>
                        {
                            prop_colon_idx = Some(idx_abs);
                            break;
                        }
                        _ => {}
                    }
                }

                let Some(colon_idx) = prop_colon_idx else {
                    search_start = abs_idx + 5;
                    continue;
                };

                let mut field_prefix = contents[segment_start_idx..colon_idx].trim();
                if field_prefix.is_empty() {
                    search_start = abs_idx + 5;
                    continue;
                }

                if let Some(comment_idx) = field_prefix.rfind("*/") {
                    field_prefix = field_prefix[comment_idx + 2..].trim_start();
                }

                if field_prefix.is_empty() {
                    search_start = abs_idx + 5;
                    continue;
                }

                if SKIP_PREFIXES
                    .iter()
                    .any(|prefix| field_prefix.starts_with(prefix))
                {
                    search_start = abs_idx + 5;
                    continue;
                }

                if field_prefix.contains('(') {
                    search_start = abs_idx + 5;
                    continue;
                }

                // If the last non-whitespace before ':' is '?', then this is an
                // optional field with a nullable type (i.e., "?: T | null").
                // These are only allowed in *Params types.
                if field_prefix.chars().rev().find(|c| !c.is_whitespace()) == Some('?')
                    && !allow_optional_nullable
                {
                    let line_number =
                        contents[..abs_idx].chars().filter(|c| *c == '\n').count() + 1;
                    let offending_line_end = contents[line_start_idx..]
                        .find('\n')
                        .map(|i| line_start_idx + i)
                        .unwrap_or(contents.len());
                    let offending_snippet = contents[line_start_idx..offending_line_end].trim();

                    optional_nullable_offenders.insert(format!(
                        "{}:{}: {offending_snippet}",
                        path.display(),
                        line_number
                    ));
                }

                search_start = abs_idx + 5;
            }
        }

        assert!(
            undefined_offenders.is_empty(),
            "Generated TypeScript still includes unions with `undefined` in {undefined_offenders:?}"
        );

        // If this assertion fails, it means a field was generated as "?: T | null",
        // which is both optional (undefined) and nullable (null), for a type not ending
        // in "Params" (which represent JSON-RPC requests).
        assert!(
            optional_nullable_offenders.is_empty(),
            "Generated TypeScript has optional nullable fields outside *Params types (disallowed '?: T | null'):\n{optional_nullable_offenders:?}"
        );

        Ok(())
    }

    fn schema_root() -> Result<PathBuf> {
        let typescript_index = codex_utils_cargo_bin::find_resource!("schema/typescript/index.ts")
            .context("resolve TypeScript schema index.ts")?;
        let schema_root = typescript_index
            .parent()
            .and_then(|parent| parent.parent())
            .context("derive schema root from schema/typescript/index.ts")?
            .to_path_buf();
        Ok(schema_root)
    }
}
