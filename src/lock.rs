//! Setting, changing, and removing the notes password.

use crate::model::Note;
use crate::storage::{
    load_notes, notes_file_is_encrypted, save_notes, set_active_password,
    set_active_password_zeroized,
};
use zeroize::Zeroizing;

/// Load notes, clearing the active password if the load fails so a wrong
/// password is not retained in memory.
fn load_notes_clearing_password_on_err() -> Result<Vec<Note>, String> {
    load_notes().inspect_err(|_| set_active_password(String::new()))
}

/// Set or change the notes password.
///
/// `prompt` is called with the prompt text and returns what the user typed.
/// When the store is already encrypted it asks for the current password
/// first and uses it to read the notes, then asks for the new password twice
/// and re-saves the notes encrypted under it. An empty new password or a
/// mismatched confirmation is rejected before anything is written.
pub fn lock_notes<P>(mut prompt: P) -> Result<(), String>
where
    P: FnMut(&str) -> Result<Zeroizing<String>, String>,
{
    let notes = if notes_file_is_encrypted() {
        set_active_password_zeroized(prompt("Current password: ")?);
        load_notes_clearing_password_on_err()?
    } else {
        load_notes()?
    };

    let pw = prompt("New password: ")?;
    if pw.is_empty() {
        return Err("password cannot be empty".to_string());
    }
    let confirm = prompt("Confirm password: ")?;
    if *pw != *confirm {
        return Err("passwords do not match".to_string());
    }

    set_active_password_zeroized(pw);
    save_notes(&notes)
}

/// Remove password protection by decrypting the store and re-saving it as
/// plain NDJSON.
///
/// `prompt` is called with the prompt text and returns what the user typed.
/// Returns `Ok(false)` without prompting when the store is not encrypted,
/// and `Ok(true)` once the password has been removed.
pub fn unlock_notes<P>(prompt: P) -> Result<bool, String>
where
    P: FnOnce(&str) -> Result<Zeroizing<String>, String>,
{
    if !notes_file_is_encrypted() {
        return Ok(false);
    }

    set_active_password_zeroized(prompt("Password: ")?);
    let notes = load_notes_clearing_password_on_err()?;
    set_active_password(String::new());
    save_notes(&notes)?;
    Ok(true)
}
