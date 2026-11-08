//! Shared test infrastructure for integration tests.
//!
//! Each test binary compiles this module but uses only part of it.
#![allow(dead_code)]

use once_cell::sync::Lazy;
use scriv::{set_active_password, set_notes_path_override};
use std::sync::{Mutex, MutexGuard};
use tempfile::TempDir;

static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// Acquires the global test serialization lock, recovering from any prior test panic.
pub fn lock_test() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Redirects note storage to a temp file and clears the active password for
/// the duration of a test, restoring both global states on drop.
pub struct TestEnv {
    /// Held only to keep the temp directory alive for the test's duration.
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

/// Makes a directory unwritable for the current user until dropped.
#[cfg(unix)]
pub struct ReadOnlyDir {
    dir: std::path::PathBuf,
    original: std::fs::Permissions,
}

#[cfg(unix)]
impl ReadOnlyDir {
    /// Returns `None` when the user can write to the directory regardless
    /// (for example root), so callers can skip rather than fail.
    pub fn new(dir: &std::path::Path) -> Option<Self> {
        use std::os::unix::fs::PermissionsExt;

        let original = std::fs::metadata(dir).expect("dir metadata").permissions();
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o500))
            .expect("make dir read-only");
        let guard = Self {
            dir: dir.to_path_buf(),
            original,
        };

        let probe = dir.join(".probe");
        if std::fs::write(&probe, "x").is_ok() {
            std::fs::remove_file(&probe).expect("remove probe");
            return None;
        }
        Some(guard)
    }
}

#[cfg(unix)]
impl Drop for ReadOnlyDir {
    fn drop(&mut self) {
        std::fs::set_permissions(&self.dir, self.original.clone()).expect("restore permissions");
    }
}

/// A random password, generated at run time so that no secret literal ends up
/// in the tests.
pub fn password() -> String {
    format!("{:016x}", rand::random::<u64>())
}
