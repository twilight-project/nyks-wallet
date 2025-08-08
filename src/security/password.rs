use anyhow::Result;
use secrecy::{ExposeSecret, SecretString};
use std::env;
use zeroize::Zeroize;

/// Secure password management for wallet operations
pub struct SecurePassword;

impl SecurePassword {
    /// Get passphrase from environment variable or prompt user
    ///
    /// Priority:
    /// 1. Environment variable: NYKS_WALLET_PASSPHRASE
    /// 2. Interactive prompt (secure input)
    pub fn get_passphrase() -> Result<SecretString> {
        // Try environment variable first
        if let Ok(passphrase) = env::var("NYKS_WALLET_PASSPHRASE") {
            // Create SecretString and immediately zeroize the original
            let mut passphrase_mut = passphrase;
            let secret = SecretString::new(passphrase_mut.clone());
            passphrase_mut.zeroize();
            return Ok(secret);
        }

        // Fall back to interactive prompt
        Self::prompt_passphrase()
    }

    /// Prompt user for passphrase with secure input
    pub fn prompt_passphrase() -> Result<SecretString> {
        let pass = rpassword::prompt_password("Wallet encryption password: ")?;
        Ok(SecretString::new(pass))
    }

    /// Get passphrase for specific operation with custom prompt
    pub fn get_passphrase_with_prompt(prompt: &str) -> Result<SecretString> {
        // Try environment variable first
        if let Ok(passphrase) = env::var("NYKS_WALLET_PASSPHRASE") {
            let mut passphrase_mut = passphrase;
            let secret = SecretString::new(passphrase_mut.clone());
            passphrase_mut.zeroize();
            return Ok(secret);
        }

        // Use custom prompt
        let pass = rpassword::prompt_password(prompt)?;
        Ok(SecretString::new(pass))
    }

    /// Derive a key from the passphrase using a secure method
    pub fn derive_key_from_passphrase(passphrase: &SecretString, salt: &[u8]) -> Result<[u8; 32]> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(passphrase.expose_secret().as_bytes());
        hasher.update(salt);
        let key_bytes = hasher.finalize();

        Ok(key_bytes.into())
    }

    /// Validate passphrase strength (basic checks)
    pub fn validate_passphrase_strength(passphrase: &SecretString) -> Result<()> {
        let pass = passphrase.expose_secret();

        if pass.len() < 8 {
            return Err(anyhow::anyhow!(
                "Passphrase must be at least 8 characters long"
            ));
        }

        if pass.chars().all(|c| c.is_alphabetic()) {
            return Err(anyhow::anyhow!(
                "Passphrase should contain numbers or special characters"
            ));
        }

        Ok(())
    }

    /// Create a new passphrase with confirmation
    pub fn create_new_passphrase() -> Result<SecretString> {
        loop {
            let pass1 = Self::get_passphrase_with_prompt("Enter new wallet password: ")?;

            // Validate strength
            if let Err(e) = Self::validate_passphrase_strength(&pass1) {
                eprintln!("Password validation failed: {}", e);
                continue;
            }

            let pass2 = Self::get_passphrase_with_prompt("Confirm wallet password: ")?;

            if pass1.expose_secret() == pass2.expose_secret() {
                return Ok(pass1);
            } else {
                eprintln!("Passwords do not match. Please try again.");
            }
        }
    }
}

/// Wrapper for secure string operations
pub trait SecureStringExt {
    /// Safely compare two secret strings
    fn secure_eq(&self, other: &Self) -> bool;
}

impl SecureStringExt for SecretString {
    fn secure_eq(&self, other: &Self) -> bool {
        use subtle::ConstantTimeEq;
        self.expose_secret()
            .as_bytes()
            .ct_eq(other.expose_secret().as_bytes())
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_derivation() {
        let passphrase = SecretString::new("test_password_123".to_string());
        let salt = b"test_salt_bytes_";

        let key1 = SecurePassword::derive_key_from_passphrase(&passphrase, salt).unwrap();
        let key2 = SecurePassword::derive_key_from_passphrase(&passphrase, salt).unwrap();

        assert_eq!(
            key1, key2,
            "Same passphrase and salt should produce same key"
        );

        let different_salt = b"different_salt__";
        let key3 = SecurePassword::derive_key_from_passphrase(&passphrase, different_salt).unwrap();

        assert_ne!(key1, key3, "Different salt should produce different key");
    }

    #[test]
    fn test_passphrase_validation() {
        let weak = SecretString::new("weak".to_string());
        assert!(SecurePassword::validate_passphrase_strength(&weak).is_err());

        let alpha_only = SecretString::new("onlyletters".to_string());
        assert!(SecurePassword::validate_passphrase_strength(&alpha_only).is_err());

        let strong = SecretString::new("MySecurePass123!".to_string());
        assert!(SecurePassword::validate_passphrase_strength(&strong).is_ok());
    }
}
