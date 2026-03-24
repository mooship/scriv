//! Integration tests for encryption format and error handling.

use scriv::{ENCRYPTED_MAGIC, decrypt_notes, encrypt_notes, is_encrypted_data};

#[test]
fn encrypt_notes_writes_magic_header() {
    let encrypted = encrypt_notes(b"hello\n", "secret").expect("encrypt");
    assert_eq!(&encrypted[..ENCRYPTED_MAGIC.len()], ENCRYPTED_MAGIC);
}

#[test]
fn encrypt_decrypt_roundtrip() {
    let plaintext = b"{\"id\":1,\"text\":\"hello\",\"created_at\":\"2024-01-01T00:00:00Z\"}\n";
    let encrypted = encrypt_notes(plaintext, "secret").expect("encrypt");
    let decrypted = decrypt_notes(&encrypted, "secret").expect("decrypt");
    assert_eq!(decrypted, plaintext);
}

#[test]
fn decrypt_with_wrong_password_fails() {
    let encrypted = encrypt_notes(b"test\n", "correct").expect("encrypt");
    let err = decrypt_notes(&encrypted, "wrong").expect_err("expected wrong password error");
    assert_eq!(err, "incorrect password");
}

#[test]
fn decrypt_rejects_truncated_data() {
    let err = decrypt_notes(b"scriv\x01short", "pw").expect_err("expected truncated error");
    assert_eq!(err, "notes file is corrupted");
}

#[test]
fn decrypt_rejects_non_magic_data() {
    let err = decrypt_notes(b"{\"id\":1}", "pw").expect_err("expected non-magic error");
    assert_eq!(err, "notes file is corrupted");
}

#[test]
fn is_encrypted_data_detects_encrypted_and_plain() {
    let plain = b"{\"id\":1,\"text\":\"hi\",\"created_at\":\"2024-01-01T00:00:00Z\"}\n";
    assert!(!is_encrypted_data(plain));

    let encrypted = encrypt_notes(b"test", "pw").expect("encrypt");
    assert!(is_encrypted_data(&encrypted));
}
