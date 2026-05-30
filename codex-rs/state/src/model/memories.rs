use chrono::DateTime;
use chrono::Utc;
use codex_protocol::ThreadId;
use serde::Deserialize;
use serde::Serialize;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;
use std::path::PathBuf;

use super::ThreadMetadata;

/// Optional structured routing metadata extracted with a stage-1 memory.
///
/// Consumers should treat this as routing evidence, not authority.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage1MemoryMetadata {
    #[serde(default)]
    pub project_key: Option<String>,
    #[serde(default)]
    pub problem_families: Vec<String>,
    #[serde(default)]
    pub symptoms: Vec<String>,
    #[serde(default)]
    pub edit_surfaces: Vec<String>,
    #[serde(default)]
    pub verified_commands: Vec<String>,
    #[serde(default)]
    pub failure_modes: Vec<String>,
    #[serde(default)]
    pub routing_keywords: Vec<String>,
    #[serde(default)]
    pub staleness_notes: Vec<String>,
}

impl Stage1MemoryMetadata {
    pub fn from_json_str(value: &str) -> Result<Self> {
        if value.trim().is_empty() {
            return Ok(Self::default());
        }

        Ok(serde_json::from_str(value)?)
    }

    pub fn to_json_string(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    pub fn has_signal(&self) -> bool {
        self.project_key
            .as_deref()
            .is_some_and(|key| !key.trim().is_empty())
            || !self.problem_families.is_empty()
            || !self.symptoms.is_empty()
            || !self.edit_surfaces.is_empty()
            || !self.verified_commands.is_empty()
            || !self.failure_modes.is_empty()
            || !self.routing_keywords.is_empty()
            || !self.staleness_notes.is_empty()
    }
}

/// Stored stage-1 memory extraction output for a single thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage1Output {
    pub thread_id: ThreadId,
    pub rollout_path: PathBuf,
    pub source_updated_at: DateTime<Utc>,
    pub raw_memory: String,
    pub rollout_summary: String,
    pub rollout_slug: Option<String>,
    pub metadata: Stage1MemoryMetadata,
    pub cwd: PathBuf,
    pub git_branch: Option<String>,
    pub generated_at: DateTime<Utc>,
}

/// Read-only aggregate counts for the local memory pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryStatusSnapshot {
    pub stage1_output_count: u64,
    pub selected_for_phase2_count: u64,
    pub latest_source_updated_at: Option<i64>,
    pub latest_generated_at: Option<i64>,
    pub jobs: Vec<MemoryJobStatusCount>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryJobStatusCount {
    pub kind: String,
    pub status: String,
    pub count: u64,
}

#[derive(Debug)]
pub(crate) struct MemoryJobStatusCountRow {
    kind: String,
    status: String,
    count: i64,
}

impl MemoryJobStatusCountRow {
    pub(crate) fn try_from_row(row: &SqliteRow) -> Result<Self> {
        Ok(Self {
            kind: row.try_get("kind")?,
            status: row.try_get("status")?,
            count: row.try_get("count")?,
        })
    }
}

impl From<MemoryJobStatusCountRow> for MemoryJobStatusCount {
    fn from(row: MemoryJobStatusCountRow) -> Self {
        Self {
            kind: row.kind,
            status: row.status,
            count: u64::try_from(row.count).unwrap_or(0),
        }
    }
}

/// Result of trying to claim a stage-1 memory extraction job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage1JobClaimOutcome {
    /// The caller owns the job and should continue with extraction.
    Claimed { ownership_token: String },
    /// Existing output is already newer than or equal to the source rollout.
    SkippedUpToDate,
    /// Another worker currently owns a fresh lease for this job.
    SkippedRunning,
    /// The job is in backoff and should not be retried yet.
    SkippedRetryBackoff,
    /// The job has exhausted retries and should not be retried automatically.
    SkippedRetryExhausted,
}

/// Claimed stage-1 job with thread metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage1JobClaim {
    pub thread: ThreadMetadata,
    pub ownership_token: String,
}

#[derive(Debug, Clone, Copy)]
pub struct Stage1StartupClaimParams<'a> {
    pub scan_limit: usize,
    pub max_claimed: usize,
    pub max_age_days: i64,
    pub min_rollout_idle_hours: i64,
    pub allowed_sources: &'a [String],
    pub lease_seconds: i64,
}

/// Result of trying to claim a phase-2 consolidation job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase2JobClaimOutcome {
    /// The caller owns the global lock and may inspect the memory workspace.
    Claimed {
        ownership_token: String,
        /// Snapshot of `input_watermark` at claim time.
        input_watermark: i64,
    },
    /// The global job is in retry backoff.
    SkippedRetryUnavailable,
    /// The global job completed recently enough that consolidation is cooling down.
    SkippedCooldown,
    /// Another worker currently owns a fresh global consolidation lease.
    SkippedRunning,
}
