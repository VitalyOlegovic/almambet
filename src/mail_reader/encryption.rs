use anyhow::{Result};
use std::fs;
use std::path::PathBuf;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;

const PASSWORD_FILE: &str = ".encrypted_password";
const KEY_FILE: &str = ".encryption_key";

pub fn get_encryption_key() -> Result<Aes256Gcm> {
    let key_path = PathBuf::from(KEY_FILE);
    let key = if key_path.exists() {
        // Read existing key
        let key_bytes = fs::read(key_path)?;
        Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to create cipher from key: {}", e))?
    } else {
        // Generate new key
        let mut key_bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut key_bytes);
        fs::write(key_path, key_bytes)?;
        Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to create cipher from new key: {}", e))?
    };
    Ok(key)
}

pub fn encrypt_password(password: &str) -> Result<String> {
    let cipher = get_encryption_key()?;
    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher.encrypt(nonce, password.as_bytes())
        .map_err(|e| anyhow::anyhow!("Failed to encrypt password: {}", e))?;
    
    let mut combined = Vec::new();
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    
    Ok(BASE64.encode(&combined))
}

pub fn decrypt_password(encrypted: &str) -> Result<String> {
    let cipher = get_encryption_key()?;
    let combined = BASE64.decode(encrypted)
        .map_err(|e| anyhow::anyhow!("Failed to decode base64: {}", e))?;
    
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    
    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("Failed to decrypt password: {}", e))?;
    
    String::from_utf8(plaintext)
        .map_err(|e| anyhow::anyhow!("Failed to convert decrypted bytes to string: {}", e))
}

pub fn get_credentials(login: &str) -> Result<(String, String)> {
    let password_path = PathBuf::from(PASSWORD_FILE);
    
    let password = if password_path.exists() {
        // Read and decrypt stored password
        let encrypted = fs::read_to_string(password_path)?;
        decrypt_password(&encrypted)?
    } else {
        // Get new password and store it
        let password = rpassword::prompt_password("Enter your password: ")?;
        let encrypted = encrypt_password(&password)?;
        fs::write(password_path, encrypted)?;
        password
    };
    
    Ok((login.to_string(), password))
} 