use chrono::NaiveDateTime;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DatedFile {
    pub path: PathBuf,
    pub datetime: NaiveDateTime,
    pub sequence_number: Option<u64>,
}

#[derive(Debug)]
pub struct Session {
    pub folder_name: String,
    pub files: Vec<DatedFile>,
}

/// Group sorted files into sessions based on the gap threshold.
/// Files must be pre-sorted by sequence number.
pub fn group_into_sessions(files: Vec<DatedFile>, gap_hours: f64) -> Vec<Session> {
    if files.is_empty() {
        return Vec::new();
    }

    let gap_seconds = (gap_hours * 3600.0) as i64;
    let mut sessions: Vec<Vec<DatedFile>> = Vec::new();
    let mut current_session: Vec<DatedFile> = vec![files[0].clone()];

    for file in files.into_iter().skip(1) {
        let prev = current_session.last().unwrap();
        let diff = (file.datetime - prev.datetime).num_seconds().abs();

        if diff > gap_seconds {
            sessions.push(std::mem::take(&mut current_session));
        }
        current_session.push(file);
    }
    if !current_session.is_empty() {
        sessions.push(current_session);
    }

    name_sessions(sessions)
}

/// Assign folder names to sessions.
/// Single session on a day: "2024-01-15"
/// Multiple sessions on same day: "2024-01-15_a", "2024-01-15_b", etc.
fn name_sessions(sessions: Vec<Vec<DatedFile>>) -> Vec<Session> {
    // Group sessions by date to detect collisions
    let mut date_counts: HashMap<String, usize> = HashMap::new();
    let session_dates: Vec<String> = sessions
        .iter()
        .map(|s| {
            let first = &s[0];
            first.datetime.format("%Y-%m-%d").to_string()
        })
        .collect();

    for date in &session_dates {
        *date_counts.entry(date.clone()).or_insert(0) += 1;
    }

    // Track how many times we've seen each date so far for suffix assignment
    let mut date_seen: HashMap<String, usize> = HashMap::new();

    sessions
        .into_iter()
        .zip(session_dates.iter())
        .map(|(files, date)| {
            let count = date_counts[date];
            let folder_name = if count == 1 {
                date.clone()
            } else {
                let idx = date_seen.entry(date.clone()).or_insert(0);
                let suffix = (b'a' + *idx as u8) as char;
                *idx += 1;
                format!("{}_{}", date, suffix)
            };

            Session { folder_name, files }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_file(hour: u32, seq: u64) -> DatedFile {
        DatedFile {
            path: PathBuf::from(format!("IMG_{:04}.CR2", seq)),
            datetime: NaiveDate::from_ymd_opt(2024, 1, 15)
                .unwrap()
                .and_hms_opt(hour, 0, 0)
                .unwrap(),
            sequence_number: Some(seq),
        }
    }

    #[test]
    fn test_single_session() {
        let files = vec![make_file(10, 1), make_file(11, 2), make_file(12, 3)];
        let sessions = group_into_sessions(files, 6.0);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].folder_name, "2024-01-15");
    }

    #[test]
    fn test_two_sessions_same_day() {
        let files = vec![
            make_file(8, 1),
            make_file(9, 2),
            // 7 hour gap
            make_file(16, 3),
            make_file(17, 4),
        ];
        let sessions = group_into_sessions(files, 6.0);
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].folder_name, "2024-01-15_a");
        assert_eq!(sessions[1].folder_name, "2024-01-15_b");
    }
}
