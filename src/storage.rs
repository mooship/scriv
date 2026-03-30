//! Notes file path resolution and persistence logic.

use crate::crypto::{ENCRYPTED_MAGIC, decrypt_notes, encrypt_notes, is_encrypted_data};
use crate::model::Note;
use once_cell::sync::Lazy;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zeroize::Zeroizing;

static NOTES_PATH_OVERRIDE: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));
static ACTIVE_PASSWORD: Lazy<Mutex<Zeroizing<String>>> =
    Lazy::new(|| Mutex::new(Zeroizing::new(String::new())));

/// Override notes path for tests and controlled environments.
pub fn set_notes_path_override(path: Option<PathBuf>) {
    let mut guard = NOTES_PATH_OVERRIDE
        .lock()
        .expect("notes path override lock poisoned");
    *guard = path;
}

/// Set in-memory password used for decrypting/encrypting notes.
pub fn set_active_password(password: String) {
    let mut guard = ACTIVE_PASSWORD
        .lock()
        .expect("active password lock poisoned");
    *guard = Zeroizing::new(password);
}

/// Get current active password value (zeroized on drop).
pub(crate) fn active_password_zeroized() -> Zeroizing<String> {
    let guard = ACTIVE_PASSWORD
        .lock()
        .expect("active password lock poisoned");
    guard.clone()
}

/// Get current active password value.
pub fn active_password() -> String {
    let guard = ACTIVE_PASSWORD
        .lock()
        .expect("active password lock poisoned");
    String::clone(&guard)
}

/// Resolve the platform-specific notes file path.
pub fn notes_path() -> PathBuf {
    if let Some(p) = NOTES_PATH_OVERRIDE
        .lock()
        .expect("notes path override lock poisoned")
        .clone()
    {
        return p;
    }

    let data_dir = if cfg!(target_os = "windows") {
        std::env::var("APPDATA").unwrap_or_default()
    } else if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_default();
        Path::new(&home)
            .join("Library")
            .join("Application Support")
            .to_string_lossy()
            .into_owned()
    } else {
        let xdg = std::env::var("XDG_DATA_HOME").unwrap_or_default();
        if !xdg.is_empty() {
            xdg
        } else {
            let home = std::env::var("HOME").unwrap_or_default();
            Path::new(&home)
                .join(".local")
                .join("share")
                .to_string_lossy()
                .into_owned()
        }
    };

    let base = if data_dir.is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(data_dir)
    };

    base.join("scriv").join("notes.json")
}

/// Return true when the on-disk notes file starts with the encrypted magic header.
pub fn notes_file_is_encrypted() -> bool {
    let path = notes_path();
    let file = fs::File::open(path);
    let mut file = match file {
        Ok(f) => f,
        Err(_) => return false,
    };

    let mut header = [0_u8; ENCRYPTED_MAGIC.len()];
    match file.read_exact(&mut header) {
        Ok(()) => header == *ENCRYPTED_MAGIC,
        Err(_) => false,
    }
}

/// Load notes from disk. Missing files are treated as an empty dataset.
pub fn load_notes() -> Result<Vec<Note>, String> {
    let path = notes_path();
    let mut data = match fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("cannot read from {}: {}", path.display(), e)),
    };

    if is_encrypted_data(&data) {
        data = decrypt_notes(&data, &active_password_zeroized())?;
    }

    let reader = BufReader::new(data.as_slice());
    let mut notes = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|e| format!("cannot read from {}: {}", path.display(), e))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let note: Note = serde_json::from_str(trimmed).map_err(|_| {
            "notes file is corrupted. Run 'scriv clear --force' to reset.".to_string()
        })?;
        notes.push(note);
    }

    Ok(notes)
}

/// Persist notes to disk using atomic replacement via a temporary file to reduce corruption risk.
pub fn save_notes(notes: &[Note]) -> Result<(), String> {
    let path = notes_path();
    let dir = path
        .parent()
        .ok_or_else(|| format!("cannot write to {}", path.display()))?
        .to_path_buf();

    fs::create_dir_all(&dir).map_err(|e| format!("cannot write to {}: {}", dir.display(), e))?;

    let mut ndjson = Vec::new();
    for note in notes {
        let line = serde_json::to_string(note).map_err(|e| e.to_string())?;
        ndjson.extend_from_slice(line.as_bytes());
        ndjson.push(b'\n');
    }

    let pw = active_password_zeroized();
    let payload = if pw.is_empty() {
        ndjson
    } else {
        encrypt_notes(&ndjson, &pw).map_err(|e| format!("cannot encrypt notes: {}", e))?
    };

    let mut tmp = tempfile::NamedTempFile::new_in(&dir)
        .map_err(|e| format!("cannot write to {}: {}", dir.display(), e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        tmp.as_file()
            .set_permissions(perms)
            .map_err(|e| format!("cannot set permissions on {}: {}", path.display(), e))?;
    }

    tmp.write_all(&payload)
        .map_err(|e| format!("cannot write to {}: {}", path.display(), e))?;
    tmp.persist(&path)
        .map_err(|e| format!("cannot write to {}: {}", path.display(), e.error))?;

    Ok(())
}
