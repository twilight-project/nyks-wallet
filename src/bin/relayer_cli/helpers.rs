use nyks_wallet::relayer_module::order_wallet::OrderWallet;
use nyks_wallet::relayer_module::relayer_types::OrderStatus;
use secrecy::SecretString;

// ---------------------------------------------------------------------------
// MaybeOwnedWallet — allows handlers to work with either an owned wallet
// (loaded fresh from DB) or a borrowed reference from the REPL session.
// ---------------------------------------------------------------------------

pub(crate) enum MaybeOwnedWallet<'a> {
    Owned(OrderWallet),
    Borrowed(&'a mut OrderWallet),
}

impl std::ops::Deref for MaybeOwnedWallet<'_> {
    type Target = OrderWallet;
    fn deref(&self) -> &OrderWallet {
        match self {
            Self::Owned(w) => w,
            Self::Borrowed(w) => w,
        }
    }
}

impl std::ops::DerefMut for MaybeOwnedWallet<'_> {
    fn deref_mut(&mut self) -> &mut OrderWallet {
        match self {
            Self::Owned(w) => w,
            Self::Borrowed(w) => w,
        }
    }
}

/// Resolve an `OrderWallet` — use the REPL wallet if provided, otherwise load from DB.
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
pub(crate) async fn get_or_resolve_wallet<'a>(
    repl_wallet: Option<&'a mut OrderWallet>,
    wallet_id: Option<String>,
    password: Option<String>,
) -> Result<MaybeOwnedWallet<'a>, String> {
    match repl_wallet {
        Some(w) => Ok(MaybeOwnedWallet::Borrowed(w)),
        None => Ok(MaybeOwnedWallet::Owned(
            resolve_order_wallet(wallet_id, password).await?,
        )),
    }
}

#[cfg(not(any(feature = "sqlite", feature = "postgresql")))]
pub(crate) async fn get_or_resolve_wallet<'a>(
    repl_wallet: Option<&'a mut OrderWallet>,
    _wallet_id: Option<String>,
    _password: Option<String>,
) -> Result<MaybeOwnedWallet<'a>, String> {
    match repl_wallet {
        Some(w) => Ok(MaybeOwnedWallet::Borrowed(w)),
        None => Ok(MaybeOwnedWallet::Owned(
            OrderWallet::new(None).map_err(|e| e.to_string())?,
        )),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(crate) fn parse_order_type(s: &str) -> Result<twilight_client_sdk::relayer_types::OrderType, String> {
    match s.to_uppercase().as_str() {
        "MARKET" => Ok(twilight_client_sdk::relayer_types::OrderType::MARKET),
        "LIMIT" => Ok(twilight_client_sdk::relayer_types::OrderType::LIMIT),
        "SLTP" => Ok(twilight_client_sdk::relayer_types::OrderType::SLTP),
        other => Err(format!(
            "Unknown order type: {other}. Use MARKET, LIMIT, or SLTP"
        )),
    }
}

pub(crate) fn parse_position_type(
    s: &str,
) -> Result<twilight_client_sdk::relayer_types::PositionType, String> {
    match s.to_uppercase().as_str() {
        "LONG" => Ok(twilight_client_sdk::relayer_types::PositionType::LONG),
        "SHORT" => Ok(twilight_client_sdk::relayer_types::PositionType::SHORT),
        other => Err(format!("Unknown position side: {other}. Use LONG or SHORT")),
    }
}

/// Parse a date string (RFC3339 or YYYY-MM-DD) into a `DateTime<Utc>`.
pub(crate) fn parse_datetime(s: &str) -> Result<chrono::DateTime<chrono::Utc>, String> {
    use chrono::{NaiveDate, TimeZone, Utc};
    // Try RFC3339 first
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    // Try YYYY-MM-DD
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).ok_or("invalid date")?));
    }
    Err(format!(
        "Invalid date '{}'. Use RFC3339 (2024-01-15T00:00:00Z) or YYYY-MM-DD (2024-01-15)",
        s
    ))
}

/// Parse a candle interval string into the Interval enum.
pub(crate) fn parse_interval(s: &str) -> Result<nyks_wallet::relayer_module::relayer_types::Interval, String> {
    use nyks_wallet::relayer_module::relayer_types::Interval;
    match s.to_lowercase().as_str() {
        "1m" | "1min" => Ok(Interval::ONE_MINUTE),
        "5m" | "5min" => Ok(Interval::FIVE_MINUTE),
        "15m" | "15min" => Ok(Interval::FIFTEEN_MINUTE),
        "30m" | "30min" => Ok(Interval::THIRTY_MINUTE),
        "1h" => Ok(Interval::ONE_HOUR),
        "4h" => Ok(Interval::FOUR_HOUR),
        "8h" => Ok(Interval::EIGHT_HOUR),
        "12h" => Ok(Interval::TWELVE_HOUR),
        "1d" => Ok(Interval::ONE_DAY),
        other => Err(format!(
            "Unknown interval: {}. Use: 1m, 5m, 15m, 30m, 1h, 4h, 8h, 12h, 1d",
            other
        )),
    }
}

/// Parse an order status string into the OrderStatus enum.
pub(crate) fn parse_order_status(s: &str) -> Result<OrderStatus, String> {
    match s.to_uppercase().as_str() {
        "PENDING" => Ok(OrderStatus::PENDING),
        "FILLED" => Ok(OrderStatus::FILLED),
        "SETTLED" => Ok(OrderStatus::SETTLED),
        "CANCELLED" => Ok(OrderStatus::CANCELLED),
        "LENDED" => Ok(OrderStatus::LENDED),
        "LIQUIDATE" => Ok(OrderStatus::LIQUIDATE),
        other => Err(format!(
            "Unknown order status: {}. Use: PENDING, FILLED, SETTLED, CANCELLED, LENDED, LIQUIDATE",
            other
        )),
    }
}

/// Build an `OrderWallet` from DB. Password falls back to `NYKS_WALLET_PASSPHRASE` env var.
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
pub(crate) fn load_order_wallet_from_db(
    wallet_id: &str,
    password: Option<String>,
    db_url: Option<String>,
) -> Result<OrderWallet, String> {
    let pwd = resolve_password(password).map(|p| SecretString::new(p.into()));
    OrderWallet::load_from_db(wallet_id.to_string(), pwd, db_url)
}

// ---------------------------------------------------------------------------
// Session password cache  (~/.cache/nyks-wallet/session-<ppid>)
// ---------------------------------------------------------------------------
//
// The file is named by the *parent* shell's PID.  Before trusting the cached
// value we verify that PID is still alive via kill(pid, 0) – so when the
// terminal is closed and the shell exits, subsequent invocations find the
// parent dead and silently discard the stale file.
//
// Security model: the file lives in ~/.cache/nyks-wallet/ (mode 0700) and is
// itself mode 0600 – the same protection as ~/.ssh/id_rsa.  No other process
// owned by the same user can read it.

#[cfg(unix)]
fn get_ppid() -> Option<u32> {
    // Use libc::getppid() which works on both Linux and macOS.
    // The previous /proc/self/status approach only worked on Linux.
    let ppid = unsafe { libc::getppid() };
    if ppid > 0 {
        Some(ppid as u32)
    } else {
        None
    }
}

#[cfg(unix)]
fn is_process_alive(pid: u32) -> bool {
    // Signal 0 checks process existence without sending a real signal.
    // Works on both Linux and macOS (unlike /proc/{pid} which is Linux-only).
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

fn session_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(
        std::path::PathBuf::from(home)
            .join(".cache")
            .join("nyks-wallet"),
    )
}

#[cfg(unix)]
fn session_file_path(ppid: u32) -> Option<std::path::PathBuf> {
    Some(session_dir()?.join(format!("session-{ppid}.lock")))
}

/// Save wallet_id and password to session cache, bound to the current shell (PPID).
#[cfg(unix)]
pub(crate) fn session_save(wallet_id: &str, password: &str) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let ppid = get_ppid().ok_or("cannot determine parent shell PID")?;
    let dir = session_dir().ok_or("cannot determine home directory")?;

    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| e.to_string())?;

    let path = session_file_path(ppid).ok_or("cannot build session file path")?;
    let content = format!("{ppid}\n{wallet_id}\n{password}");
    // Create with 0o600 atomically to avoid a TOCTOU window where the file
    // is briefly world-readable under the default umask.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)
        .map_err(|e| e.to_string())?;
    file.write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Load wallet_id and password from session cache; returns None if shell is gone or cache is missing.
#[cfg(unix)]
pub(crate) fn session_load() -> Option<(String, String)> {
    let ppid = get_ppid()?;
    if !is_process_alive(ppid) {
        session_clear_for(ppid); // clean up stale file
        return None;
    }
    let path = session_file_path(ppid)?;
    let content = std::fs::read_to_string(&path).ok()?;
    let mut lines = content.splitn(3, '\n');
    let stored = lines.next()?;
    if stored.trim().parse::<u32>().ok()? != ppid {
        return None; // sanity-check: file belongs to this shell
    }
    let wallet_id = lines.next()?.to_string();
    let password = lines.next()?.to_string();
    Some((wallet_id, password))
}

/// Load only the password from session cache.
#[cfg(unix)]
pub(crate) fn session_load_password() -> Option<String> {
    session_load().map(|(_, p)| p)
}

/// Load only the wallet_id from session cache.
#[cfg(unix)]
pub(crate) fn session_load_wallet_id() -> Option<String> {
    session_load().map(|(w, _)| w)
}

/// Zeroize and delete the session file for the current shell.
#[cfg(unix)]
pub(crate) fn session_clear() {
    if let Some(ppid) = get_ppid() {
        session_clear_for(ppid);
    }
}

#[cfg(unix)]
fn session_clear_for(ppid: u32) {
    if let Some(path) = session_file_path(ppid) {
        // Overwrite with zeros before unlinking so the content isn't recoverable
        if let Ok(meta) = std::fs::metadata(&path) {
            let zeros = vec![0u8; meta.len() as usize];
            let _ = std::fs::write(&path, &zeros);
        }
        let _ = std::fs::remove_file(path);
    }
}

// Non-Unix stubs (Windows / wasm – session cache is a no-op there).
#[cfg(not(unix))]
pub(crate) fn session_save(_wallet_id: &str, _password: &str) -> Result<(), String> {
    Err("session cache is only supported on Unix".to_string())
}
#[cfg(not(unix))]
pub(crate) fn session_load() -> Option<(String, String)> {
    None
}
#[cfg(not(unix))]
pub(crate) fn session_load_password() -> Option<String> {
    None
}
#[cfg(not(unix))]
pub(crate) fn session_load_wallet_id() -> Option<String> {
    None
}
#[cfg(not(unix))]
pub(crate) fn session_clear() {}

// ---------------------------------------------------------------------------
// Password / wallet-ID resolution helpers
// ---------------------------------------------------------------------------

/// Resolve password: CLI flag -> session cache -> `NYKS_WALLET_PASSPHRASE` env var -> None.
pub(crate) fn resolve_password(password: Option<String>) -> Option<String> {
    password
        .or_else(session_load_password)
        .or_else(|| std::env::var("NYKS_WALLET_PASSPHRASE").ok())
}

/// Resolve wallet_id: CLI flag -> session cache -> `NYKS_WALLET_ID` env var -> None.
pub(crate) fn resolve_wallet_id(wallet_id: Option<String>) -> Option<String> {
    wallet_id
        .or_else(session_load_wallet_id)
        .or_else(|| std::env::var("NYKS_WALLET_ID").ok())
}

/// Resolve an `OrderWallet` -- load from DB using wallet_id (arg or env).
///
/// Priority: CLI arg -> `NYKS_WALLET_ID` env var -> error.
/// Password priority: CLI arg -> `NYKS_WALLET_PASSPHRASE` env var -> session cache.
#[cfg(any(feature = "sqlite", feature = "postgresql"))]
pub(crate) async fn resolve_order_wallet(
    wallet_id: Option<String>,
    password: Option<String>,
) -> Result<OrderWallet, String> {
    let wid = resolve_wallet_id(wallet_id)
        .ok_or("wallet_id is required (pass --wallet-id, set NYKS_WALLET_ID env var, or run `wallet unlock`)")?;
    let pwd = resolve_password(password);
    load_order_wallet_from_db(&wid, pwd, None)
}

// ---------------------------------------------------------------------------
// QR code display
// ---------------------------------------------------------------------------

/// Visible width of a string after stripping ANSI escape sequences.
fn visible_width(s: &str) -> usize {
    let mut width = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if c == '\x1b' {
            in_escape = true;
        } else {
            width += 1;
        }
    }
    width
}

/// Get the terminal width using libc ioctl (Unix), falling back to `$COLUMNS`.
#[cfg(unix)]
fn get_terminal_width() -> Option<u16> {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 {
            return Some(ws.ws_col);
        }
    }
    std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse().ok())
}

/// Get the terminal width from `$COLUMNS` (non-Unix fallback).
#[cfg(not(unix))]
fn get_terminal_width() -> Option<u16> {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse().ok())
}

/// Print a QR code for `data` alongside `info_lines`.
///
/// If the terminal is wide enough the QR code is rendered to the right of the
/// text; otherwise it is printed below.  Gracefully degrades when the QR
/// library fails or the terminal width cannot be determined.
pub(crate) fn print_with_qr(info_lines: &[String], data: &str) {
    match qr2term::generate_qr_string(data) {
        Ok(qr) => {
            let qr_lines: Vec<&str> = qr.lines().collect();
            let qr_width = qr_lines
                .iter()
                .map(|l| visible_width(l))
                .max()
                .unwrap_or(0);
            let text_width = info_lines.iter().map(|l| l.len()).max().unwrap_or(0);
            let gap = 4;
            let term_width = get_terminal_width().unwrap_or(80) as usize;

            if text_width + gap + qr_width <= term_width {
                // Side-by-side: text on the left, QR on the right
                let total_rows = std::cmp::max(info_lines.len(), qr_lines.len());
                for i in 0..total_rows {
                    let text_part = if i < info_lines.len() {
                        &info_lines[i]
                    } else {
                        ""
                    };
                    let qr_part = if i < qr_lines.len() {
                        qr_lines[i]
                    } else {
                        ""
                    };
                    println!(
                        "{:<width$}{}{}",
                        text_part,
                        " ".repeat(gap),
                        qr_part,
                        width = text_width
                    );
                }
            } else {
                // Stacked: text first, then QR below
                for line in info_lines {
                    println!("{line}");
                }
                println!();
                print!("{qr}");
            }
        }
        Err(e) => {
            for line in info_lines {
                println!("{line}");
            }
            eprintln!("\n  (Could not render QR code: {e})");
        }
    }
}
