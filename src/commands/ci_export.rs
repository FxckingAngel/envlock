use crate::config;
use crate::crypto;
use anyhow::{bail, Context, Result};
use std::path::PathBuf;

/// `envlock ci export`
/// Decrypt the vault and output KEY=VALUE lines for piping to CI secret managers.
pub fn execute(format: &str, vault_path: Option<PathBuf>) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let vault = vault_path.unwrap_or_else(|| PathBuf::from(".env.vault"));

    // Read identity
    let identity_str = config::read_identity(&project_root)?;
    let identity = crypto::parse_identity(&identity_str)?;

    // Read and decrypt vault
    let ciphertext = std::fs::read(&vault)
        .with_context(|| format!("Failed to read vault file {}", vault.display()))?;
    let plaintext = crypto::decrypt_bytes(&ciphertext, &identity)?;

    if plaintext.is_empty() {
        bail!("Vault {} is empty — nothing to export", vault.display());
    }

    // Parse into key-value pairs
    let pairs: Vec<(String, String)> = dotenvy::from_read_iter(plaintext.as_slice())
        .map(|item| {
            let (key, val) = item.context("Failed to parse .env line")?;
            Ok((key, val))
        })
        .collect::<Result<Vec<_>>>()?;

    if pairs.is_empty() {
        bail!("Vault contains no key-value pairs — nothing to export");
    }

    match format {
        "dotenv" => {
            for (key, val) in &pairs {
                println!("{}={}", key, val);
            }
        }
        "json" => {
            println!("{{");
            let last = pairs.len() - 1;
            for (i, (key, val)) in pairs.iter().enumerate() {
                let comma = if i == last { "" } else { "," };
                println!("  \"{}\": \"{}\"{}", key, escape_json(val), comma);
            }
            println!("}}");
        }
        "github" => {
            // GitHub Actions format for $GITHUB_ENV
            for (key, val) in &pairs {
                if val.contains('\n') {
                    println!("{}<<EOF", key);
                    println!("{}", val);
                    println!("EOF");
                } else {
                    println!("{}={}", key, val);
                }
            }
        }
        _ => bail!("Unknown export format '{}'. Supported: dotenv, json, github", format),
    }

    eprintln!(
        "✓ Exported {} secret(s) in {} format",
        pairs.len(),
        format
    );

    Ok(())
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
