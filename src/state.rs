use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const STATE_FILENAME: &str = ".image-processor-state.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct TransferState {
    pub transfer_id: String,
    pub completed_files: HashSet<String>,
    pub total_files: usize,
    pub total_bytes: u64,
}

impl TransferState {
    pub fn new(total_files: usize, total_bytes: u64) -> Self {
        Self {
            transfer_id: uuid_v4(),
            completed_files: HashSet::new(),
            total_files,
            total_bytes,
        }
    }

    pub fn is_completed(&self, file_key: &str) -> bool {
        self.completed_files.contains(file_key)
    }

    pub fn mark_completed(&mut self, file_key: String) {
        self.completed_files.insert(file_key);
    }

    pub fn all_done(&self) -> bool {
        self.completed_files.len() >= self.total_files
    }
}

/// Generate a simple unique ID without pulling in the uuid crate.
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}-{:x}", d.as_secs(), d.subsec_nanos())
}

/// Build the canonical file key used to track completion.
/// Uses the relative path from the input directory.
pub fn file_key(file_path: &Path, input_dir: &Path) -> String {
    file_path
        .strip_prefix(input_dir)
        .unwrap_or(file_path)
        .to_string_lossy()
        .to_string()
}

fn state_path(dir: &Path) -> PathBuf {
    dir.join(STATE_FILENAME)
}

/// Try to load an existing state file from the output directory, falling back to the input directory.
pub fn load_state(input_dir: &Path, output_dir: &Path) -> Option<TransferState> {
    // Prefer output dir state (always writable)
    load_from(output_dir).or_else(|| load_from(input_dir))
}

fn load_from(dir: &Path) -> Option<TransferState> {
    let path = state_path(dir);
    let data = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Write state atomically to a directory. Returns Ok(()) even if the write
/// fails on a read-only filesystem (best-effort for input/SD card).
pub fn save_state(state: &TransferState, dir: &Path, best_effort: bool) -> Result<()> {
    let target = state_path(dir);
    let tmp = dir.join(format!(".image-processor-state.tmp.{}", std::process::id()));
    let data = serde_json::to_string_pretty(state)?;

    match fs::write(&tmp, &data) {
        Ok(()) => {
            fs::rename(&tmp, &target)?;
            Ok(())
        }
        Err(e) if best_effort => {
            // Silently ignore write failures on read-only media
            let _ = fs::remove_file(&tmp);
            eprintln!(
                "Warning: could not write state to {}: {}",
                dir.display(),
                e
            );
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// Save state to both input (best-effort) and output (required) directories.
pub fn save_state_both(state: &TransferState, input_dir: &Path, output_dir: &Path) -> Result<()> {
    save_state(state, output_dir, false)?;
    save_state(state, input_dir, true)?;
    Ok(())
}

/// Remove state files from both directories after a successful transfer.
pub fn cleanup_state(input_dir: &Path, output_dir: &Path) {
    let _ = fs::remove_file(state_path(output_dir));
    let _ = fs::remove_file(state_path(input_dir));
}
