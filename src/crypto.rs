//! Encryption helpers for notes-at-rest support.

use aes_gcm::Aes256Gcm;
use aes_gcm::aead::{Aead, KeyInit, Nonce};
use pbkdf2::pbkdf2_hmac;
use rand::RngExt;
use sha2::Sha256;
use zeroize::Zeroizing;

/// File signature for encrypted note payloads.
pub const ENCRYPTED_MAGIC: &[u8; 6] = b"scriv\x01";
const PBKDF2_ITERS: u32 = 600_000;
const PBKDF2_KEY_LEN: usize = 32;
const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const CORRUPTED: &str = "notes file is corrupted";

/// Derive the AES-256-GCM cipher for `password` and `salt` with PBKDF2.
fn cipher_for(password: &str, salt: &[u8]) -> Result<Aes256Gcm, String> {
    let mut key = Zeroizing::new(vec![0_u8; PBKDF2_KEY_LEN]);
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERS, &mut key);
    Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())
}

/// Encrypt NDJSON note bytes using AES-256-GCM and PBKDF2 key derivation.
pub fn encrypt_notes(plaintext: &[u8], password: &str) -> Result<Vec<u8>, String> {
    let mut rng = rand::rng();
    let mut salt = [0_u8; SALT_LEN];
    rng.fill(&mut salt);

    let cipher = cipher_for(password, &salt)?;
    let mut nonce = [0_u8; NONCE_LEN];
    rng.fill(&mut nonce);

    let ciphertext = cipher
        .encrypt(&Nonce::<Aes256Gcm>::from(nonce), plaintext)
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
    let body = data.strip_prefix(ENCRYPTED_MAGIC).ok_or(CORRUPTED)?;
    let (salt, rest) = body.split_first_chunk::<SALT_LEN>().ok_or(CORRUPTED)?;
    let (nonce, ciphertext) = rest.split_first_chunk::<NONCE_LEN>().ok_or(CORRUPTED)?;
    if ciphertext.len() < TAG_LEN {
        return Err(CORRUPTED.to_string());
    }

    cipher_for(password, salt)?
        .decrypt(&Nonce::<Aes256Gcm>::from(*nonce), ciphertext)
        .map_err(|_| "incorrect password".to_string())
}

/// Quick header check used to detect encrypted files.
pub fn is_encrypted_data(data: &[u8]) -> bool {
    data.len() >= ENCRYPTED_MAGIC.len() && &data[0..ENCRYPTED_MAGIC.len()] == ENCRYPTED_MAGIC
}
