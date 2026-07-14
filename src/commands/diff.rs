use crate::config;
use crate::crypto;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// `envlock diff <env-a>.vault <env-b>.vault`
/// Decrypts both vaults in memory, compares keys, and prints a diff
/// with redacted values.
pub fn execute(vault_a: PathBuf, vault_b: PathBuf) -> Result<()> {
    let project_root = std::env::current_dir()?;

    // Read identity
    let identity_str = config::read_identity(&project_root)?;
    let identity = crypto::parse_identity(&identity_str)?;

    // Decrypt both vaults in memory
    let pairs_a = decrypt_vault(&vault_a, &identity)?;
    let pairs_b = decrypt_vault(&vault_b, &identity)?;

    let keys_a: BTreeSet<String> = pairs_a.keys().cloned().collect();
    let keys_b: BTreeSet<String> = pairs_b.keys().cloned().collect();

    // Keys only in b → added
    let added: BTreeSet<&String> = keys_b.difference(&keys_a).collect();
    // Keys only in a → removed
    let removed: BTreeSet<&String> = keys_a.difference(&keys_b).collect();
    // Keys in both → check for value changes
    let common: BTreeSet<&String> = keys_a.intersection(&keys_b).collect();
    let changed: BTreeSet<String> = common
        .iter()
        .filter(|k| pairs_a[**k] != pairs_b[**k])
        .map(|k| (**k).clone())
        .collect();

    if added.is_empty() && removed.is_empty() && changed.is_empty() {
        println!("No differences found.");
        return Ok(());
    }

    println!(
        "Comparing {} ↔ {}",
        vault_a.display(),
        vault_b.display()
    );
    println!();

    // Added keys
    for key in &added {
        println!("+ {}", key);
    }

    // Removed keys
    for key in &removed {
        println!("- {}", key);
    }

    // Changed keys (values redacted)
    for key in &changed {
        println!("~ {} (value differs)", key);
    }

    println!();
    println!(
        "Summary: {} added, {} removed, {} changed",
        added.len(),
        removed.len(),
        changed.len()
    );

    Ok(())
}

/// Decrypt a vault file in memory and parse into an ordered map of key→value.
fn decrypt_vault(
    vault_path: &PathBuf,
    identity: &age::x25519::Identity,
) -> Result<BTreeMap<String, String>> {
    let ciphertext = std::fs::read(vault_path)
        .with_context(|| format!("Failed to read vault file {}", vault_path.display()))?;

    let plaintext = crypto::decrypt_bytes(&ciphertext, identity)?;

    if plaintext.is_empty() {
        anyhow::bail!(
            "Vault {} decrypted to empty content",
            vault_path.display()
        );
    }

    let pairs: BTreeMap<String, String> = dotenvy::from_read_iter(plaintext.as_slice())
        .map(|item| {
            let (key, val) = item.context("Failed to parse .env line")?;
            Ok((key, val))
        })
        .collect::<Result<_, anyhow::Error>>()?;

    Ok(pairs)
}
