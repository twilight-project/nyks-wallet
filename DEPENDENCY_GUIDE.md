# Dependency Configuration Guide

This guide explains the dependency configuration in `Cargo.toml` and how to handle different database scenarios.

## Features Configuration

```toml
[features]
default = ["sqlite"]                                                    # Default to SQLite
sqlite = ["diesel/sqlite", "diesel_migrations", "libsqlite3-sys"]      # System SQLite
postgresql = ["diesel/postgres", "diesel_migrations", "pq-sys"]        # System PostgreSQL
sqlite-bundled = ["diesel/sqlite", "diesel_migrations", "libsqlite3-sys/bundled"]  # Bundled SQLite
validator-wallet = []                                                   # Existing validator feature
```

## Dependencies

```toml
[dependencies]
# Core dependencies (always included)
anyhow = "1.0"
secrecy = "0.8"               # Secure string handling
zeroize = "1.7"               # Memory zeroing
subtle = "2.5"                # Constant-time comparisons
aes-gcm = "0.10"             # Encryption
rand_core = "0.6"            # Random number generation

# Database dependencies (optional, feature-gated)
diesel = { version = "2.1", features = ["chrono"], optional = true }
diesel_migrations = { version = "2.1", optional = true }

# System database libraries (optional)
libsqlite3-sys = { version = "0.27", optional = true }    # SQLite system library
pq-sys = { version = "0.4", optional = true }             # PostgreSQL system library
```

## Usage Scenarios

### 1. Bundled SQLite (Recommended for Development)

```bash
# No system dependencies required - SQLite is statically linked
cargo build --features sqlite-bundled
cargo test --features sqlite-bundled
```

**Pros:**

- No system dependencies needed
- Works on any system
- Easy for CI/CD and development

**Cons:**

- Slightly larger binary size
- May not use the latest SQLite version

### 2. System SQLite

```bash
# Requires: sudo apt install libsqlite3-dev (Ubuntu/Debian)
# Or: brew install sqlite (macOS)
cargo build --features sqlite
cargo test --features sqlite
```

**Pros:**

- Smaller binary size
- Uses system's SQLite version
- Better for production deployments

**Cons:**

- Requires system dependencies

### 3. PostgreSQL

```bash
# Requires: sudo apt install libpq-dev (Ubuntu/Debian)
# Or: brew install postgresql (macOS)
cargo build --features postgresql
cargo test --features postgresql
```

**Pros:**

- Production-grade database
- Better for multi-user scenarios
- Advanced features

**Cons:**

- Requires PostgreSQL server setup
- More complex deployment

### 4. No Database Features

```bash
# Build without any database features
cargo build --no-default-features
```

**Use case:** When you only need core wallet functionality without persistence.

## Dependency Installation by Platform

### Ubuntu/Debian

```bash
# For system SQLite
sudo apt update && sudo apt install libsqlite3-dev pkg-config

# For PostgreSQL
sudo apt update && sudo apt install libpq-dev postgresql postgresql-contrib

# For both
sudo apt update && sudo apt install libsqlite3-dev libpq-dev pkg-config
```

### CentOS/RHEL/Fedora

```bash
# For system SQLite
sudo yum install sqlite-devel  # CentOS/RHEL
sudo dnf install sqlite-devel  # Fedora

# For PostgreSQL
sudo yum install postgresql-devel  # CentOS/RHEL
sudo dnf install postgresql-devel  # Fedora
```

### macOS

```bash
# Using Homebrew
brew install sqlite postgresql pkg-config
```

### Windows

```powershell
# Using vcpkg
vcpkg install sqlite3 libpq

# Or download pre-built libraries and set environment variables
```

## Environment Variables for Build

```bash
# For custom library paths
export PKG_CONFIG_PATH="/usr/local/lib/pkgconfig"
export SQLITE3_LIB_DIR="/usr/local/lib"
export SQLITE3_INCLUDE_DIR="/usr/local/include"

# For PostgreSQL
export PQ_LIB_DIR="/usr/local/lib"
export PQ_INCLUDE_DIR="/usr/local/include"
```

## Docker Configuration

For Docker builds, use bundled SQLite to avoid system dependencies:

```dockerfile
FROM rust:1.70 as builder

WORKDIR /app
COPY . .

# Build with bundled SQLite (no system deps needed)
RUN cargo build --release --features sqlite-bundled

FROM debian:bullseye-slim
COPY --from=builder /app/target/release/nyks-wallet /usr/local/bin/
```

## CI/CD Configuration

### GitHub Actions

```yaml
- name: Build with bundled SQLite
  run: cargo build --features sqlite-bundled

- name: Run tests
  run: cargo test --features sqlite-bundled
```

### With system dependencies:

```yaml
- name: Install dependencies
  run: sudo apt-get update && sudo apt-get install -y libsqlite3-dev libpq-dev

- name: Build and test
  run: |
    cargo build --features sqlite
    cargo test --features sqlite
```

## Troubleshooting

### 1. SQLite linking errors

```
error: linking with `cc` failed: exit status: 1
note: rust-lld: error: unable to find library -lsqlite3
```

**Solution:** Use bundled SQLite or install system dependencies:

```bash
cargo build --features sqlite-bundled
# OR
sudo apt install libsqlite3-dev
```

### 2. PostgreSQL linking errors

```
error: could not find native static library `pq`
```

**Solution:** Install PostgreSQL development libraries:

```bash
sudo apt install libpq-dev
```

### 3. pkg-config errors

```
error: could not run `pkg-config`
```

**Solution:** Install pkg-config:

```bash
sudo apt install pkg-config
```

## Production Recommendations

1. **Development:** Use `sqlite-bundled` for simplicity
2. **Testing/CI:** Use `sqlite-bundled` to avoid dependency management
3. **Production (single-user):** Use `sqlite` with system libraries
4. **Production (multi-user):** Use `postgresql` with proper database server

## Feature Combinations

```bash
# Database + Validator wallet
cargo build --features "sqlite-bundled,validator-wallet"

# Multiple features
cargo build --features "postgresql,validator-wallet"

# All features
cargo build --all-features
```

This configuration provides maximum flexibility while handling the complexities of database system dependencies.
