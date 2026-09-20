use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rand::{rngs::OsRng as RandOsRng, Rng, RngCore};

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

    /// Trích xuất Master Key (32 bytes) duy nhất từ password và salt toàn cục
    pub fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], String> {
        let mut key = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), salt, &mut key)
            .map_err(|e| format!("Key derivation error: {}", e))?;
        Ok(key)
    }

    /// Mã hóa secret với AES-256-GCM sử dụng khóa đã dẫn xuất từ trước
    pub fn encrypt_with_derived_key(key: &[u8; 32], plaintext: &str) -> Result<(String, String), String> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

        let mut nonce_bytes = [0u8; 12];
        RandOsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| format!("AES encryption error: {}", e))?;

        Ok((BASE64.encode(ciphertext), BASE64.encode(nonce_bytes)))
    }

    /// Giải mã siêu tốc (micro-second) khi Master Key đã sẵn sàng
    pub fn decrypt_with_derived_key(
        key: &[u8; 32],
        ciphertext_b64: &str,
        nonce_b64: &str,
    ) -> Result<String, String> {
        let nonce_bytes = BASE64.decode(nonce_b64).map_err(|e| e.to_string())?;
        let ciphertext = BASE64.decode(ciphertext_b64).map_err(|e| e.to_string())?;

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce = Nonce::from_slice(&nonce_bytes);

        let decrypted = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| "Decryption failed (Corrupted data or wrong key)".to_string())?;

        String::from_utf8(decrypted).map_err(|e| e.to_string())
    }

    pub fn generate_strong_password(length: usize) -> String {
        let len = length.clamp(12, 64);
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;:,.<>?";
        let mut rng = RandOsRng;
        (0..len)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}
