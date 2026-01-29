use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub sequence_number: Option<u64>,
}

pub fn scan_files(input_dir: &Path) -> Result<Vec<ScannedFile>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(input_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());

        match ext.as_deref() {
            Some("cr2") | Some("mp4") => {}
            _ => continue,
        }

        let sequence_number = extract_sequence_number(path);
        files.push(ScannedFile {
            path: path.to_path_buf(),
            sequence_number,
        });
    }

    // Sort by sequence number, files without a sequence number go last
    files.sort_by(|a, b| {
        let sa = a.sequence_number.unwrap_or(u64::MAX);
        let sb = b.sequence_number.unwrap_or(u64::MAX);
        sa.cmp(&sb)
    });

    Ok(files)
}

/// Extract the trailing digits from the file stem as a sequence number.
/// Examples:
///   _MG_1001.CR2  -> 1001
///   IMG_0042.CR2   -> 42
///   MVI_0042.MP4   -> 42
fn extract_sequence_number(path: &Path) -> Option<u64> {
    let stem = path.file_stem()?.to_str()?;
    let digits: String = stem.chars().rev().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let digits: String = digits.chars().rev().collect();
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_extract_sequence_number() {
        assert_eq!(
            extract_sequence_number(Path::new("_MG_1001.CR2")),
            Some(1001)
        );
        assert_eq!(
            extract_sequence_number(Path::new("IMG_0042.CR2")),
            Some(42)
        );
        assert_eq!(
            extract_sequence_number(Path::new("MVI_0042.MP4")),
            Some(42)
        );
        assert_eq!(extract_sequence_number(Path::new("nodigits.CR2")), None);
    }
}
