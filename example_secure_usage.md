# Secure Password and Wallet Management

This document demonstrates how to use the secure password management features implemented in nyks-wallet.

## Key Security Features

- **SecretString**: Passwords stored securely in memory
- **Zeroize**: Automatic memory cleanup when secrets go out of scope
- **Secure key derivation**: Using SHA-256 with salt for encryption keys
- **Password validation**: Enforcing minimum security requirements
- **Environment variable support**: `NYKS_WALLET_PASSPHRASE`

## Usage Examples

### 1. Basic Secure Password Functions

```rust
use nyks_wallet::security::{SecurePassword, SecureWalletData};
use secrecy::SecretString;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Method 1: Auto-detect (env var or prompt)
    let password = SecurePassword::get_passphrase()?;
    println!("Got secure password");

    // Method 2: Force prompt with validation
    let new_password = SecurePassword::create_new_passphrase()?;
    println!("Created new validated password");

    // Method 3: Custom prompt
    let custom_password = SecurePassword::get_passphrase_with_prompt(
        "Enter your master password: "
    )?;

    // Method 4: Direct validation
    let test_password = SecretString::new("TestPass123!".to_string());
    SecurePassword::validate_passphrase_strength(&test_password)?;
    println!("Password validation passed");

    Ok(())
}
```

### 2. Environment Variable Setup

```bash
# Set the wallet passphrase in environment (for automation)
export NYKS_WALLET_PASSPHRASE="MySecureWalletPassword123!"

# Now all password prompts will automatically use this
```

### 3. Secure OrderWallet Creation

```rust
use nyks_wallet::relayer_module::order_wallet::OrderWallet;
use nyks_wallet::security::SecurePassword;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Create new wallet with automatic secure setup
    let mut order_wallet = OrderWallet::create_new_with_database("nyks", None).await?;

    // This automatically:
    // 1. Creates a new wallet
    // 2. Prompts for password (with confirmation and validation)
    // 3. Enables database persistence
    // 4. Stores encrypted wallet data

    println!("Secure wallet created and ready to use!");

    // Use the wallet normally - all operations are automatically persisted
    let (tx_result, account_index) = order_wallet.funding_to_trading(1000).await?;

    Ok(())
}
```

### 4. Loading Existing Secure Wallet

```rust
use nyks_wallet::relayer_module::order_wallet::OrderWallet;
use nyks_wallet::zkos_accounts::zkaccount::ZkAccountDB;

#[tokio::main]
async fn main() -> Result<(), String> {
    let wallet_id = "nyks1abcd1234...".to_string(); // Your wallet address

    // Method 1: Auto-prompt for password
    let (wallet_opt, accounts) = OrderWallet::load_from_database_with_prompt(wallet_id.clone())?;

    if let Some(wallet) = wallet_opt {
        // Reconstruct ZkAccountDB
        let mut zk_accounts = ZkAccountDB::new();
        for (index, account) in accounts {
            zk_accounts.accounts.insert(index, account);
        }

        // Create OrderWallet with loaded data
        let mut order_wallet = OrderWallet::new(wallet, zk_accounts, "nyks", None)?;
        order_wallet.enable_database_persistence_with_prompt()?;

        println!("Wallet loaded with {} accounts", order_wallet.zk_accounts.accounts.len());
    }

    Ok(())
}
```

### 5. Manual Password Management

```rust
use nyks_wallet::security::SecurePassword;
use secrecy::{SecretString, ExposeSecret};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create password from environment or prompt
    let password = SecurePassword::get_passphrase()?;

    // Use the password (expose with caution)
    println!("Password length: {}", password.expose_secret().len());

    // Key derivation for encryption
    let salt = b"my_application_salt_12345678901234";
    let derived_key = SecurePassword::derive_key_from_passphrase(&password, salt)?;

    println!("Derived encryption key (first 8 bytes): {:?}", &derived_key[..8]);

    // Password is automatically zeroized when it goes out of scope
    Ok(())
}
```

### 6. Secure Memory Operations

```rust
use nyks_wallet::security::{SecureMemory, SecureCleanup};

fn handle_sensitive_data() {
    // Use secure temporary buffer
    let result = SecureMemory::with_secure_buffer(32, |buffer| {
        // Fill buffer with sensitive data
        buffer.copy_from_slice(b"sensitive_data_here_1234567890ab");

        // Process the data
        buffer.len()
    });

    // Buffer is automatically zeroed after this block
    println!("Processed {} bytes securely", result);

    // Manually clear sensitive vectors
    let mut sensitive_vec = vec![1, 2, 3, 4, 5];
    SecureMemory::clear_vec(sensitive_vec);

    // Clear strings
    let sensitive_string = "secret_data".to_string();
    SecureMemory::clear_string(sensitive_string);
}
```

### 7. Wallet Security Integration

```rust
use nyks_wallet::security::{SecureWalletData, SecureCleanup};
use nyks_wallet::wallet::Wallet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create regular wallet
    let mut wallet = Wallet::create_new_with_random_btc_address().await?;

    // Convert to secure format
    let secure_wallet = SecureWalletData::from_wallet(&wallet);

    // Access sensitive data carefully
    let private_key = secure_wallet.expose_private_key();
    println!("Private key length: {}", private_key.len());

    // Clean up the original wallet
    wallet.secure_cleanup();

    // secure_wallet will be automatically zeroized when dropped
    Ok(())
}
```

## Security Best Practices

### 1. Password Strength

- Minimum 8 characters
- Mix of letters, numbers, and special characters
- Use `create_new_passphrase()` for prompted validation

### 2. Environment Variables

- Use `NYKS_WALLET_PASSPHRASE` for automation
- Ensure proper environment security in production
- Consider using secret management systems

### 3. Memory Management

- Secrets are automatically zeroized on drop
- Use `SecureMemory` utilities for temporary sensitive data
- Minimize exposure of sensitive data in scope

### 4. Database Security

- Wallet data is encrypted with AES-256-GCM
- Keys derived using SHA-256 with salt
- ZkAccount data stored in plaintext (consider your threat model)

## Error Handling

```rust
use nyks_wallet::security::SecurePassword;

fn secure_operation() -> Result<(), Box<dyn std::error::Error>> {
    match SecurePassword::get_passphrase() {
        Ok(password) => {
            println!("Password obtained securely");
            // Use password...
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to get password: {}", e);
            Err(e.into())
        }
    }
}
```

## Testing

The security module includes comprehensive tests:

```bash
# Run security tests
cargo test security::

# Run all tests with security features
cargo test --features sqlite
```

## Integration with Existing Code

The security features are designed to be backward compatible:

- Existing `OrderWallet` code works unchanged
- New secure methods are opt-in
- Environment variable support is automatic
- Database encryption is transparent

This provides a secure foundation for wallet operations while maintaining ease of use.
