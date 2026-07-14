use crate::config;
use crate::crypto;
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// `envlock rotate KEY_NAME [--value <new-value>|--prompt]`
/// Decrypts the vault, replaces one key's value, re-encrypts with the same
/// recipients, and writes the updated vault back.
pub fn execute(key_name: &str, value: Option<String>, prompt: bool, vault_path: Option<PathBuf>) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let vault = vault_path.unwrap_or_else(|| PathBuf::from(".env.vault"));

    // Determine the new value
    let new_value = if prompt {
        rpassword::prompt_password(format!("Enter new value for {}: ", key_name))
            .context("Failed to read password from terminal")?
    } else {
        value.with_context(|| format!(
            "No new value provided for '{}'. Use --value <val> or --prompt.",
            key_name
        ))?
    };

    // Read identity
    let identity_str = config::read_identity(&project_root)?;
    let identity = crypto::parse_identity(&identity_str)?;

    // Read ciphertext
    let ciphertext = std::fs::read(&vault)
        .with_context(|| format!("Failed to read vault file {}", vault.display()))?;

    // Decrypt
    let plaintext = crypto::decrypt_bytes(&ciphertext, &identity)?;

    if plaintext.is_empty() {
        bail!("Vault {} is empty — nothing to rotate", vault.display());
    }

    // Parse into ordered map
    let mut pairs: BTreeMap<String, String> = dotenvy::from_read_iter(plaintext.as_slice())
        .map(|item| {
            let (key, val) = item.context("Failed to parse .env line")?;
            Ok((key, val))
        })
        .collect::<Result<_, anyhow::Error>>()?;

    // Check key exists
    if !pairs.contains_key(key_name) {
        bail!(
            "Key '{}' not found in vault {}. Existing keys: {}",
            key_name,
            vault.display(),
            pairs.keys().cloned().collect::<Vec<_>>().join(", ")
        );
    }

    // Rotate the value
    pairs.insert(key_name.to_string(), new_value);

    // Rebuild the .env plaintext
    let new_plaintext: Vec<u8> = pairs
        .iter()
        .map(|(k, v)| format!("{}={}\n", k, v))
        .collect::<String>()
        .into_bytes();

    // Read recipients and re-encrypt
    let recipient_strs = config::read_recipients(&project_root)?;
    let recipients: Vec<age::x25519::Recipient> = recipient_strs
        .iter()
        .map(|s| crypto::parse_recipient(s))
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to parse one or more recipients from .envlock/recipients.txt")?;

    let new_ciphertext = crypto::encrypt_bytes(&new_plaintext, &recipients)?;

    // Write updated vault
    std::fs::write(&vault, &new_ciphertext)
        .with_context(|| format!("Failed to write updated vault {}", vault.display()))?;

    println!("✓ Rotated '{}' in {}", key_name, vault.display());

    Ok(())
}
