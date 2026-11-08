//! Integration tests for persistence and encrypted/plain file handling.

mod common;
#[cfg(unix)]
use common::ReadOnlyDir;
use common::{TestEnv, lock_test};
use scriv::{
    Note, is_encrypted_data, load_notes, notes_file_is_encrypted, notes_path, save_notes,
    set_active_password, set_notes_path_override,
};
use std::fs;
use std::path::{Path, PathBuf};

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
    assert!(err.contains("notes file is corrupted"));
    assert!(err.contains("Run 'scriv clear --force' to reset."));
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

fn sample_note() -> Note {
    Note {
        id: 1,
        text: "sample".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: String::new(),
        tags: Vec::new(),
    }
}

#[test]
fn notes_path_defaults_to_scriv_notes_json_without_an_override() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    set_notes_path_override(None);

    assert!(notes_path().ends_with(Path::new("scriv").join("notes.json")));
}

#[test]
fn load_notes_reports_a_path_that_cannot_be_read() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    fs::create_dir(notes_path()).expect("create directory at notes path");

    let err = load_notes().expect_err("expected read error");
    assert!(
        err.starts_with("cannot read from"),
        "unexpected error: {err}"
    );
}

#[test]
fn load_notes_reports_content_that_is_not_utf8() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    fs::write(notes_path(), [0xff_u8, 0xfe, b'\n']).expect("write notes file");

    let err = load_notes().expect_err("expected read error");
    assert!(
        err.starts_with("cannot read from"),
        "unexpected error: {err}"
    );
}

#[test]
fn load_notes_skips_blank_lines() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    let line = serde_json::to_string(&sample_note()).expect("serialise note");
    fs::write(notes_path(), format!("\n{line}\n   \n")).expect("write notes file");

    assert_eq!(load_notes().expect("load notes"), vec![sample_note()]);
}

#[test]
fn save_notes_reports_a_path_with_no_parent() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    set_notes_path_override(Some(PathBuf::from("/")));

    let err = save_notes(&[sample_note()]).expect_err("expected path error");
    assert_eq!(err, format!("cannot write to {}", Path::new("/").display()));
}

#[test]
fn save_notes_reports_a_data_directory_that_cannot_be_created() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    let blocker = notes_path().with_file_name("blocker");
    fs::write(&blocker, "a file, not a directory").expect("write blocker");
    set_notes_path_override(Some(blocker.join("notes.json")));

    let err = save_notes(&[sample_note()]).expect_err("expected directory error");
    assert!(
        err.starts_with("cannot write to"),
        "unexpected error: {err}"
    );
}

#[test]
fn save_notes_reports_a_notes_path_that_is_a_directory() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    fs::create_dir(notes_path()).expect("create directory at notes path");

    let err = save_notes(&[sample_note()]).expect_err("expected replace error");
    assert!(
        err.starts_with("cannot write to"),
        "unexpected error: {err}"
    );
}

#[cfg(unix)]
#[test]
fn save_notes_creates_a_private_directory_and_file() {
    use std::os::unix::fs::PermissionsExt;

    let _guard = lock_test();
    let _env = TestEnv::new();

    let data_dir = notes_path().with_file_name("data");
    set_notes_path_override(Some(data_dir.join("notes.json")));

    save_notes(&[sample_note()]).expect("save notes");

    let dir_mode = fs::metadata(&data_dir)
        .expect("dir metadata")
        .permissions()
        .mode();
    let file_mode = fs::metadata(data_dir.join("notes.json"))
        .expect("file metadata")
        .permissions()
        .mode();
    assert_eq!(dir_mode & 0o777, 0o700);
    assert_eq!(file_mode & 0o777, 0o600);
}

#[cfg(unix)]
#[test]
fn save_notes_keeps_the_permissions_of_an_existing_directory() {
    use std::os::unix::fs::PermissionsExt;

    let _guard = lock_test();
    let _env = TestEnv::new();

    let data_dir = notes_path().with_file_name("existing");
    fs::create_dir(&data_dir).expect("create data dir");
    fs::set_permissions(&data_dir, fs::Permissions::from_mode(0o755)).expect("set permissions");
    set_notes_path_override(Some(data_dir.join("notes.json")));

    save_notes(&[sample_note()]).expect("save notes");

    let dir_mode = fs::metadata(&data_dir)
        .expect("dir metadata")
        .permissions()
        .mode();
    assert_eq!(dir_mode & 0o777, 0o755);
}

#[test]
#[allow(deprecated)]
fn active_password_returns_the_password_that_was_set() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    assert_eq!(scriv::active_password(), "");
    set_active_password("hunter2".to_string());
    assert_eq!(scriv::active_password(), "hunter2");
}

#[cfg(unix)]
#[test]
fn save_notes_reports_a_directory_it_cannot_write_into() {
    let _guard = lock_test();
    let _env = TestEnv::new();

    let data_dir = notes_path().with_file_name("read-only");
    fs::create_dir(&data_dir).expect("create data dir");
    set_notes_path_override(Some(data_dir.join("notes.json")));
    let Some(_read_only) = ReadOnlyDir::new(&data_dir) else {
        eprintln!("skipped: permission checks do not apply to this user");
        return;
    };

    let err = save_notes(&[sample_note()]).expect_err("expected write error");
    assert!(
        err.starts_with("cannot write to"),
        "unexpected error: {err}"
    );
}
