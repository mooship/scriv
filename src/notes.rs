//! Business operations over notes.

use crate::model::{ListOptions, Note};
use crate::storage::{load_notes, save_notes};
use chrono::Utc;
use std::collections::{HashMap, HashSet};

/// Current UTC timestamp in RFC3339 format used by persisted note fields.
fn now_timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Create and persist a new note with `max(existing_id) + 1` semantics.
pub fn add_note(text: &str) -> Result<Note, String> {
    let mut notes = load_notes()?;
    let max_id = notes.iter().map(|n| n.id).max().unwrap_or(0);
    let note = Note {
        id: max_id + 1,
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
    Err(format!("no note with id {}", id))
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
    let mut notes = load_notes()?;
    if let Some(note) = notes.iter_mut().find(|n| n.id == id) {
        note.text = text.to_string();
        note.updated_at = now_timestamp();
        let out = note.clone();
        save_notes(&notes)?;
        return Ok(out);
    }
    Err(format!("no note with id {}", id))
}

/// Append text to a note and set `updated_at`.
pub fn append_note(id: u64, text: &str) -> Result<Note, String> {
    let mut notes = load_notes()?;
    if let Some(note) = notes.iter_mut().find(|n| n.id == id) {
        note.text = format!("{} {}", note.text, text);
        note.updated_at = now_timestamp();
        let out = note.clone();
        save_notes(&notes)?;
        return Ok(out);
    }
    Err(format!("no note with id {}", id))
}

/// Fetch one note by id.
pub fn get_note(id: u64) -> Result<Note, String> {
    let notes = load_notes()?;
    notes
        .into_iter()
        .find(|n| n.id == id)
        .ok_or_else(|| format!("no note with id {}", id))
}

/// Remove all notes.
pub fn clear_notes() -> Result<(), String> {
    save_notes(&[])
}

/// Import notes and reassign ids to avoid conflicts.
pub fn import_notes(mut incoming: Vec<Note>) -> Result<(), String> {
    let mut notes = load_notes()?;
    let mut max_id = notes.iter().map(|n| n.id).max().unwrap_or(0);

    for note in &mut incoming {
        max_id += 1;
        note.id = max_id;
    }

    notes.extend(incoming);
    save_notes(&notes)
}

/// Add tags to a note while preserving existing tags and deduplicating new ones.
pub fn tag_note(id: u64, tags: &[String]) -> Result<Note, String> {
    let mut notes = load_notes()?;
    if let Some(note) = notes.iter_mut().find(|n| n.id == id) {
        for tag in tags {
            if !note
                .tags
                .iter()
                .any(|t| t.to_lowercase() == tag.to_lowercase())
            {
                note.tags.push(tag.clone());
            }
        }
        let out = note.clone();
        save_notes(&notes)?;
        return Ok(out);
    }
    Err(format!("no note with id {}", id))
}

/// Remove one tag from a note (case-insensitive). No-op if the tag is absent.
pub fn untag_note(id: u64, tag: &str) -> Result<Note, String> {
    let mut notes = load_notes()?;
    if let Some(note) = notes.iter_mut().find(|n| n.id == id) {
        let before = note.tags.len();
        note.tags.retain(|t| t.to_lowercase() != tag.to_lowercase());
        let changed = note.tags.len() < before;
        let out = note.clone();
        if changed {
            save_notes(&notes)?;
        }
        return Ok(out);
    }
    Err(format!("no note with id {}", id))
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
