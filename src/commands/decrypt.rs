use crate::config;
use crate::crypto;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Run `envlock decrypt [path]`.
/// Decrypts the vault file using the local identity and writes the plaintext.
pub fn execute(path: Option<PathBuf>) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let vault_path = path.unwrap_or_else(|| PathBuf::from(".env.vault"));

    // Read ciphertext
    let ciphertext = fs::read(&vault_path)
        .with_context(|| format!("Failed to read vault file {}", vault_path.display()))?;

    // Read identity
    let identity_str = config::read_identity(&project_root)?;
    let identity = crypto::parse_identity(&identity_str)?;

    // Decrypt
    let plaintext = crypto::decrypt_bytes(&ciphertext, &identity)?;

    // Derive output path
    let out_path = config::plaintext_path(&vault_path)?;

    // Write decrypted file
    fs::write(&out_path, &plaintext)
        .with_context(|| format!("Failed to write decrypted file {}", out_path.display()))?;

    // Set private permissions
    config::set_private_permissions(&out_path)?;

    println!(
        "✓ Decrypted {} → {} ({} bytes)",
        vault_path.display(),
        out_path.display(),
        plaintext.len()
    );

    Ok(())
}
