//! Stage-1 structured routing metadata helpers.
//!
//! The fork extends phase-1 extraction with an optional [`Stage1MemoryMetadata`]
//! payload used for project/problem recall. This module owns the fork-specific
//! schema fragment and secret-redaction pipeline for that payload so the volatile
//! phase-1 output plumbing stays small and easy to merge against upstream.

use codex_secrets::redact_secrets;
use codex_state::Stage1MemoryMetadata;
use serde_json::Value;
use serde_json::json;

/// JSON schema fragment describing the optional stage-1 routing metadata object.
///
/// Embedded into the phase-1 `output_schema()` under the `metadata` property.
pub(crate) fn metadata_schema_fragment() -> Value {
    json!({
        "type": "object",
        "properties": {
            "project_key": { "type": ["string", "null"] },
            "problem_families": { "type": "array", "items": { "type": "string" } },
            "symptoms": { "type": "array", "items": { "type": "string" } },
            "edit_surfaces": { "type": "array", "items": { "type": "string" } },
            "verified_commands": { "type": "array", "items": { "type": "string" } },
            "failure_modes": { "type": "array", "items": { "type": "string" } },
            "routing_keywords": { "type": "array", "items": { "type": "string" } },
            "staleness_notes": { "type": "array", "items": { "type": "string" } }
        },
        "required": [
            "project_key",
            "problem_families",
            "symptoms",
            "edit_surfaces",
            "verified_commands",
            "failure_modes",
            "routing_keywords",
            "staleness_notes"
        ],
        "additionalProperties": false
    })
}

/// Redacts secrets from every string field of stage-1 routing metadata before
/// persistence.
pub(crate) fn redact_stage_one_metadata(metadata: Stage1MemoryMetadata) -> Stage1MemoryMetadata {
    Stage1MemoryMetadata {
        project_key: metadata.project_key.map(redact_secrets),
        problem_families: redact_metadata_strings(metadata.problem_families),
        symptoms: redact_metadata_strings(metadata.symptoms),
        edit_surfaces: redact_metadata_strings(metadata.edit_surfaces),
        verified_commands: redact_metadata_strings(metadata.verified_commands),
        failure_modes: redact_metadata_strings(metadata.failure_modes),
        routing_keywords: redact_metadata_strings(metadata.routing_keywords),
        staleness_notes: redact_metadata_strings(metadata.staleness_notes),
    }
}

fn redact_metadata_strings(values: Vec<String>) -> Vec<String> {
    values.into_iter().map(redact_secrets).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_one_metadata_is_redacted_before_persistence() {
        let metadata = Stage1MemoryMetadata {
            project_key: Some("token=abcdef1234567890".to_string()),
            symptoms: vec!["observed sk-abcdefghijklmnopqrstuvwxyz123456".to_string()],
            verified_commands: vec![
                "curl -H 'Authorization: Bearer abcdefghijklmnop'".to_string(),
            ],
            routing_keywords: vec!["password=hunter222222".to_string()],
            ..Stage1MemoryMetadata::default()
        };

        let redacted = redact_stage_one_metadata(metadata);

        assert_eq!(
            redacted.project_key.as_deref(),
            Some("token=[REDACTED_SECRET]")
        );
        assert_eq!(redacted.symptoms, vec!["observed [REDACTED_SECRET]"]);
        assert_eq!(
            redacted.verified_commands,
            vec!["curl -H 'Authorization: Bearer [REDACTED_SECRET]'"]
        );
        assert_eq!(redacted.routing_keywords, vec!["password=[REDACTED_SECRET]"]);
    }
}
