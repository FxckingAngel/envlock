use anyhow::{bail, Context, Result};
use std::process::Command;

/// `envlock ci run --prefix <PREFIX> -- <command...>`
/// Read secrets from environment variables with the given prefix, strip the
/// prefix, and inject them into the subprocess — same interface as `envlock run`
/// but sourced from the CI provider's environment instead of a vault.
///
/// In CI, the provider sets env vars like ENVLOCK_DB_URL=xxx. This command
/// reads all ENVLOCK_* vars, strips the prefix, and injects DB_URL=xxx into
/// the subprocess. No private key or vault file needed.
///
/// Example:
///   export ENVLOCK_DB_URL=postgres://localhost
///   export ENVLOCK_API_KEY=sk-123
///   envlock ci run --prefix ENVLOCK_ -- python app.py
pub fn execute(prefix: String, command: Vec<String>, vault: Option<std::path::PathBuf>) -> Result<()> {
    if command.is_empty() {
        bail!("No command provided. Usage: envlock ci run --prefix <PREFIX> -- <command...>");
    }

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

    // Optionally: also load from vault if it exists and identity is available
    // This gives CI parity with local — vault vars + CI-specific overrides
    if let Some(vault_path) = &vault {
        let project_root = std::env::current_dir()?;
        if vault_path.exists() {
            if let Ok(identity_str) = crate::config::read_identity(&project_root) {
                if let Ok(identity) = crate::crypto::parse_identity(&identity_str) {
                    if let Ok(ciphertext) = std::fs::read(vault_path) {
                        if let Ok(plaintext) = crate::crypto::decrypt_bytes(&ciphertext, &identity) {
                            let vault_pairs: Vec<(String, String)> =
                                dotenvy::from_read_iter(plaintext.as_slice())
                                    .filter_map(|item| item.ok())
                                    .collect();
                            // Vault vars first, then CI env vars override them
                            for (key, val) in vault_pairs {
                                if !pairs.iter().any(|(k, _)| k == &key) {
                                    pairs.push((key, val));
                                }
                            }
                        }
                    }
                }
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

    // Build the subprocess
    let mut cmd = Command::new(&command[0]);
    if command.len() > 1 {
        cmd.args(&command[1..]);
    }

    // Inject env vars — CI vars override vault vars (same as `envlock run`)
    for (key, val) in &pairs {
        cmd.env(key, val);
    }

    // Stream stdout/stderr through, forward exit code
    let status = cmd.status().context("Failed to spawn subprocess")?;
    let code = status.code().unwrap_or(1);
    std::process::exit(code);
}
