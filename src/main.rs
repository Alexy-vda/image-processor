mod cli;
mod metadata;
mod scanner;
mod session;
mod state;
mod transfer;

use anyhow::Result;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

fn main() -> Result<()> {
    let args = cli::Args::parse();

    // Validate paths
    if !args.input.exists() {
        anyhow::bail!("Input directory does not exist: {}", args.input.display());
    }
    if !args.input.is_dir() {
        anyhow::bail!("Input path is not a directory: {}", args.input.display());
    }

    // Scan for CR2/MP4 files
    println!("Scanning {}...", args.input.display());
    let scanned = scanner::scan_files(&args.input)?;
    if scanned.is_empty() {
        println!("No CR2/MP4 files found.");
        return Ok(());
    }
    println!("Found {} files", scanned.len());

    // Extract metadata (datetime) for each file
    let pb = ProgressBar::new(scanned.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("Reading metadata {pos}/{len} {wide_bar} {msg}")?
            .progress_chars("=> "),
    );
    let mut dated_files: Vec<session::DatedFile> = Vec::with_capacity(scanned.len());
    for file in &scanned {
        let file_name = file
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        pb.set_message(file_name.to_string());
        let datetime = match metadata::extract_datetime(&file.path) {
            Ok(dt) => dt,
            Err(e) => {
                pb.suspend(|| {
                    eprintln!(
                        "Warning: could not read date from {}: {}",
                        file.path.display(),
                        e
                    );
                });
                pb.inc(1);
                continue;
            }
        };
        dated_files.push(session::DatedFile {
            path: file.path.clone(),
            datetime,
            sequence_number: file.sequence_number,
        });
        pb.inc(1);
    }
    pb.finish_and_clear();

    if dated_files.is_empty() {
        println!("No files with readable dates found.");
        return Ok(());
    }

    // Group into sessions
    let sessions = session::group_into_sessions(dated_files, args.gap_hours);
    println!("Organized into {} session(s):", sessions.len());
    for session in &sessions {
        println!(
            "  {} ({} files)",
            session.folder_name,
            session.files.len()
        );
    }

    if args.dry_run {
        println!("\n[dry-run] No files will be copied.");
    }

    // Prepare output directory
    if !args.dry_run {
        std::fs::create_dir_all(&args.output)?;
    }

    // Load or create transfer state
    let total_files = sessions.iter().map(|s| s.files.len()).sum::<usize>();
    let total_bytes: u64 = sessions
        .iter()
        .flat_map(|s| &s.files)
        .filter_map(|f| std::fs::metadata(&f.path).ok())
        .map(|m| m.len())
        .sum();

    let mut transfer_state = if !args.dry_run {
        match state::load_state(&args.input, &args.output) {
            Some(existing) => {
                let skipped = existing.completed_files.len();
                if skipped > 0 {
                    println!("Resuming transfer: {}/{} files already copied", skipped, total_files);
                }
                existing
            }
            None => state::TransferState::new(total_files, total_bytes),
        }
    } else {
        state::TransferState::new(total_files, total_bytes)
    };

    // Transfer files
    transfer::transfer_sessions(&sessions, &args.output, &args.input, &mut transfer_state, args.dry_run)?;

    // Cleanup state files on successful completion
    if !args.dry_run && transfer_state.all_done() {
        state::cleanup_state(&args.input, &args.output);
        println!("State files cleaned up.");
    }

    println!("Done.");
    Ok(())
}
