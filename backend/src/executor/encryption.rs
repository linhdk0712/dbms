use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use aes_gcm::aead::AeadCore;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::DbCredentials;

/// JSON structure lưu credentials trước khi encrypt.
#[derive(Debug, Serialize, Deserialize)]
struct CredentialsPayload {
    username: String,
    password: String,
}

/// Encrypt credentials thành base64 string để lưu vào DB.
/// Format: base64(nonce_12_bytes || ciphertext)
///
/// # Security
/// - AES-256-GCM: authenticated encryption
/// - Random 96-bit nonce cho mỗi lần encrypt (không reuse)
/// - Key phải là 32 bytes (256 bit)
pub fn encrypt_credentials(
    username: &str,
    password: &str,
    key_bytes: &[u8; 32],
) -> Result<String> {
    let payload = CredentialsPayload {
        username: username.to_string(),
        password: password.to_string(),
    };
    let plaintext = serde_json::to_vec(&payload).context("failed to serialize credentials")?;

    let cipher = Aes256Gcm::new(key_bytes.into());
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_ref())
        .map_err(|e| anyhow::anyhow!("encryption failed: {e}"))?;

    // Concatenate: nonce (12 bytes) || ciphertext
    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);

    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &combined,
    ))
}

/// Decrypt credentials từ base64 string.
/// Trả về DbCredentials chứa username/password plaintext.
///
/// # Security
/// - Chỉ gọi khi cần execute SQL — không giữ plaintext lâu
/// - Key phải khớp với key đã dùng khi encrypt
pub fn decrypt_credentials(
    encrypted_base64: &str,
    key_bytes: &[u8; 32],
) -> Result<DbCredentials> {
    let combined = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        encrypted_base64,
    )
    .context("invalid base64 in encrypted credentials")?;

    if combined.len() < 12 {
        anyhow::bail!("encrypted data too short — expected at least 12 bytes for nonce");
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new(key_bytes.into());

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("decryption failed (wrong key or tampered data): {e}"))?;

    let payload: CredentialsPayload =
        serde_json::from_slice(&plaintext).context("failed to deserialize decrypted credentials")?;

    Ok(DbCredentials {
        username: payload.username,
        password: payload.password,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        // 32 bytes key cho test
        let hex_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let bytes = hex::decode(hex_key).unwrap();
        bytes.try_into().unwrap()
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = test_key();
        let encrypted = encrypt_credentials("admin", "s3cret_p@ss!", &key).unwrap();

        // Encrypted output should be base64
        assert!(!encrypted.is_empty());
        assert_ne!(encrypted, "admin");

        let decrypted = decrypt_credentials(&encrypted, &key).unwrap();
        assert_eq!(decrypted.username, "admin");
        assert_eq!(decrypted.password, "s3cret_p@ss!");
    }

    #[test]
    fn test_different_nonce_each_time() {
        let key = test_key();
        let enc1 = encrypt_credentials("user", "pass", &key).unwrap();
        let enc2 = encrypt_credentials("user", "pass", &key).unwrap();

        // Same input but different nonce → different ciphertext
        assert_ne!(enc1, enc2);
    }

    #[test]
    fn test_wrong_key_fails_decrypt() {
        let key = test_key();
        let encrypted = encrypt_credentials("admin", "password", &key).unwrap();

        let wrong_key: [u8; 32] = [0xFF; 32];
        let result = decrypt_credentials(&encrypted, &wrong_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_data_fails_decrypt() {
        let key = test_key();
        let encrypted = encrypt_credentials("admin", "password", &key).unwrap();

        // Tamper last character
        let mut tampered = encrypted.clone();
        tampered.push('A');
        // This might or might not fail on base64 decode depending on padding
        // but definitely should fail on decrypt
        let result = decrypt_credentials(&tampered, &key);
        // Either base64 decode fails or GCM auth fails
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_credentials() {
        let key = test_key();
        let encrypted = encrypt_credentials("", "", &key).unwrap();
        let decrypted = decrypt_credentials(&encrypted, &key).unwrap();
        assert_eq!(decrypted.username, "");
        assert_eq!(decrypted.password, "");
    }

    #[test]
    fn test_unicode_credentials() {
        let key = test_key();
        let encrypted = encrypt_credentials("用户", "密码🔒", &key).unwrap();
        let decrypted = decrypt_credentials(&encrypted, &key).unwrap();
        assert_eq!(decrypted.username, "用户");
        assert_eq!(decrypted.password, "密码🔒");
    }
}
