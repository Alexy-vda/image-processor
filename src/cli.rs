use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "image-processor")]
#[command(about = "Copy CR2/MP4 files from SD card to destination, organized by shooting session")]
pub struct Args {
    /// Input directory (e.g. SD card mount point)
    #[arg(short, long)]
    pub input: PathBuf,

    /// Output directory where session folders will be created
    #[arg(short, long)]
    pub output: PathBuf,

    /// Minimum gap in hours between consecutive files to start a new session
    #[arg(long, default_value_t = 6.0)]
    pub gap_hours: f64,

    /// Show what would be done without actually copying files
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}
