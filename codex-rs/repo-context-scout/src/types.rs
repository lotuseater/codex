use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

pub type Result<T> = std::result::Result<T, ScoutError>;

#[derive(Debug, thiserror::Error)]
pub enum ScoutError {
    #[error("repo context scout I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("repo context scout walkdir error: {0}")]
    Walkdir(#[from] walkdir::Error),
    #[error("repo context scout JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid project root: {0}")]
    InvalidProjectRoot(PathBuf),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepoContextScoutMode {
    Off,
    #[default]
    Shadow,
    Tool,
}

impl RepoContextScoutMode {
    pub fn shadow_enabled(self) -> bool {
        matches!(self, Self::Shadow)
    }

    pub fn tool_enabled(self) -> bool {
        matches!(self, Self::Tool)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RepoContextScoutConfig {
    pub mode: RepoContextScoutMode,
    pub max_files: usize,
    pub max_file_bytes: u64,
    pub max_anchors_per_file: usize,
    pub max_output_tokens: usize,
    pub max_candidates: usize,
}

impl Default for RepoContextScoutConfig {
    fn default() -> Self {
        Self {
            mode: RepoContextScoutMode::Shadow,
            max_files: 5_000,
            max_file_bytes: 128 * 1024,
            max_anchors_per_file: 24,
            max_output_tokens: 1_200,
            max_candidates: 12,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoutCommandMode {
    #[default]
    Scout,
    Status,
    Refresh,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoutTrigger {
    #[default]
    Manual,
    FreshTurn,
    Resume,
    Clear,
    PostCompaction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexState {
    Cold,
    Warm,
    Stale,
}

#[derive(Debug, Clone, Copy)]
pub struct ScoutRequest<'a> {
    pub project_root: &'a Path,
    pub codex_home: &'a Path,
    pub prompt: &'a str,
    pub config: RepoContextScoutConfig,
    pub mode: ScoutCommandMode,
    pub trigger: ScoutTrigger,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anchor {
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRecord {
    pub path: String,
    pub size: u64,
    pub modified_unix: u64,
    pub language: String,
    pub line_count: usize,
    pub anchors: Vec<Anchor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedPath {
    pub path: String,
    pub status: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedAreas {
    pub paths: Vec<ChangedPath>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoIndex {
    pub schema_version: u32,
    pub project_root: PathBuf,
    pub generated_at_unix: u64,
    pub git_head: Option<String>,
    pub files: Vec<FileRecord>,
    pub file_limit_reached: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoutCandidate {
    pub path: String,
    pub score: f64,
    pub reasons: Vec<String>,
    pub anchors: Vec<Anchor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportRoute {
    pub name: String,
    pub reason: String,
    pub path_hints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoutBundle {
    pub repo_key: String,
    pub project_root: PathBuf,
    pub status: ScoutStatus,
    pub candidates: Vec<ScoutCandidate>,
    pub support_routes: Vec<SupportRoute>,
    pub packet_tokens: usize,
    pub packet: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoutStatus {
    pub repo_key: String,
    pub cache_dir: PathBuf,
    pub index_path: PathBuf,
    pub generated_at_unix: u64,
    pub index_state: IndexState,
    pub indexed_files: usize,
    pub git_head: Option<String>,
    pub changed_paths: Vec<ChangedPath>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShadowRecord {
    pub timestamp_unix: u64,
    pub trigger: ScoutTrigger,
    pub prompt_hash: String,
    pub repo_key: String,
    pub index_state: IndexState,
    pub selected_paths: Vec<String>,
    pub support_routes: Vec<SupportRoute>,
    pub packet_tokens: usize,
    pub changed_path_count: usize,
    pub fallback_reason: Option<String>,
    pub verdict: String,
}
