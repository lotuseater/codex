use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseItem;
use codex_utils_cache::sha1_digest;
use std::collections::HashMap;
use std::fmt::Write as _;

const LARGE_TOOL_OUTPUT_MIN_BYTES: usize = 2 * 1024;
const ELISION_PREFIX: &str = "Repeated tool output omitted to save prompt tokens.";
const RUNNING_PROCESS_MARKER: &str = "Process running with session ID";

struct SeenToolOutput {
    call_id: String,
    bytes: usize,
}

pub(super) fn elide_repeated_large_tool_outputs(items: &mut [ResponseItem]) {
    let mut seen = HashMap::new();

    for item in items {
        match item {
            ResponseItem::FunctionCallOutput { call_id, output }
            | ResponseItem::CustomToolCallOutput {
                call_id, output, ..
            } => maybe_elide_tool_output(call_id, output, &mut seen),
            ResponseItem::Message { .. }
            | ResponseItem::Reasoning { .. }
            | ResponseItem::LocalShellCall { .. }
            | ResponseItem::FunctionCall { .. }
            | ResponseItem::ToolSearchCall { .. }
            | ResponseItem::CustomToolCall { .. }
            | ResponseItem::ToolSearchOutput { .. }
            | ResponseItem::WebSearchCall { .. }
            | ResponseItem::ImageGenerationCall { .. }
            | ResponseItem::CompactionTrigger
            | ResponseItem::Compaction { .. }
            | ResponseItem::ContextCompaction { .. }
            | ResponseItem::Other => {}
        }
    }
}

fn maybe_elide_tool_output(
    call_id: &str,
    output: &mut FunctionCallOutputPayload,
    seen: &mut HashMap<[u8; 20], SeenToolOutput>,
) {
    let FunctionCallOutputBody::Text(text) = &mut output.body else {
        return;
    };

    if text.starts_with(ELISION_PREFIX) || text.contains(RUNNING_PROCESS_MARKER) {
        return;
    }

    let canonical_text = canonical_text_for_duplicate_detection(text);
    let canonical_bytes = canonical_text.len();
    if canonical_bytes < LARGE_TOOL_OUTPUT_MIN_BYTES {
        return;
    }

    let hash = sha1_digest(canonical_text.as_bytes());
    if let Some(first) = seen.get(&hash) {
        let first_call_id = &first.call_id;
        let first_bytes = first.bytes;
        let hash_hex = sha1_hex(&hash);
        *text = format!(
            "{ELISION_PREFIX} It is identical to earlier tool output `{first_call_id}` (sha1: {hash_hex}, {first_bytes} bytes after normalization). Reuse that earlier output unless freshness matters."
        );
        return;
    }

    seen.insert(
        hash,
        SeenToolOutput {
            call_id: call_id.to_string(),
            bytes: canonical_bytes,
        },
    );
}

fn canonical_text_for_duplicate_detection(text: &str) -> String {
    if !text.lines().any(|line| line == "Output:") {
        return text.to_string();
    }

    let mut canonical = String::with_capacity(text.len());
    for line in text.lines() {
        if is_volatile_exec_metadata_line(line) {
            continue;
        }
        canonical.push_str(line);
        canonical.push('\n');
    }
    if !text.ends_with('\n') {
        canonical.pop();
    }
    canonical
}

fn is_volatile_exec_metadata_line(line: &str) -> bool {
    line.starts_with("Wall time:") || line.starts_with("Chunk ID:")
}

fn sha1_hex(hash: &[u8; 20]) -> String {
    let mut hex = String::with_capacity(hash.len() * 2);
    for byte in hash {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}
