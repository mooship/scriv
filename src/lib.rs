//! Public crate surface for scriv.
//! Modules stay internal and are re-exported here to keep the external API stable.

mod crypto;
mod format;
mod model;
mod notes;
mod storage;

pub use crypto::{ENCRYPTED_MAGIC, decrypt_notes, encrypt_notes, is_encrypted_data};
pub use format::{highlight_match, note_age, read_stdin_text};
pub use model::{ListOptions, Note};
pub use notes::{
    add_note, append_note, clear_notes, collect_tags, edit_note, get_note, import_notes,
    list_notes, remove_note, remove_notes, search_notes, tag_note, untag_note,
};
pub use storage::{
    active_password, has_active_password, load_notes, notes_file_is_encrypted, notes_path,
    save_notes, set_active_password, set_notes_path_override,
};
