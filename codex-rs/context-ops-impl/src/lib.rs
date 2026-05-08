mod file_outline;
mod search_text;

use std::error::Error;
use std::fmt;

pub use file_outline::DEFAULT_MAX_OUTLINE_ITEMS;
pub use file_outline::file_outline;
pub use file_outline::file_outline_from_bytes;
pub use search_text::DEFAULT_MAX_FILES;
pub use search_text::DEFAULT_MAX_MATCHES_PER_FILE;
pub use search_text::clamp_max_files;
pub use search_text::clamp_max_matches_per_file;
pub use search_text::combined_globs;
pub use search_text::rg_args;
pub use search_text::search_text;
pub use search_text::search_text_from_rg_json_output;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextOpsError {
    message: String,
}

impl ContextOpsError {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ContextOpsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl Error for ContextOpsError {}
