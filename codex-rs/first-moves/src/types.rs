use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, FirstMovesError>;

#[derive(Debug, thiserror::Error)]
pub enum FirstMovesError {
    #[error("first-moves I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("first-moves sqlite error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("first-moves json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstMovesMode {
    #[default]
    Auto,
    SuggestOnly,
    Prewarm,
    Off,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstMovesPrewarm {
    Off,
    #[default]
    HighConfidenceOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirstMovesConfig {
    pub mode: FirstMovesMode,
    pub inject_context: bool,
    pub prewarm: FirstMovesPrewarm,
    pub max_candidates: usize,
    pub max_context_moves: usize,
    pub max_prewarm_files: usize,
    pub min_context_score: f64,
    pub min_prewarm_score: f64,
    pub max_scan_files: usize,
    pub max_scan_depth: usize,
    pub max_read_bytes: usize,
}

impl Default for FirstMovesConfig {
    fn default() -> Self {
        Self {
            mode: FirstMovesMode::Auto,
            inject_context: true,
            prewarm: FirstMovesPrewarm::HighConfidenceOnly,
            max_candidates: 14,
            max_context_moves: 8,
            max_prewarm_files: 2,
            min_context_score: 0.55,
            min_prewarm_score: 0.82,
            max_scan_files: 2_000,
            max_scan_depth: 6,
            max_read_bytes: 8 * 1024,
        }
    }
}

impl FirstMovesConfig {
    pub fn enabled(&self) -> bool {
        !matches!(self.mode, FirstMovesMode::Off)
    }

    pub fn prewarm_enabled(&self) -> bool {
        matches!(self.mode, FirstMovesMode::Auto | FirstMovesMode::Prewarm)
            && !matches!(self.prewarm, FirstMovesPrewarm::Off)
            && self.max_prewarm_files > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstMoveKind {
    Read,
    Search,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirstMove {
    pub kind: FirstMoveKind,
    pub confidence: f64,
    pub reason: String,
    pub source_layer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirstMovesStorage {
    pub repo_key: String,
    pub system_db: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo_db: Option<PathBuf>,
    pub repo_db_exists: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirstMovesBundle {
    pub confidence: f64,
    pub intent: String,
    pub project_root: PathBuf,
    pub repo_key: String,
    pub storage: FirstMovesStorage,
    pub moves: Vec<FirstMove>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirstMovesStats {
    pub repo_key: String,
    pub storage: FirstMovesStorage,
    pub prediction_rows: i64,
    pub predicted_path_rows: i64,
    pub hit_count: i64,
    pub hit_path_rows: i64,
    pub learned_path_rows: i64,
}

pub struct PredictRequest<'a> {
    pub project_root: &'a Path,
    pub codex_home: &'a Path,
    pub prompt: &'a str,
    pub session_id: Option<&'a str>,
    pub config: FirstMovesConfig,
    pub already_loaded_paths: Vec<PathBuf>,
    pub record_prediction: bool,
}

pub struct ToolUseHitRequest<'a> {
    pub project_root: &'a Path,
    pub codex_home: &'a Path,
    pub tool_name: &'a str,
    pub tool_input: &'a str,
}
