use crate::session::Session;
use crate::state::{self, TransferState};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

const BUFFER_SIZE: usize = 256 * 1024; // 256 KB

pub fn transfer_sessions(
    sessions: &[Session],
    output_dir: &Path,
    input_dir: &Path,
    state: &mut TransferState,
    dry_run: bool,
) -> Result<()> {
    let total_bytes: u64 = sessions
        .iter()
        .flat_map(|s| &s.files)
        .filter(|f| !state.is_completed(&state::file_key(&f.path, input_dir)))
        .filter_map(|f| fs::metadata(&f.path).ok())
        .map(|m| m.len())
        .sum();

    let pb = ProgressBar::new(total_bytes);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{wide_bar} {percent}% {bytes}/{total_bytes} [{eta}]")?
            .progress_chars("=> "),
    );

    for session in sessions {
        let session_dir = output_dir.join(&session.folder_name);

        if !dry_run {
            fs::create_dir_all(&session_dir)?;
        }

        for file in &session.files {
            let key = state::file_key(&file.path, input_dir);

            if state.is_completed(&key) {
                // Already copied in a previous run, skip but count the bytes
                if let Ok(meta) = fs::metadata(&file.path) {
                    pb.inc(meta.len());
                }
                continue;
            }

            let file_name = file
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            let dest = session_dir.join(&*file_name);

            pb.set_message(format!(
                "{}/{}",
                session.folder_name,
                file_name
            ));

            if dry_run {
                println!(
                    "[dry-run] {} -> {}",
                    file.path.display(),
                    dest.display()
                );
                if let Ok(meta) = fs::metadata(&file.path) {
                    pb.inc(meta.len());
                }
            } else {
                copy_with_progress(&file.path, &dest, &pb)?;
                state.mark_completed(key);
                state::save_state_both(state, input_dir, output_dir)?;
            }
        }
    }

    pb.finish_with_message("Transfer complete");
    Ok(())
}

fn copy_with_progress(src: &Path, dest: &Path, pb: &ProgressBar) -> Result<()> {
    let mut source = fs::File::open(src)?;
    let mut destination = fs::File::create(dest)?;
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        let bytes_read = source.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        destination.write_all(&buffer[..bytes_read])?;
        pb.inc(bytes_read as u64);
    }

    // Preserve modified time
    if let Ok(meta) = fs::metadata(src) {
        if let Ok(mtime) = meta.modified() {
            let _ = filetime_set(dest, mtime);
        }
    }

    Ok(())
}

fn filetime_set(path: &Path, mtime: std::time::SystemTime) -> Result<()> {
    // Use file's set_modified via a re-opened handle
    let file = fs::OpenOptions::new().write(true).open(path)?;
    file.set_modified(mtime)?;
    Ok(())
}
