use crate::config;
use crate::crypto;
use anyhow::{bail, Context, Result};
use std::path::PathBuf;

/// `envlock ci seal --prefix <PREFIX> [--output <path>]`
/// Read secrets from environment variables with the given prefix, strip the
/// prefix, encrypt to a vault using the project's public recipients.
///
/// This creates an ephemeral .env.vault in CI — encrypted to the project's
/// public key (committed in recipients.txt) — that any developer can decrypt
/// locally with their private key. The private key NEVER touches CI.
///
/// Example (GitHub Actions):
///   export ENVLOCK_DB_URL=${{ secrets.DB_URL }}
///   export ENVLOCK_API_KEY=${{ secrets.API_KEY }}
///   envlock ci seal --prefix ENVLOCK_
///   # Now .env.vault exists, encrypted to the project's recipients
pub fn execute(prefix: String, output: Option<PathBuf>, overwrite: bool) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let vault_path = output.unwrap_or_else(|| PathBuf::from(".env.vault"));

    let mut pairs: Vec<(String, String)> = Vec::new();

    // Read prefixed env vars from the current process environment
    for (key, val) in std::env::vars() {
        if key.starts_with(&prefix) {
            let stripped_key = key[prefix.len()..].to_string();
            if !stripped_key.is_empty() {
                pairs.push((stripped_key, val));
            }
        }
    }

    if pairs.is_empty() {
        bail!(
            "No environment variables found with prefix '{}'. \
             Set vars like {}DB_URL=xxx before running this command.",
            prefix,
            prefix
        );
    }

    // Don't overwrite existing vault unless --overwrite
    if vault_path.exists() && !overwrite {
        bail!(
            "{} already exists. Use --overwrite to replace it, or choose a different --output path.",
            vault_path.display()
        );
    }

    // Build plaintext .env content
    let plaintext: Vec<u8> = pairs
        .iter()
        .map(|(k, v)| format!("{}={}\n", k, v))
        .collect::<String>()
        .into_bytes();

    // Read recipients (public keys — committed to the repo, safe for CI)
    let recipient_strs = config::read_recipients(&project_root)?;
    let recipients: Vec<age::x25519::Recipient> = recipient_strs
        .iter()
        .map(|s| crypto::parse_recipient(s))
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to parse recipients from .envlock/recipients.txt")?;

    // Encrypt using only public keys — no private key needed
    let ciphertext = crypto::encrypt_bytes(&plaintext, &recipients)?;

    // Write vault
    std::fs::write(&vault_path, &ciphertext)
        .with_context(|| format!("Failed to write vault file {}", vault_path.display()))?;

    println!(
        "✓ Sealed {} secret(s) → {} ({} bytes, encrypted to {} recipient(s))",
        pairs.len(),
        vault_path.display(),
        ciphertext.len(),
        recipients.len()
    );
    println!("  No private key was used — vault can be decrypted by any project member.");

    Ok(())
}
