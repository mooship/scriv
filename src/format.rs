//! Formatting helpers shared by CLI commands.

use chrono::{DateTime, Utc};
use std::io::{BufRead, BufReader, Read};

/// Read piped stdin as trimmed multi-line text.
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
pub fn highlight_match(text: &str, query: &str) -> String {
    if query.is_empty() {
        return text.to_string();
    }

    let lower = text.to_lowercase();
    let lower_q = query.to_lowercase();

    let mut result = String::new();
    let mut i = 0;

    while let Some(idx) = lower[i..].find(&lower_q) {
        let start = i + idx;
        let end = start + lower_q.len();

        result.push_str(&text[i..start]);
        result.push_str("\x1b[1;33m");
        result.push_str(&text[start..end]);
        result.push_str("\x1b[0m");

        i = end;
    }

    result.push_str(&text[i..]);
    result
}
