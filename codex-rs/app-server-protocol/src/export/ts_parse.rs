use super::*;

pub(crate) fn prune_unused_type_imports(content: String, type_alias_body: &str) -> String {
    let trailing_newline = content.ends_with('\n');
    let mut lines = Vec::new();
    for line in content.lines() {
        if let Some(type_name) = parse_imported_type_name(line)
            && !type_alias_body.contains(type_name)
        {
            continue;
        }
        lines.push(line);
    }

    let mut rewritten = lines.join("\n");
    if trailing_newline {
        rewritten.push('\n');
    }
    rewritten
}

fn parse_imported_type_name(line: &str) -> Option<&str> {
    let line = line.trim();
    let rest = line.strip_prefix("import type {")?;
    let (type_name, _) = rest.split_once("} from ")?;
    let type_name = type_name.trim();
    if type_name.is_empty() || type_name.contains(',') || type_name.contains(" as ") {
        return None;
    }
    Some(type_name)
}

pub(crate) fn json_files_in_recursive(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in fs::read_dir(&current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if matches!(path.extension().and_then(|ext| ext.to_str()), Some("json")) {
                out.push(path);
            }
        }
    }
    Ok(out)
}

pub(crate) fn read_json_value(path: &Path) -> Result<Value> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

pub(crate) fn split_type_alias(content: &str) -> Option<(String, String, String)> {
    let eq_index = content.find('=')?;
    let semi_index = content.rfind(';')?;
    if semi_index <= eq_index {
        return None;
    }
    let prefix = content[..eq_index + 1].to_string();
    let body = content[eq_index + 1..semi_index].to_string();
    let suffix = content[semi_index..].to_string();
    Some((prefix, body, suffix))
}

pub(crate) fn type_body_brace_span(content: &str) -> Option<(usize, usize)> {
    if let Some(eq_index) = content.find('=') {
        let after_eq = &content[eq_index + 1..];
        let (open_rel, close_rel) = find_top_level_brace_span(after_eq)?;
        return Some((eq_index + 1 + open_rel, eq_index + 1 + close_rel));
    }

    const INTERFACE_MARKER: &str = "export interface";
    let interface_index = content.find(INTERFACE_MARKER)?;
    let after_interface = &content[interface_index + INTERFACE_MARKER.len()..];
    let (open_rel, close_rel) = find_top_level_brace_span(after_interface)?;
    Some((
        interface_index + INTERFACE_MARKER.len() + open_rel,
        interface_index + INTERFACE_MARKER.len() + close_rel,
    ))
}

fn find_top_level_brace_span(input: &str) -> Option<(usize, usize)> {
    let mut state = ScanState::default();
    let mut open_index = None;
    for (index, ch) in input.char_indices() {
        if !state.in_ignored_syntax() && ch == '{' && state.depth.is_top_level() {
            open_index = Some(index);
        }
        state.observe(ch);
        if !state.in_ignored_syntax()
            && ch == '}'
            && state.depth.is_top_level()
            && let Some(open) = open_index
        {
            return Some((open, index));
        }
    }
    None
}

pub(crate) fn split_top_level(input: &str, delimiter: char) -> Vec<String> {
    split_top_level_multi(input, &[delimiter])
}

pub(crate) fn split_top_level_multi(input: &str, delimiters: &[char]) -> Vec<String> {
    let mut state = ScanState::default();
    let mut start = 0usize;
    let mut parts = Vec::new();
    for (index, ch) in input.char_indices() {
        if !state.in_ignored_syntax() && state.depth.is_top_level() && delimiters.contains(&ch) {
            let part = input[start..index].trim();
            if !part.is_empty() {
                parts.push(part.to_string());
            }
            start = index + ch.len_utf8();
        }
        state.observe(ch);
    }
    let tail = input[start..].trim();
    if !tail.is_empty() {
        parts.push(tail.to_string());
    }
    parts
}

pub(crate) fn extract_method_from_arm(arm: &str) -> Option<String> {
    let (open, close) = find_top_level_brace_span(arm)?;
    let inner = &arm[open + 1..close];
    for field in split_top_level(inner, ',') {
        let Some((name, value)) = parse_property(field.as_str()) else {
            continue;
        };
        if name != "method" {
            continue;
        }
        let value = value.trim_start();
        let (literal, _) = parse_string_literal(value)?;
        return Some(literal);
    }
    None
}

fn parse_property(input: &str) -> Option<(String, &str)> {
    let name = parse_property_name(input)?;
    let colon_index = input.find(':')?;
    Some((name, input[colon_index + 1..].trim_start()))
}

pub(crate) fn strip_leading_block_comments(input: &str) -> &str {
    let mut rest = input.trim_start();
    loop {
        let Some(after_prefix) = rest.strip_prefix("/*") else {
            return rest;
        };
        let Some(end_rel) = after_prefix.find("*/") else {
            return rest;
        };
        rest = after_prefix[end_rel + 2..].trim_start();
    }
}

pub(crate) fn parse_property_name(input: &str) -> Option<String> {
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    if let Some((literal, consumed)) = parse_string_literal(trimmed) {
        let rest = trimmed[consumed..].trim_start();
        if rest.starts_with(':') {
            return Some(literal);
        }
        return None;
    }

    let mut end = 0usize;
    for (index, ch) in trimmed.char_indices() {
        if !is_ident_char(ch) {
            break;
        }
        end = index + ch.len_utf8();
    }
    if end == 0 {
        return None;
    }
    let name = &trimmed[..end];
    let rest = trimmed[end..].trim_start();
    let rest = if let Some(stripped) = rest.strip_prefix('?') {
        stripped.trim_start()
    } else {
        rest
    };
    if rest.starts_with(':') {
        return Some(name.to_string());
    }
    None
}

fn parse_string_literal(input: &str) -> Option<(String, usize)> {
    let mut chars = input.char_indices();
    let (start_index, quote) = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let mut escape = false;
    for (index, ch) in chars {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' {
            escape = true;
            continue;
        }
        if ch == quote {
            let literal = input[start_index + 1..index].to_string();
            let consumed = index + ch.len_utf8();
            return Some((literal, consumed));
        }
    }
    None
}

fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[derive(Default)]
struct ScanState {
    depth: Depth,
    string_delim: Option<char>,
    escape: bool,
    block_comment: bool,
    line_comment: bool,
    previous_char: Option<char>,
}

impl ScanState {
    fn observe(&mut self, ch: char) {
        if self.line_comment {
            if ch == '\n' {
                self.line_comment = false;
            }
            self.previous_char = Some(ch);
            return;
        }

        if self.block_comment {
            if self.previous_char == Some('*') && ch == '/' {
                self.block_comment = false;
                self.previous_char = None;
            } else {
                self.previous_char = Some(ch);
            }
            return;
        }

        if let Some(delim) = self.string_delim {
            if self.escape {
                self.escape = false;
                self.previous_char = Some(ch);
                return;
            }
            if ch == '\\' {
                self.escape = true;
                self.previous_char = Some(ch);
                return;
            }
            if ch == delim {
                self.string_delim = None;
            }
            self.previous_char = Some(ch);
            return;
        }

        if self.previous_char == Some('/') && ch == '/' {
            self.line_comment = true;
            self.previous_char = Some(ch);
            return;
        }

        if self.previous_char == Some('/') && ch == '*' {
            self.block_comment = true;
            self.previous_char = Some(ch);
            return;
        }

        match ch {
            '"' | '\'' => {
                self.string_delim = Some(ch);
            }
            '{' => self.depth.brace += 1,
            '}' => self.depth.brace = (self.depth.brace - 1).max(0),
            '[' => self.depth.bracket += 1,
            ']' => self.depth.bracket = (self.depth.bracket - 1).max(0),
            '(' => self.depth.paren += 1,
            ')' => self.depth.paren = (self.depth.paren - 1).max(0),
            '<' => self.depth.angle += 1,
            '>' => {
                if self.depth.angle > 0 {
                    self.depth.angle -= 1;
                }
            }
            _ => {}
        }
        self.previous_char = Some(ch);
    }

    fn in_ignored_syntax(&self) -> bool {
        self.string_delim.is_some() || self.block_comment || self.line_comment
    }
}

#[derive(Default)]
struct Depth {
    brace: i32,
    bracket: i32,
    paren: i32,
    angle: i32,
}

impl Depth {
    fn is_top_level(&self) -> bool {
        self.brace == 0 && self.bracket == 0 && self.paren == 0 && self.angle == 0
    }
}
