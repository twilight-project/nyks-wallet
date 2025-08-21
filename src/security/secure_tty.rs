use std::io::{Result, Write};
use zeroize::Zeroize;

#[cfg(unix)]
fn open_tty() -> Result<std::fs::File> {
    use std::fs::OpenOptions;
    OpenOptions::new().write(true).open("/dev/tty")
}

#[cfg(windows)]
fn open_tty() -> Result<std::fs::File> {
    use std::fs::OpenOptions;
    // "CONOUT$" is the console output device; bypasses stdout redirection
    OpenOptions::new().write(true).open("CONOUT$")
}

/// Print a secret directly to the terminal device (never stdout/stderr),
/// then zeroize the buffer. If no TTY is attached, returns an error and prints nothing.
pub fn print_secret_to_tty(secret: &mut String) -> Result<()> {
    let mut tty = open_tty()?;
    // write line + flush; DO NOT println!/eprintln!
    tty.write_all(secret.as_bytes())?;
    tty.write_all(b"\n")?;
    tty.flush()?;

    // wipe caller-owned buffer
    secret.zeroize();
    Ok(())
}
