//! Integration tests for persistence and encrypted/plain file handling.

use jot::{
    Note, is_encrypted_data, load_notes, notes_file_is_encrypted, notes_path, save_notes,
    set_active_password, set_notes_path_override,
};
use once_cell::sync::Lazy;
use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

// Global lock avoids cross-test interference from global path/password state.
static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _dir: TempDir,
}

impl TestEnv {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        // Keep tests isolated from real user notes.
        set_notes_path_override(Some(dir.path().join("notes.json")));
        set_active_password(String::new());
        Self { _dir: dir }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        set_notes_path_override(None);
        set_active_password(String::new());
    }
}

#[test]
fn load_notes_missing_file_returns_empty() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    let notes = load_notes().expect("load notes");
    assert!(notes.is_empty());
}

#[test]
fn save_and_load_plain_notes() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    let notes = vec![Note {
        id: 1,
        text: "plain note".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: String::new(),
        tags: Vec::new(),
    }];

    save_notes(&notes).expect("save notes");
    let raw = fs::read(notes_path()).expect("read notes file");
    assert!(!is_encrypted_data(&raw));

    let loaded = load_notes().expect("load notes");
    assert_eq!(loaded, notes);
}

#[test]
fn save_and_load_encrypted_notes() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    set_active_password("secret".to_string());

    let notes = vec![Note {
        id: 1,
        text: "secret note".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: String::new(),
        tags: vec!["private".to_string()],
    }];

    save_notes(&notes).expect("save notes");
    let raw = fs::read(notes_path()).expect("read notes file");
    assert!(is_encrypted_data(&raw));

    let loaded = load_notes().expect("load notes");
    assert_eq!(loaded, notes);
}

#[test]
fn notes_file_is_encrypted_reflects_current_file_state() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    assert!(!notes_file_is_encrypted());

    save_notes(&[Note {
        id: 1,
        text: "plain".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: String::new(),
        tags: Vec::new(),
    }])
    .expect("save plain notes");
    assert!(!notes_file_is_encrypted());

    set_active_password("pw".to_string());
    save_notes(&[Note {
        id: 2,
        text: "encrypted".to_string(),
        created_at: "2024-01-02T00:00:00Z".to_string(),
        updated_at: String::new(),
        tags: Vec::new(),
    }])
    .expect("save encrypted notes");
    assert!(notes_file_is_encrypted());
}

#[test]
fn load_notes_corrupted_ndjson_returns_compat_error() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    fs::write(notes_path(), "not json").expect("write corrupted file");

    let err = load_notes().expect_err("expected corrupted-file error");
    assert_eq!(
        err,
        "notes file is corrupted. Run 'jot clear --force' to reset."
    );
}

#[test]
fn load_notes_with_wrong_password_fails() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    set_active_password("correct".to_string());
    save_notes(&[Note {
        id: 1,
        text: "top secret".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: String::new(),
        tags: Vec::new(),
    }])
    .expect("save encrypted notes");

    set_active_password("wrong".to_string());
    let err = load_notes().expect_err("expected wrong password error");
    assert_eq!(err, "incorrect password");
}
