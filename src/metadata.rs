use anyhow::Result;
use chrono::NaiveDateTime;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Extract the creation datetime from a file.
/// Tries EXIF for CR2, mvhd for MP4, falls back to filesystem modified time.
pub fn extract_datetime(path: &Path) -> Result<NaiveDateTime> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());

    let result = match ext.as_deref() {
        Some("cr2") => extract_exif_datetime(path),
        Some("mp4") => extract_mp4_datetime(path),
        _ => Err(anyhow::anyhow!("Unsupported file type")),
    };

    match result {
        Ok(dt) => Ok(dt),
        Err(_) => extract_filesystem_datetime(path),
    }
}

fn extract_exif_datetime(path: &Path) -> Result<NaiveDateTime> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader)?;

    // Try DateTimeOriginal first, then DateTime
    let field = exif
        .get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
        .or_else(|| exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY))
        .ok_or_else(|| anyhow::anyhow!("No EXIF datetime field found"))?;

    let value = field.display_value().to_string();
    // EXIF format: "2024-01-15 14:30:00"
    let dt = NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S")?;
    Ok(dt)
}

fn extract_mp4_datetime(path: &Path) -> Result<NaiveDateTime> {
    let file = File::open(path)?;
    let size = file.metadata()?.len();
    let reader = BufReader::new(file);
    let mp4_file = mp4::Mp4Reader::read_header(reader, size)?;

    // MP4 creation_time is seconds since 1904-01-01 00:00:00 UTC
    let creation_time = mp4_file.moov.mvhd.creation_time;
    if creation_time == 0 {
        return Err(anyhow::anyhow!("MP4 creation_time is 0"));
    }

    // MP4 epoch: 1904-01-01 00:00:00 UTC
    let mp4_epoch = NaiveDateTime::parse_from_str("1904-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")?;
    let dt = mp4_epoch + chrono::Duration::seconds(creation_time as i64);
    Ok(dt)
}

fn extract_filesystem_datetime(path: &Path) -> Result<NaiveDateTime> {
    let metadata = std::fs::metadata(path)?;
    let modified = metadata.modified()?;
    let datetime: chrono::DateTime<chrono::Local> = modified.into();
    Ok(datetime.naive_local())
}
