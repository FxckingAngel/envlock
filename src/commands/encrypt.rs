use crate::config;
use crate::crypto;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Run `envlock encrypt [path]`.
/// Encrypts the plaintext file to the recipients and writes <path>.vault.
pub fn execute(path: Option<PathBuf>) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let plaintext_path = path.unwrap_or_else(|| PathBuf::from(".env"));

    // Read plaintext
    let plaintext = fs::read(&plaintext_path).with_context(|| {
        format!(
            "Failed to read plaintext file {}",
            plaintext_path.display()
        )
    })?;

    if plaintext.is_empty() {
        anyhow::bail!("Plaintext file {} is empty — nothing to encrypt", plaintext_path.display());
    }

    // Read recipients
    let recipient_strs = config::read_recipients(&project_root)?;

    let recipients: Vec<age::x25519::Recipient> = recipient_strs
        .iter()
        .map(|s| crypto::parse_recipient(s))
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to parse one or more recipients from .envlock/recipients.txt")?;

    // Encrypt
    let ciphertext = crypto::encrypt_bytes(&plaintext, &recipients)?;

    // Write vault file
    let vault_path = config::vault_path(&plaintext_path);
    fs::write(&vault_path, &ciphertext).with_context(|| {
        format!("Failed to write vault file {}", vault_path.display())
    })?;

    println!(
        "✓ Encrypted {} → {} ({} bytes)",
        plaintext_path.display(),
        vault_path.display(),
        ciphertext.len()
    );

    Ok(())
}
