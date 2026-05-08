use std::path::Path;

use crate::ContextOpsError;

pub const DEFAULT_MAX_OUTLINE_ITEMS: usize = 200;
const MAX_OUTLINE_ITEMS: usize = 1000;
const MAX_SIGNATURE_CHARS: usize = 180;
const MAX_IMPORTS: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
struct OutlineItem {
    line: usize,
    kind: &'static str,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileOutline {
    line_count: usize,
    imports: Vec<OutlineItem>,
    omitted_imports: usize,
    definitions: Vec<OutlineItem>,
    omitted_definitions: usize,
}

pub async fn file_outline(path: &Path, max_items: usize) -> Result<String, ContextOpsError> {
    let max_items = max_items.clamp(1, MAX_OUTLINE_ITEMS);
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|err| ContextOpsError::new(format!("failed to read file: {err}")))?;
    Ok(file_outline_from_bytes(path, &bytes, max_items))
}

pub fn file_outline_from_bytes(path: &Path, bytes: &[u8], max_items: usize) -> String {
    let text = String::from_utf8_lossy(bytes);
    let outline = build_file_outline(&text, max_items.clamp(1, MAX_OUTLINE_ITEMS));
    render_file_outline(path, &outline)
}

fn build_file_outline(text: &str, max_items: usize) -> FileOutline {
    let mut imports = Vec::new();
    let mut omitted_imports = 0usize;
    let mut definitions = Vec::new();
    let mut omitted_definitions = 0usize;
    let mut line_count = 0usize;

    for (index, line) in text.lines().enumerate() {
        line_count = index + 1;
        let trimmed = line.trim_start();
        if let Some(kind) = classify_import(trimmed) {
            if imports.len() < MAX_IMPORTS {
                imports.push(outline_item(index + 1, kind, trimmed));
            } else {
                omitted_imports += 1;
            }
            continue;
        }

        if let Some(kind) = classify_definition(trimmed) {
            if definitions.len() < max_items {
                definitions.push(outline_item(index + 1, kind, trimmed));
            } else {
                omitted_definitions += 1;
            }
        }
    }

    FileOutline {
        line_count,
        imports,
        omitted_imports,
        definitions,
        omitted_definitions,
    }
}

fn outline_item(line: usize, kind: &'static str, text: &str) -> OutlineItem {
    OutlineItem {
        line,
        kind,
        text: truncate_signature(text),
    }
}

fn classify_import(trimmed: &str) -> Option<&'static str> {
    let line = strip_rust_visibility(trimmed);
    if line.starts_with("use ") {
        return Some("use");
    }
    if line.starts_with("import ") || line.starts_with("from ") {
        return Some("import");
    }
    if line.starts_with("#include") {
        return Some("include");
    }
    None
}

fn classify_definition(trimmed: &str) -> Option<&'static str> {
    if trimmed.is_empty()
        || trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.starts_with('#')
    {
        return None;
    }

    if trimmed.starts_with("macro_rules!") {
        return Some("macro");
    }

    if trimmed.starts_with("async def ") || trimmed.starts_with("def ") {
        return Some("fn");
    }
    if trimmed.starts_with("class ") {
        return Some("class");
    }

    let js_line = strip_js_export_prefix(trimmed);
    if js_line.starts_with("async function ") || js_line.starts_with("function ") {
        return Some("fn");
    }
    if js_line.starts_with("class ") {
        return Some("class");
    }
    if js_line.starts_with("interface ") {
        return Some("interface");
    }
    if js_line.starts_with("type ") {
        return Some("type");
    }
    if looks_like_js_function_binding(js_line) {
        return Some("fn");
    }

    let rust_line = strip_rust_prefixes(strip_rust_visibility(trimmed));
    if rust_line.starts_with("fn ") {
        return Some("fn");
    }
    if rust_line.starts_with("struct ") {
        return Some("struct");
    }
    if rust_line.starts_with("enum ") {
        return Some("enum");
    }
    if rust_line.starts_with("trait ") {
        return Some("trait");
    }
    if rust_line.starts_with("impl") {
        return Some("impl");
    }
    if rust_line.starts_with("mod ") {
        return Some("mod");
    }
    if rust_line.starts_with("type ") {
        return Some("type");
    }

    None
}

fn strip_rust_visibility(line: &str) -> &str {
    if let Some(rest) = line.strip_prefix("pub ") {
        return rest.trim_start();
    }
    if let Some(rest) = line.strip_prefix("pub(")
        && let Some((_, after)) = rest.split_once(") ")
    {
        return after.trim_start();
    }
    line
}

fn strip_rust_prefixes(mut line: &str) -> &str {
    loop {
        let next = if let Some(rest) = line.strip_prefix("async ") {
            rest
        } else if let Some(rest) = line.strip_prefix("unsafe ") {
            rest
        } else if let Some(rest) = line.strip_prefix("const ") {
            rest
        } else {
            line
        };
        if next.len() == line.len() {
            return line;
        }
        line = next.trim_start();
    }
}

fn strip_js_export_prefix(mut line: &str) -> &str {
    loop {
        let next = if let Some(rest) = line.strip_prefix("export default ") {
            rest
        } else if let Some(rest) = line.strip_prefix("export ") {
            rest
        } else if let Some(rest) = line.strip_prefix("declare ") {
            rest
        } else {
            line
        };
        if next.len() == line.len() {
            return line;
        }
        line = next.trim_start();
    }
}

fn looks_like_js_function_binding(line: &str) -> bool {
    let Some(rest) = line
        .strip_prefix("const ")
        .or_else(|| line.strip_prefix("let "))
        .or_else(|| line.strip_prefix("var "))
    else {
        return false;
    };
    rest.contains("=>") || rest.contains("function")
}

fn truncate_signature(text: &str) -> String {
    let mut output = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index >= MAX_SIGNATURE_CHARS {
            output.push_str("...");
            return output;
        }
        output.push(ch);
    }
    output
}

fn render_file_outline(path: &Path, outline: &FileOutline) -> String {
    let mut lines = vec![
        "file_outline".to_string(),
        format!("path: {}", path.display()),
        format!("lines: {}", outline.line_count),
        format!(
            "imports: {} shown, {} omitted",
            outline.imports.len(),
            outline.omitted_imports
        ),
        format!(
            "definitions: {} shown, {} omitted",
            outline.definitions.len(),
            outline.omitted_definitions
        ),
        "fallback_required: true".to_string(),
    ];

    if !outline.imports.is_empty() {
        lines.push("imports:".to_string());
        lines.extend(outline.imports.iter().map(render_outline_item));
    }
    if !outline.definitions.is_empty() {
        lines.push("definitions:".to_string());
        lines.extend(outline.definitions.iter().map(render_outline_item));
    }

    lines.join("\n")
}

fn render_outline_item(item: &OutlineItem) -> String {
    format!("L{} {} {}", item.line, item.kind, item.text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn file_outline_detects_common_definitions_and_caps_results() {
        let outline = build_file_outline(
            r#"
use crate::thing;
pub struct Config {
}
impl Config {
    pub async fn load() {}
}
export const run = () => true;
def helper():
    pass
"#,
            3,
        );

        assert_eq!(outline.imports.len(), 1);
        assert_eq!(outline.omitted_definitions, 2);
        assert_eq!(
            outline
                .definitions
                .iter()
                .map(|item| (item.line, item.kind, item.text.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (3, "struct", "pub struct Config {"),
                (5, "impl", "impl Config {"),
                (6, "fn", "pub async fn load() {}"),
            ]
        );
    }

    #[test]
    fn rendered_file_outline_marks_lossy_output_as_fallback_required() {
        let outline = build_file_outline("plain body\nwithout definitions\n", 10);
        let rendered = render_file_outline(Path::new("README.md"), &outline);

        assert!(rendered.contains("fallback_required: true"));
    }
}
