//! Integration tests for core note behavior and compatibility guarantees.

use scriv::*;
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
fn add_note_assigns_id_1_when_empty() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    let note = add_note("first").expect("add note");
    assert_eq!(note.id, 1);
}

#[test]
fn add_note_id_is_max_plus_one() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    add_note("one").expect("add one");
    add_note("two").expect("add two");
    let note = add_note("three").expect("add three");
    assert_eq!(note.id, 3);
}

#[test]
fn remove_note_ids_are_stable() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    add_note("one").expect("add one");
    add_note("two").expect("add two");
    add_note("three").expect("add three");
    remove_note(2).expect("remove two");

    let note = add_note("four").expect("add four");
    assert_eq!(note.id, 4);
}

#[test]
fn search_notes_case_insensitive() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    add_note("Fix the Auth Bug").expect("add 1");
    add_note("write tests").expect("add 2");

    let results = search_notes("auth").expect("search");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 1);
}

#[test]
fn edit_note_sets_updated_at() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    add_note("original").expect("add");
    let updated = edit_note(1, "revised").expect("edit");

    assert!(!updated.updated_at.is_empty());
}

#[test]
fn tag_note_deduplicates_tags() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    add_note("buy milk").expect("add");
    tag_note(1, &["groceries".to_string()]).expect("tag 1");
    let note = tag_note(1, &["groceries".to_string()]).expect("tag 2");

    assert_eq!(note.tags.len(), 1);
}

#[test]
fn list_notes_sort_by_updated() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    add_note("first").expect("add first");
    add_note("second").expect("add second");
    edit_note(1, "first edited").expect("edit first");

    let notes = list_notes(&ListOptions {
        sort: "updated".to_string(),
        ..Default::default()
    })
    .expect("list notes");

    assert_eq!(notes[0].id, 1);
}

#[test]
fn remove_notes_force_ignores_missing() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    add_note("alpha").expect("add alpha");
    add_note("beta").expect("add beta");

    let removed = remove_notes(&[1, 99], true).expect("remove notes");
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].id, 1);
}

#[test]
fn import_notes_assigns_new_ids() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    add_note("existing").expect("add existing");
    import_notes(vec![
        Note {
            id: 1,
            text: "imported one".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: String::new(),
            tags: Vec::new(),
        },
        Note {
            id: 2,
            text: "imported two".to_string(),
            created_at: "2024-01-02T00:00:00Z".to_string(),
            updated_at: String::new(),
            tags: Vec::new(),
        },
    ])
    .expect("import notes");

    let notes = load_notes().expect("load notes");
    assert_eq!(notes.len(), 3);
    assert_eq!(notes[1].id, 2);
    assert_eq!(notes[2].id, 3);
}

#[test]
fn compat_minimal_fixture_loads() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    fs::write(
        notes_path(),
        "{\"id\":1,\"text\":\"hello world\",\"created_at\":\"2024-01-15T10:30:00Z\"}",
    )
    .expect("write fixture");

    let notes = load_notes().expect("load notes");
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].id, 1);
    assert_eq!(notes[0].updated_at, "");
    assert!(notes[0].tags.is_empty());
}

#[test]
fn compat_unknown_fields_are_ignored() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    fs::write(
        notes_path(),
        "{\"id\":1,\"text\":\"note\",\"created_at\":\"2024-01-01T00:00:00Z\",\"future_field\":\"ignored\",\"another\":123}",
    )
    .expect("write fixture");

    let notes = load_notes().expect("load notes");
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, "note");
}

#[test]
fn encrypt_decrypt_roundtrip() {
    let plaintext = b"{\"id\":1,\"text\":\"hello\",\"created_at\":\"2024-01-01T00:00:00Z\"}\n";
    let encrypted = encrypt_notes(plaintext, "secret").expect("encrypt");
    assert_eq!(&encrypted[0..ENCRYPTED_MAGIC.len()], ENCRYPTED_MAGIC);

    let decrypted = decrypt_notes(&encrypted, "secret").expect("decrypt");
    assert_eq!(decrypted, plaintext);
}

#[test]
fn decrypt_wrong_password() {
    let plaintext = b"test\n";
    let encrypted = encrypt_notes(plaintext, "correct").expect("encrypt");
    let err = decrypt_notes(&encrypted, "wrong").expect_err("expected wrong password");
    assert_eq!(err, "incorrect password");
}

#[test]
fn save_load_encrypted_notes() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let _env = TestEnv::new();

    set_active_password("testpassword".to_string());

    let want = vec![
        Note {
            id: 1,
            text: "secret note".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: String::new(),
            tags: Vec::new(),
        },
        Note {
            id: 2,
            text: "another secret".to_string(),
            created_at: "2024-02-01T00:00:00Z".to_string(),
            updated_at: String::new(),
            tags: vec!["private".to_string()],
        },
    ];

    save_notes(&want).expect("save notes");
    let raw = fs::read(notes_path()).expect("read raw");
    assert!(is_encrypted_data(&raw));

    let got = load_notes().expect("load notes");
    assert_eq!(got.len(), want.len());
    assert_eq!(got[0].text, "secret note");
}
