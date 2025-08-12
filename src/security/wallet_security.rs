use secrecy::{ExposeSecret, Secret, SecretString};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Secure wrapper for sensitive wallet data
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecureWalletData {
    /// Private key - automatically zeroized on drop
    #[zeroize(skip)] // We'll handle this manually for Secret
    pub private_key: Secret<Vec<u8>>,

    /// Public key (less sensitive, but still secured)
    pub public_key: Vec<u8>,

    /// Twilight address (public, not sensitive)
    pub twilight_address: String,

    /// BTC address (public, not sensitive)  
    pub btc_address: String,

    /// Seed phrase or mnemonic - highly sensitive
    #[zeroize(skip)] // We'll handle this manually for SecretString
    pub seed_data: Option<SecretString>,
}

impl SecureWalletData {
    /// Create new secure wallet data
    pub fn new(
        private_key: Vec<u8>,
        public_key: Vec<u8>,
        twilight_address: String,
        btc_address: String,
        seed_data: Option<String>,
    ) -> Self {
        Self {
            private_key: Secret::new(private_key),
            public_key,
            twilight_address,
            btc_address,
            seed_data: seed_data.map(SecretString::new),
        }
    }

    /// Get private key (use with caution)
    pub fn expose_private_key(&self) -> &[u8] {
        self.private_key.expose_secret()
    }

    /// Get seed data (use with caution)
    pub fn expose_seed(&self) -> Option<&str> {
        self.seed_data.as_ref().map(|s| s.expose_secret().as_str())
    }

    /// Create from existing wallet for backward compatibility
    pub fn from_wallet(wallet: &crate::wallet::Wallet) -> Self {
        Self::new(
            wallet.private_key.clone(),
            wallet.public_key.clone(),
            wallet.twilightaddress.clone(),
            wallet.btc_address.clone(),
            None, // Seed not available in old wallet format
        )
    }

    /// Convert to serializable format (for encryption)
    pub fn to_serializable(&self) -> SerializableSecureWallet {
        SerializableSecureWallet {
            private_key: self.private_key.expose_secret().clone(),
            public_key: self.public_key.clone(),
            twilight_address: self.twilight_address.clone(),
            btc_address: self.btc_address.clone(),
            seed_data: self.seed_data.as_ref().map(|s| s.expose_secret().clone()),
        }
    }

    /// Create from serializable format (after decryption)
    pub fn from_serializable(data: SerializableSecureWallet) -> Self {
        Self::new(
            data.private_key.clone(),
            data.public_key.clone(),
            data.twilight_address.clone(),
            data.btc_address.clone(),
            data.seed_data.clone(),
        )
    }
}

/// Serializable version for encryption/decryption
/// This will be zeroized after use
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct SerializableSecureWallet {
    pub private_key: Vec<u8>,
    pub public_key: Vec<u8>,
    pub twilight_address: String,
    pub btc_address: String,
    pub seed_data: Option<String>,
}

/// Secure memory manager for sensitive operations
pub struct SecureMemory;

impl SecureMemory {
    /// Securely clear a vector
    pub fn clear_vec<T: Zeroize>(mut vec: Vec<T>) {
        vec.zeroize();
        // Vector will be dropped here, further clearing memory
    }

    /// Securely clear a string
    pub fn clear_string(mut string: String) {
        string.zeroize();
        // String will be dropped here
    }

    /// Create a secure temporary buffer that auto-clears
    pub fn with_secure_buffer<T, F>(size: usize, f: F) -> T
    where
        F: FnOnce(&mut Vec<u8>) -> T,
    {
        let mut buffer = vec![0u8; size];
        let result = f(&mut buffer);
        buffer.zeroize();
        result
    }
}

/// Trait for types that need secure cleanup
pub trait SecureCleanup {
    /// Perform secure cleanup of sensitive data
    fn secure_cleanup(&mut self);
}

impl SecureCleanup for crate::wallet::Wallet {
    fn secure_cleanup(&mut self) {
        // Zeroize sensitive fields
        self.private_key.zeroize();

        // Note: We don't zeroize public data like addresses
        // as they're not sensitive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_wallet_data() {
        let private_key = vec![1, 2, 3, 4, 5];
        let public_key = vec![6, 7, 8, 9, 10];
        let address = "twilight123".to_string();
        let btc_addr = "bc1q123".to_string();
        let seed = Some("test seed phrase".to_string());

        let secure_wallet = SecureWalletData::new(
            private_key.clone(),
            public_key.clone(),
            address.clone(),
            btc_addr.clone(),
            seed.clone(),
        );

        assert_eq!(secure_wallet.expose_private_key(), &private_key);
        assert_eq!(secure_wallet.public_key, public_key);
        assert_eq!(secure_wallet.twilight_address, address);
        assert_eq!(secure_wallet.btc_address, btc_addr);
        assert_eq!(secure_wallet.expose_seed(), seed.as_deref());
    }

    #[test]
    fn test_serialization() {
        let secure_wallet = SecureWalletData::new(
            vec![1, 2, 3],
            vec![4, 5, 6],
            "addr1".to_string(),
            "btc1".to_string(),
            Some("seed".to_string()),
        );

        let serializable = secure_wallet.to_serializable();
        let recovered = SecureWalletData::from_serializable(serializable);

        assert_eq!(
            recovered.expose_private_key(),
            secure_wallet.expose_private_key()
        );
        assert_eq!(recovered.public_key, secure_wallet.public_key);
    }
}
