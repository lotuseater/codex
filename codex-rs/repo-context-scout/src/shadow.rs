use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use crate::types::Result;
use crate::types::ShadowRecord;

pub(crate) fn append_shadow_record(path: &Path, record: &ShadowRecord) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, record)?;
    file.write_all(b"\n")?;
    Ok(())
}
