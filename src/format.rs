//! Formatting helpers shared by CLI commands.

use chrono::{DateTime, Utc};
use std::io::{BufRead, BufReader, Read};

const MAX_STDIN_BYTES: usize = 10 * 1024 * 1024;

/// Read piped stdin as trimmed multi-line text (capped at 10 MB).
pub fn read_stdin_text<R: Read>(reader: R) -> Result<String, String> {
    let mut out = String::new();
    let mut br = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let read = br.read_line(&mut line).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        if out.len() + line.len() > MAX_STDIN_BYTES {
            return Err("stdin input exceeds 10 MB limit".to_string());
        }
        out.push_str(line.trim_end_matches('\n'));
        out.push('\n');
    }

    let trimmed = out.trim().to_string();
    if trimmed.is_empty() {
        return Err("no text provided via stdin".to_string());
    }
    Ok(trimmed)
}

/// Convert an RFC3339 UTC timestamp into a compact relative-age label.
pub fn note_age(ts: &str) -> String {
    let parsed = DateTime::parse_from_rfc3339(ts);
    let t = match parsed {
        Ok(v) => v.with_timezone(&Utc),
        Err(_) => return "?".to_string(),
    };

    let d = Utc::now().signed_duration_since(t);

    if d.num_hours() < 1 {
        "<1h".to_string()
    } else if d.num_hours() < 24 {
        format!("{}h", d.num_hours())
    } else if d.num_hours() < 24 * 14 {
        format!("{}d", d.num_days())
    } else if d.num_hours() < 24 * 56 {
        format!("{}w", d.num_weeks())
    } else if d.num_hours() < 24 * 730 {
        format!("{}mo", d.num_days() / 30)
    } else {
        format!("{}y", d.num_days() / 365)
    }
}

/// ANSI-highlight all case-insensitive matches of `query` inside `text`.
///
/// Uses character-level indexing to handle multi-byte and case-folding
/// differences safely (e.g. where `to_lowercase()` changes byte length).
pub fn highlight_match(text: &str, query: &str) -> String {
    if query.is_empty() {
        return text.to_string();
    }

    let lower_q = query.to_lowercase();
    let q_chars: Vec<char> = lower_q.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    let lower_chars: Vec<char> = text.to_lowercase().chars().collect();

    if q_chars.len() > lower_chars.len() {
        return text.to_string();
    }

    let mut result = String::new();
    let mut i = 0;

    while i <= lower_chars.len() - q_chars.len() {
        if lower_chars[i..i + q_chars.len()] == q_chars[..] {
            result.push_str("\x1b[1;33m");
            for ch in &text_chars[i..i + q_chars.len()] {
                result.push(*ch);
            }
            result.push_str("\x1b[0m");
            i += q_chars.len();
        } else {
            result.push(text_chars[i]);
            i += 1;
        }
    }

    for ch in &text_chars[i..] {
        result.push(*ch);
    }

    result
}
