//! Integration tests for setting, changing, and removing the notes password.

mod common;
use common::{TestEnv, lock_test};
use scriv::{
    add_note, has_active_password, load_notes, lock_notes, notes_file_is_encrypted, notes_path,
    set_active_password, unlock_notes,
};
use std::collections::VecDeque;
use std::fs;
use zeroize::Zeroizing;

/// Feeds scripted answers to the password prompts and records each prompt shown.
struct Script {
    answers: VecDeque<String>,
    asked: Vec<String>,
}

impl Script {
    fn new(answers: &[&str]) -> Self {
        Self {
            answers: answers.iter().map(|a| a.to_string()).collect(),
            asked: Vec::new(),
        }
    }

    fn prompt(&mut self, message: &str) -> Result<Zeroizing<String>, String> {
        self.asked.push(message.to_string());
        self.answers
            .pop_front()
            .map(Zeroizing::new)
            .ok_or_else(|| "no scripted answer".to_string())
    }
}

/// Encrypt the current store under `password`, then forget it the way a fresh
/// process would.
fn lock_with(password: &str) {
    let mut script = Script::new(&[password, password]);
    lock_notes(|message| script.prompt(message)).expect("lock notes");
    set_active_password(String::new());
}

#[test]
fn lock_notes_encrypts_a_plain_store() {
    let _guard = lock_test();
    let _env = TestEnv::new();
    add_note("keep me private").expect("add note");

    let mut script = Script::new(&["hunter2", "hunter2"]);
    lock_notes(|message| script.prompt(message)).expect("lock notes");

    assert_eq!(script.asked, ["New password: ", "Confirm password: "]);
    assert!(notes_file_is_encrypted());
    assert!(has_active_password());
    let raw = fs::read(notes_path()).expect("read notes file");
    assert!(!String::from_utf8_lossy(&raw).contains("keep me private"));
    let notes = load_notes().expect("load notes");
    assert_eq!(notes[0].text, "keep me private");
}

#[test]
fn lock_notes_rejects_an_empty_password() {
    let _guard = lock_test();
    let _env = TestEnv::new();
    add_note("plain").expect("add note");

    let mut script = Script::new(&[""]);
    let err = lock_notes(|message| script.prompt(message)).expect_err("empty password");

    assert_eq!(err, "password cannot be empty");
    assert_eq!(script.asked, ["New password: "]);
    assert!(!notes_file_is_encrypted());
    assert!(!has_active_password());
}

#[test]
fn lock_notes_rejects_a_mismatched_confirmation() {
    let _guard = lock_test();
    let _env = TestEnv::new();
    add_note("plain").expect("add note");

    let mut script = Script::new(&["one", "two"]);
    let err = lock_notes(|message| script.prompt(message)).expect_err("mismatch");

    assert_eq!(err, "passwords do not match");
    assert!(!notes_file_is_encrypted());
    assert!(!has_active_password());
    assert_eq!(load_notes().expect("load notes")[0].text, "plain");
}

#[test]
fn lock_notes_changes_the_password_of_an_encrypted_store() {
    let _guard = lock_test();
    let _env = TestEnv::new();
    add_note("shared secret").expect("add note");
    lock_with("old");

    let mut script = Script::new(&["old", "new", "new"]);
    lock_notes(|message| script.prompt(message)).expect("change password");

    assert_eq!(
        script.asked,
        ["Current password: ", "New password: ", "Confirm password: "]
    );
    set_active_password("old".to_string());
    assert_eq!(
        load_notes().expect_err("old password must stop working"),
        "incorrect password"
    );
    set_active_password("new".to_string());
    assert_eq!(load_notes().expect("load notes")[0].text, "shared secret");
}

#[test]
fn lock_notes_with_the_wrong_current_password_changes_nothing() {
    let _guard = lock_test();
    let _env = TestEnv::new();
    add_note("shared secret").expect("add note");
    lock_with("right");

    let mut script = Script::new(&["wrong"]);
    let err = lock_notes(|message| script.prompt(message)).expect_err("wrong password");

    assert_eq!(err, "incorrect password");
    assert_eq!(script.asked, ["Current password: "]);
    assert!(!has_active_password());
    assert!(notes_file_is_encrypted());
    set_active_password("right".to_string());
    assert_eq!(load_notes().expect("load notes")[0].text, "shared secret");
}

#[test]
fn lock_notes_stops_when_a_prompt_fails() {
    let _guard = lock_test();
    let _env = TestEnv::new();
    add_note("plain").expect("add note");

    let mut no_answers = Script::new(&[]);
    let err = lock_notes(|message| no_answers.prompt(message)).expect_err("new password prompt");
    assert_eq!(err, "no scripted answer");

    let mut one_answer = Script::new(&["pw"]);
    let err = lock_notes(|message| one_answer.prompt(message)).expect_err("confirm prompt");
    assert_eq!(err, "no scripted answer");
    assert_eq!(one_answer.asked, ["New password: ", "Confirm password: "]);

    assert!(!notes_file_is_encrypted());
    assert!(!has_active_password());
}

#[test]
fn lock_notes_stops_when_the_current_password_prompt_fails() {
    let _guard = lock_test();
    let _env = TestEnv::new();
    add_note("shared secret").expect("add note");
    lock_with("pw");

    let mut script = Script::new(&[]);
    let err = lock_notes(|message| script.prompt(message)).expect_err("current password prompt");

    assert_eq!(err, "no scripted answer");
    assert!(notes_file_is_encrypted());
}

#[test]
fn unlock_notes_does_nothing_for_a_plain_store() {
    let _guard = lock_test();
    let _env = TestEnv::new();
    add_note("plain").expect("add note");

    let mut script = Script::new(&[]);
    let unlocked = unlock_notes(|message| script.prompt(message)).expect("unlock");

    assert!(!unlocked);
    assert!(script.asked.is_empty());
    assert_eq!(load_notes().expect("load notes")[0].text, "plain");
}

#[test]
fn unlock_notes_removes_password_protection() {
    let _guard = lock_test();
    let _env = TestEnv::new();
    add_note("shared secret").expect("add note");
    lock_with("pw");

    let mut script = Script::new(&["pw"]);
    let unlocked = unlock_notes(|message| script.prompt(message)).expect("unlock");

    assert!(unlocked);
    assert_eq!(script.asked, ["Password: "]);
    assert!(!notes_file_is_encrypted());
    assert!(!has_active_password());
    assert_eq!(load_notes().expect("load notes")[0].text, "shared secret");
}

#[test]
fn unlock_notes_with_the_wrong_password_keeps_the_store_encrypted() {
    let _guard = lock_test();
    let _env = TestEnv::new();
    add_note("shared secret").expect("add note");
    lock_with("right");

    let mut script = Script::new(&["wrong"]);
    let err = unlock_notes(|message| script.prompt(message)).expect_err("wrong password");

    assert_eq!(err, "incorrect password");
    assert!(!has_active_password());
    assert!(notes_file_is_encrypted());
}

#[test]
fn unlock_notes_stops_when_the_prompt_fails() {
    let _guard = lock_test();
    let _env = TestEnv::new();
    add_note("shared secret").expect("add note");
    lock_with("pw");

    let mut script = Script::new(&[]);
    let err = unlock_notes(|message| script.prompt(message)).expect_err("password prompt");

    assert_eq!(err, "no scripted answer");
    assert!(notes_file_is_encrypted());
}
