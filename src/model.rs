use serde::{Deserialize, Serialize};

/// Persisted note record stored as NDJSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Note {
    /// Stable note identifier.
    pub id: u64,
    /// User-entered note content.
    pub text: String,
    /// RFC3339 UTC creation timestamp.
    pub created_at: String,
    /// RFC3339 UTC update timestamp; empty means never edited.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub updated_at: String,
    /// Optional user tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Options for `list_notes` filtering, sorting, and output shape.
#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    /// Optional case-insensitive tag filter.
    pub tag: String,
    /// Sort mode: `id`, `date`, or `updated`.
    pub sort: String,
    /// Maximum notes to return; `0` means unlimited.
    pub limit: usize,
    /// Whether list output should skip truncation in the CLI.
    pub full: bool,
}
