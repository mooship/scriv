//! Business operations over notes.

use crate::model::{ListOptions, Note};
use crate::storage::{load_notes, save_notes};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read};

/// Maximum allowed byte length for note text.
pub const MAX_NOTE_BYTES: usize = 1_048_576;

/// Maximum bytes accepted from an import stream.
pub const MAX_IMPORT_BYTES: u64 = 50 * 1024 * 1024;

/// Parse a required positive note id from a string.
///
/// Rejects non-integers and `0`, which is never a valid assigned id.
pub fn parse_id(s: &str) -> Result<u64, String> {
    let id = s
        .parse::<u64>()
        .map_err(|_| "id must be a positive integer".to_string())?;
    if id == 0 {
        return Err("id must be a positive integer".to_string());
    }
    Ok(id)
}

/// Parse NDJSON note records from a reader, validating each record and
/// enforcing the import size limit.
///
/// Blank lines are skipped. Parse and validation errors are prefixed with the
/// 1-based source line number. The returned vector may be empty when the input
/// contains no note records.
pub fn parse_import_ndjson<R: Read>(reader: R) -> Result<Vec<Note>, String> {
    let mut incoming = Vec::<Note>::new();
    let br = BufReader::new(reader.take(MAX_IMPORT_BYTES + 1));
    let mut total_bytes: u64 = 0;

    for (idx, line) in br.lines().enumerate() {
        let line = line.map_err(|e| e.to_string())?;
        total_bytes += line.len() as u64 + 1;
        if total_bytes > MAX_IMPORT_BYTES {
            return Err("import input exceeds 50 MB limit".to_string());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let note: Note = serde_json::from_str(trimmed)
            .map_err(|e| format!("line {}: invalid JSON: {}", idx + 1, e))?;
        validate_note(&note).map_err(|e| format!("line {}: {}", idx + 1, e))?;
        incoming.push(note);
    }

    Ok(incoming)
}

/// Validate user-supplied note text: non-empty (ignoring surrounding
/// whitespace) and within the size limit.
fn validate_text(text: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("note text cannot be empty".to_string());
    }
    if text.len() > MAX_NOTE_BYTES {
        return Err("note text exceeds 1 MB limit".to_string());
    }
    Ok(())
}

/// Validate a whole note record, enforcing the same invariants for every
/// entry path (creation, editing, and import).
pub fn validate_note(note: &Note) -> Result<(), String> {
    validate_text(&note.text)?;
    if note.created_at.is_empty() || DateTime::parse_from_rfc3339(&note.created_at).is_err() {
        return Err("invalid created_at timestamp".to_string());
    }
    if !note.updated_at.is_empty() && DateTime::parse_from_rfc3339(&note.updated_at).is_err() {
        return Err("invalid updated_at timestamp".to_string());
    }
    for tag in &note.tags {
        if tag.trim().is_empty() {
            return Err("tag cannot be empty".to_string());
        }
    }
    Ok(())
}

/// Case-insensitive tag equality.
fn tag_matches(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// Current UTC timestamp in RFC3339 format used by persisted note fields.
fn now_timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Standard "not found" error message for a missing note id.
fn note_not_found(id: u64) -> String {
    format!("no note with id {}", id)
}

/// Compute the next id after `max_id`, erroring instead of wrapping on overflow.
fn next_id(max_id: u64) -> Result<u64, String> {
    max_id
        .checked_add(1)
        .ok_or_else(|| "note id limit reached".to_string())
}

/// Load a note by id, apply `f`, and persist if `f` returns `true`.
///
/// When `f` returns `true` the note's `updated_at` is set before saving.
/// When `f` returns `false` the note is returned unchanged (no I/O).
fn modify_note<F>(id: u64, f: F) -> Result<Note, String>
where
    F: FnOnce(&mut Note) -> Result<bool, String>,
{
    let mut notes = load_notes()?;
    if let Some(note) = notes.iter_mut().find(|n| n.id == id) {
        let changed = f(note)?;
        if changed {
            note.updated_at = now_timestamp();
        }
        let out = note.clone();
        if changed {
            save_notes(&notes)?;
        }
        return Ok(out);
    }
    Err(note_not_found(id))
}

/// Create and persist a new note with `max(existing_id) + 1` semantics.
pub fn add_note(text: &str) -> Result<Note, String> {
    validate_text(text)?;
    let mut notes = load_notes()?;
    let max_id = notes.iter().map(|n| n.id).max().unwrap_or(0);
    let note = Note {
        id: next_id(max_id)?,
        text: text.to_string(),
        created_at: now_timestamp(),
        updated_at: String::new(),
        tags: Vec::new(),
    };
    notes.push(note.clone());
    save_notes(&notes)?;
    Ok(note)
}

/// Remove a single note by id.
pub fn remove_note(id: u64) -> Result<Note, String> {
    let mut notes = load_notes()?;
    if let Some(pos) = notes.iter().position(|n| n.id == id) {
        let note = notes.remove(pos);
        save_notes(&notes)?;
        return Ok(note);
    }
    Err(note_not_found(id))
}

/// Remove multiple notes by id. In non-force mode, operation is all-or-nothing.
pub fn remove_notes(ids: &[u64], force: bool) -> Result<Vec<Note>, String> {
    let mut notes = load_notes()?;
    let mut target_ids: HashSet<u64> = ids.iter().copied().collect();

    if !force {
        let existing: HashSet<u64> = notes.iter().map(|n| n.id).collect();
        let not_found: Vec<u64> = ids
            .iter()
            .copied()
            .filter(|id| !existing.contains(id))
            .collect();
        if !not_found.is_empty() {
            let joined = not_found
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!("no note with id {}; no notes were removed", joined));
        }
    }

    let mut removed = Vec::new();
    notes.retain(|n| {
        if target_ids.remove(&n.id) {
            removed.push(n.clone());
            false
        } else {
            true
        }
    });

    save_notes(&notes)?;
    Ok(removed)
}

/// Search notes by text or tag (case-insensitive substring match).
pub fn search_notes(query: &str) -> Result<Vec<Note>, String> {
    let notes = load_notes()?;
    let q = query.to_lowercase();
    Ok(notes
        .into_iter()
        .filter(|n| {
            n.text.to_lowercase().contains(&q)
                || n.tags.iter().any(|t| t.to_lowercase().contains(&q))
        })
        .collect())
}

/// Replace note text and set `updated_at`.
pub fn edit_note(id: u64, text: &str) -> Result<Note, String> {
    validate_text(text)?;
    modify_note(id, |note| {
        note.text = text.to_string();
        Ok(true)
    })
}

/// Append text to a note's existing text, separated by a space, and set
/// `updated_at`. Fails if the combined text would exceed the size limit.
pub fn append_note(id: u64, text: &str) -> Result<Note, String> {
    if text.trim().is_empty() {
        return Err("note text cannot be empty".to_string());
    }
    let suffix = text.to_string();
    modify_note(id, |note| {
        let combined = format!("{} {}", note.text, suffix);
        if combined.len() > MAX_NOTE_BYTES {
            return Err("note text exceeds 1 MB limit after append".to_string());
        }
        note.text = combined;
        Ok(true)
    })
}

/// Fetch one note by id.
pub fn get_note(id: u64) -> Result<Note, String> {
    let notes = load_notes()?;
    notes
        .into_iter()
        .find(|n| n.id == id)
        .ok_or_else(|| note_not_found(id))
}

/// Remove all notes.
pub fn clear_notes() -> Result<(), String> {
    save_notes(&[])
}

/// Import notes and reassign ids to avoid conflicts.
pub fn import_notes(mut incoming: Vec<Note>) -> Result<(), String> {
    for note in &incoming {
        validate_note(note)?;
    }

    let mut notes = load_notes()?;
    let mut max_id = notes.iter().map(|n| n.id).max().unwrap_or(0);

    for note in &mut incoming {
        max_id = next_id(max_id)?;
        note.id = max_id;
    }

    notes.extend(incoming);
    save_notes(&notes)
}

/// Add tags to a note while preserving existing tags and deduplicating new ones.
pub fn tag_note(id: u64, tags: &[String]) -> Result<Note, String> {
    modify_note(id, |note| {
        let mut changed = false;
        for tag in tags {
            if !note.tags.iter().any(|t| tag_matches(t, tag)) {
                note.tags.push(tag.clone());
                changed = true;
            }
        }
        Ok(changed)
    })
}

/// Remove one tag from a note (case-insensitive). No-op if the tag is absent.
pub fn untag_note(id: u64, tag: &str) -> Result<Note, String> {
    let needle = tag.to_lowercase();
    modify_note(id, |note| {
        let before = note.tags.len();
        note.tags.retain(|t| t.to_lowercase() != needle);
        Ok(note.tags.len() < before)
    })
}

/// Build tag usage counts across a set of notes.
pub fn collect_tags(notes: &[Note]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for note in notes {
        for tag in &note.tags {
            *counts.entry(tag.clone()).or_insert(0) += 1;
        }
    }
    counts
}

/// List notes with optional tag filtering, sort mode, and result limit.
pub fn list_notes(opts: &ListOptions) -> Result<Vec<Note>, String> {
    let mut notes = load_notes()?;

    /// Returns the sort key for updated-mode: `updated_at` when set, otherwise `created_at`.
    fn updated_sort_key(note: &Note) -> &str {
        if note.updated_at.is_empty() {
            note.created_at.as_str()
        } else {
            note.updated_at.as_str()
        }
    }

    if !opts.tag.is_empty() {
        let needle = opts.tag.to_lowercase();
        notes.retain(|n| n.tags.iter().any(|t| t.to_lowercase() == needle));
    }

    match opts.sort.as_str() {
        "" | "id" => notes.sort_by_key(|n| n.id),
        "date" => notes.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        "updated" => {
            notes.sort_by(|a, b| updated_sort_key(b).cmp(updated_sort_key(a)));
        }
        other => {
            return Err(format!(
                "unknown sort \"{}\": use id, date, or updated",
                other
            ));
        }
    }

    if opts.limit > 0 && notes.len() > opts.limit {
        notes.truncate(opts.limit);
    }

    Ok(notes)
}
