# Database Persistence Usage Example

This example shows how to use the new database persistence features with OrderWallet.

## Features

- **SQLite** (default) and **PostgreSQL** support
- Encrypted wallet storage with password protection
- Automatic persistence of ZkAccount operations
- Drop trait implementation for automatic data saving

## Setup

### Using SQLite (Default)

```toml
# Cargo.toml
[features]
default = ["sqlite"]
```

```bash
# Optional: Set custom database path
export DATABASE_URL="./my_wallet_data.db"
```

### Using PostgreSQL

```toml
# Cargo.toml
[features]
default = ["postgresql"]
```

```bash
# Required: Set PostgreSQL connection string
export DATABASE_URL="postgresql://username:password@localhost/wallet_db"
```

## Usage Examples

### 1. Basic Setup with Database Persistence

```rust
use nyks_wallet::relayer_module::order_wallet::OrderWallet;
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Create or load wallet
    let wallet = Wallet::create_new_with_random_btc_address().await?;
    let zk_accounts = ZkAccountDB::new();

    // Create OrderWallet
    let mut order_wallet = OrderWallet::new(wallet, zk_accounts, "nyks", None)?;

    // Enable database persistence with password
    let password = Some("my_secure_password".to_string());
    order_wallet.enable_database_persistence(password)?;

    // All ZkAccount operations are now automatically persisted!
    let (tx_result, account_index) = order_wallet.funding_to_trading(1000).await?;

    // Data is automatically saved when OrderWallet is dropped
    Ok(())
}
```

### 2. Loading from Database

```rust
use nyks_wallet::relayer_module::order_wallet::OrderWallet;

#[tokio::main]
async fn main() -> Result<(), String> {
    let wallet_id = "nyks1abcd...".to_string(); // Wallet twilight address
    let password = Some("my_secure_password".to_string());

    // Load wallet and accounts from database
    let (wallet_opt, zk_accounts_map) = OrderWallet::load_from_database(wallet_id, password)?;

    if let Some(wallet) = wallet_opt {
        // Reconstruct ZkAccountDB from loaded accounts
        let mut zk_accounts = ZkAccountDB::new();
        for (index, account) in zk_accounts_map {
            zk_accounts.accounts.insert(index, account);
        }

        // Create OrderWallet with loaded data
        let mut order_wallet = OrderWallet::new(wallet, zk_accounts, "nyks", None)?;
        order_wallet.enable_database_persistence(Some("my_secure_password".to_string()))?;

        // Continue operations...
        println!("Loaded {} accounts from database", order_wallet.zk_accounts.accounts.len());
    }

    Ok(())
}
```

### 3. Retrieving All ZkAccounts from Database

```rust
// After enabling database persistence
let all_accounts = order_wallet.get_all_zk_accounts_from_db()?;
for (index, account) in all_accounts {
    println!("Account {}: balance = {}, on_chain = {}",
             index, account.balance, account.on_chain);
}
```

### 4. Manual Database Operations

```rust
// Remove a specific account from database
order_wallet.remove_zk_account_from_db(account_index)?;

// Note: This only removes from database, not from the in-memory ZkAccountDB
// To remove from both, use the ZkAccountDB methods which will trigger database updates
order_wallet.zk_accounts.remove_account_by_index(&account_index)?;
```

### 5. Secure Password Management

```rust
use nyks_wallet::security::{SecurePassword, SecureWalletData};
use secrecy::SecretString;

// Option 1: Automatic (tries env var, then prompts)
let password = SecurePassword::get_passphrase()?;
order_wallet.enable_database_persistence(Some(password))?;

// Option 2: Direct environment variable
std::env::set_var("NYKS_WALLET_PASSPHRASE", "my_secure_password");
let password = SecurePassword::get_passphrase()?; // Will use env var

// Option 3: Interactive prompt with validation
let password = SecurePassword::create_new_passphrase()?; // Prompts with confirmation

// Option 4: Custom prompt
let password = SecurePassword::get_passphrase_with_prompt("Enter your wallet key: ")?;

// Option 5: Simple method (auto-prompt)
order_wallet.enable_database_persistence_with_prompt()?;
```

### 6. Secure Wallet Creation

```rust
// Create new wallet with automatic secure setup
let order_wallet = OrderWallet::create_new_with_database("nyks", None).await?;
// This will:
// 1. Create a new wallet
// 2. Prompt for secure password (with confirmation)
// 3. Enable database persistence automatically

// Load wallet with automatic password prompt
let (wallet, accounts) = OrderWallet::load_from_database_with_prompt(wallet_id)?;
```

## Automatic Persistence

The following operations automatically update the database:

1. **generate_new_account()** - Saves new ZkAccount
2. **update_balance()** - Updates account balance
3. **update_io_type()** - Updates account IOType
4. **update_on_chain()** - Updates on-chain status
5. **remove_account()** - Removes account from database

## Database Schema

### ZkAccounts Table

- `id` - Primary key
- `wallet_id` - Wallet identifier (twilight address)
- `account_index` - ZkAccount index
- `qq_address` - Encrypted account address
- `balance` - Account balance
- `account` - Account string
- `scalar` - Account scalar
- `io_type_value` - IOType (0=Coin, 1=Memo)
- `on_chain` - On-chain status
- `created_at`, `updated_at` - Timestamps

### Encrypted Wallets Table

- `id` - Primary key
- `wallet_id` - Wallet identifier (unique)
- `encrypted_data` - AES-256-GCM encrypted wallet data
- `salt` - Encryption salt
- `nonce` - Encryption nonce
- `created_at`, `updated_at` - Timestamps

## Security

- Wallet data is encrypted using AES-256-GCM
- Keys are derived using SHA-256 with salt
- Database only stores encrypted wallet data
- ZkAccount data is stored in plaintext (consider your security requirements)

## Error Handling

```rust
match order_wallet.enable_database_persistence(password) {
    Ok(()) => println!("Database persistence enabled"),
    Err(e) => eprintln!("Failed to enable database persistence: {}", e),
}
```

## Migration from JSON Files

If you have existing ZkAccount data in JSON format:

```rust
// Load existing JSON data
let zk_accounts = ZkAccountDB::import_from_json("ZkAccounts.json")?;

// Create OrderWallet and enable database persistence
let mut order_wallet = OrderWallet::new(wallet, zk_accounts, "nyks", None)?;
order_wallet.enable_database_persistence(password)?;

// All existing accounts are now migrated to database!
```
