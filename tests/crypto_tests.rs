//! Integration tests for encryption format and error handling.

mod common;
use aes_gcm::Aes256Gcm;
use aes_gcm::aead::{Aead, KeyInit, Nonce};
use common::password;
use pbkdf2::pbkdf2_hmac;
use scriv::{ENCRYPTED_MAGIC, decrypt_notes, encrypt_notes, is_encrypted_data};
use sha2::Sha256;

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

/// Derive the AES key the way the documented file format does.
fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0_u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 600_000, &mut key);
    key
}

#[test]
fn encrypt_notes_output_follows_the_documented_layout() {
    let pw = password("layout");
    let plaintext = b"layout check\n";
    let encrypted = encrypt_notes(plaintext, &pw).expect("encrypt");

    let (magic, rest) = encrypted.split_at(ENCRYPTED_MAGIC.len());
    let (salt, rest) = rest.split_at(32);
    let (nonce, ciphertext) = rest.split_at(12);
    assert_eq!(magic, ENCRYPTED_MAGIC);

    let cipher = Aes256Gcm::new_from_slice(&derive_key(&pw, salt)).expect("key");
    let nonce = <&Nonce<Aes256Gcm>>::try_from(nonce).expect("nonce length");
    let decrypted = cipher
        .decrypt(nonce, ciphertext)
        .expect("independent decrypt");
    assert_eq!(decrypted, plaintext);
}

#[test]
fn decrypt_notes_reads_a_file_built_independently() {
    let pw = password("hand");
    let plaintext = b"built by hand\n";
    let salt: [u8; 32] = rand::random();
    let nonce: [u8; 12] = rand::random();
    let cipher = Aes256Gcm::new_from_slice(&derive_key(&pw, &salt)).expect("key");
    let ciphertext = cipher
        .encrypt(&Nonce::<Aes256Gcm>::from(nonce), plaintext.as_slice())
        .expect("encrypt");

    let mut file = ENCRYPTED_MAGIC.to_vec();
    file.extend_from_slice(&salt);
    file.extend_from_slice(&nonce);
    file.extend_from_slice(&ciphertext);

    assert_eq!(decrypt_notes(&file, &pw).expect("decrypt"), plaintext);
}

#[test]
fn decrypt_rejects_tampered_ciphertext() {
    let pw = password("tamper");
    let mut encrypted = encrypt_notes(b"tamper check\n", &pw).expect("encrypt");
    let last = encrypted.len() - 1;
    encrypted[last] ^= 0x01;

    let err = decrypt_notes(&encrypted, &pw).expect_err("expected authentication failure");
    assert_eq!(err, "incorrect password");
}

#[test]
fn encrypting_twice_uses_a_fresh_salt_and_nonce() {
    let pw = password("fresh");
    let first = encrypt_notes(b"same input\n", &pw).expect("encrypt");
    let second = encrypt_notes(b"same input\n", &pw).expect("encrypt");

    let header = ENCRYPTED_MAGIC.len();
    assert_ne!(first[header..header + 32], second[header..header + 32]);
    assert_ne!(
        first[header + 32..header + 44],
        second[header + 32..header + 44]
    );
}
