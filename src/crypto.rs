//! Encryption helpers for notes-at-rest support.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use pbkdf2::pbkdf2_hmac;
use rand::RngExt;
use sha2::Sha256;

/// File signature for encrypted note payloads.
pub const ENCRYPTED_MAGIC: &[u8; 6] = b"scriv\x01"; // pragma: allowlist secret
const PBKDF2_ITERS: u32 = 100_000;
const PBKDF2_KEY_LEN: usize = 32;
const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;

/// Encrypt NDJSON note bytes using AES-256-GCM and PBKDF2 key derivation.
pub fn encrypt_notes(plaintext: &[u8], password: &str) -> Result<Vec<u8>, String> {
    let mut salt = [0_u8; SALT_LEN];
    rand::rng().fill(&mut salt);

    let mut key = vec![0_u8; PBKDF2_KEY_LEN];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, PBKDF2_ITERS, &mut key);

    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let mut nonce = [0_u8; NONCE_LEN];
    rand::rng().fill(&mut nonce);

    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|e| e.to_string())?;

    let mut out =
        Vec::with_capacity(ENCRYPTED_MAGIC.len() + SALT_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(ENCRYPTED_MAGIC);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypt note bytes previously produced by `encrypt_notes`.
pub fn decrypt_notes(data: &[u8], password: &str) -> Result<Vec<u8>, String> {
    let min_len = ENCRYPTED_MAGIC.len() + SALT_LEN + NONCE_LEN + 16;
    if data.len() < min_len || &data[0..ENCRYPTED_MAGIC.len()] != ENCRYPTED_MAGIC {
        return Err("notes file is corrupted".to_string());
    }

    let mut offset = ENCRYPTED_MAGIC.len();
    let salt = &data[offset..offset + SALT_LEN];
    offset += SALT_LEN;
    let nonce = &data[offset..offset + NONCE_LEN];
    offset += NONCE_LEN;
    let ciphertext = &data[offset..];

    let mut key = vec![0_u8; PBKDF2_KEY_LEN];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERS, &mut key);

    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| "incorrect password".to_string())
}

/// Quick header check used to detect encrypted files.
pub fn is_encrypted_data(data: &[u8]) -> bool {
    data.len() >= ENCRYPTED_MAGIC.len() && &data[0..ENCRYPTED_MAGIC.len()] == ENCRYPTED_MAGIC
}
