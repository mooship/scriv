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
        out.push_str(line.trim_end_matches(['\n', '\r']));
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
/// Builds a byte-offset mapping between the lowercased and original strings
/// so that case-folding length changes (e.g. German ß, Turkish İ) cannot
/// cause panics.
pub fn highlight_match(text: &str, query: &str) -> String {
    if query.is_empty() {
        return text.to_string();
    }

    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();

    let mut lower_to_orig_start: Vec<usize> = Vec::with_capacity(lower_text.len() + 1);
    let mut lower_to_orig_end: Vec<usize> = Vec::with_capacity(lower_text.len() + 1);
    for (orig_pos, oc) in text.char_indices() {
        let orig_char_end = orig_pos + oc.len_utf8();
        let lc_len: usize = oc.to_lowercase().map(|c| c.len_utf8()).sum();
        for _ in 0..lc_len {
            lower_to_orig_start.push(orig_pos);
            lower_to_orig_end.push(orig_char_end);
        }
    }
    lower_to_orig_start.push(text.len());
    lower_to_orig_end.push(text.len());

    let mut result = String::new();
    let mut prev_orig_end: usize = 0;

    for (low_start, _) in lower_text.match_indices(&lower_query) {
        let low_end = low_start + lower_query.len();
        let orig_start = lower_to_orig_start[low_start];
        let orig_end = if low_end > 0 {
            lower_to_orig_end[low_end - 1]
        } else {
            lower_to_orig_start[0]
        };

        if orig_start < prev_orig_end || orig_start >= orig_end {
            continue;
        }

        result.push_str(&text[prev_orig_end..orig_start]);
        result.push_str("\x1b[1;33m");
        result.push_str(&text[orig_start..orig_end]);
        result.push_str("\x1b[0m");
        prev_orig_end = orig_end;
    }

    result.push_str(&text[prev_orig_end..]);
    result
}
