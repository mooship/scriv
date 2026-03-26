//! Shared test infrastructure for integration tests.

use once_cell::sync::Lazy;
use scriv::{set_active_password, set_notes_path_override};
use std::sync::{Mutex, MutexGuard};
use tempfile::TempDir;

static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// Acquires the global test serialization lock, recovering from any prior test panic.
pub fn lock_test() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct TestEnv {
    pub _dir: TempDir,
}

impl TestEnv {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
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
