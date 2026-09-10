use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rand::{distributions::Alphanumeric, Rng, RngCore};

pub struct CryptoEngine;

impl CryptoEngine {
    pub fn hash_master_password(password: &str) -> Result<String, String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Argon2 hash failure: {}", e))?;
        Ok(hash.to_string())
    }

    pub fn verify_master_password(password: &str, stored_hash: &str) -> bool {
        let parsed_hash = match PasswordHash::new(stored_hash) {
            Ok(h) => h,
            Err(_) => return false,
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    }

    fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], String> {
        let mut key = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), salt, &mut key)
            .map_err(|e| format!("Key derivation error: {}", e))?;
        Ok(key)
    }

    pub fn encrypt_secret(master_key: &str, plaintext: &str) -> Result<(String, String, String), String> {
        let mut salt_bytes = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt_bytes);

        let key_bytes = Self::derive_key(master_key, &salt_bytes)?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| format!("AES encryption error: {}", e))?;

        Ok((
            BASE64.encode(ciphertext),
            BASE64.encode(nonce_bytes),
            BASE64.encode(salt_bytes),
        ))
    }

    pub fn decrypt_secret(
        master_key: &str,
        ciphertext_b64: &str,
        nonce_b64: &str,
        salt_b64: &str,
    ) -> Result<String, String> {
        let salt = BASE64.decode(salt_b64).map_err(|e| e.to_string())?;
        let nonce_bytes = BASE64.decode(nonce_b64).map_err(|e| e.to_string())?;
        let ciphertext = BASE64.decode(ciphertext_b64).map_err(|e| e.to_string())?;

        let key_bytes = Self::derive_key(master_key, &salt)?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
        let nonce = Nonce::from_slice(&nonce_bytes);

        let decrypted = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| "Decryption failed (Invalid master key or corrupted data)".to_string())?;

        String::from_utf8(decrypted).map_err(|e| e.to_string())
    }

    pub fn generate_strong_password(length: usize) -> String {
        let len = length.clamp(12, 64);
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(len)
            .map(char::from)
            .collect()
    }
}
