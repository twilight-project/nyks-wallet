use keyring::Entry;
use zeroize::Zeroize;

const SERVICE: &str = "com.nyks.wallet";

pub fn save_mnemonic(wallet_label: &str, mut mnemonic: String) -> anyhow::Result<()> {
    let entry = Entry::new(SERVICE, wallet_label)?;
    // Overwrite any existing value
    entry.set_password(&mnemonic)?;
    mnemonic.zeroize(); // scrub memory
    Ok(())
}

pub fn load_mnemonic(wallet_label: &str) -> anyhow::Result<String> {
    let entry = Entry::new(SERVICE, wallet_label)?;
    Ok(entry.get_password()?)
}

pub fn delete_mnemonic(wallet_label: &str) -> anyhow::Result<()> {
    let entry = Entry::new(SERVICE, wallet_label)?;
    let _ = entry.delete_credential(); // ignore if missing
    Ok(())
}
