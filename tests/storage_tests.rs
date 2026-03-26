//! Integration tests for persistence and encrypted/plain file handling.

mod common;
use common::{TestEnv, lock_test};
use scriv::{
    Note, is_encrypted_data, load_notes, notes_file_is_encrypted, notes_path, save_notes,
    set_active_password,
};
use std::fs;

#[test]
fn load_notes_missing_file_returns_empty() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    let notes = load_notes().expect("load notes");
    assert!(notes.is_empty());
}

#[test]
fn save_and_load_plain_notes() {
    let _guard = lock_test();
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
    let _guard = lock_test();
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
    let _guard = lock_test();
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
    let _guard = lock_test();
    let _env = TestEnv::new();

    fs::write(notes_path(), "not json").expect("write corrupted file");

    let err = load_notes().expect_err("expected corrupted-file error");
    assert_eq!(
        err,
        "notes file is corrupted. Run 'scriv clear --force' to reset."
    );
}

#[test]
fn load_notes_with_wrong_password_fails() {
    let _guard = lock_test();
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
