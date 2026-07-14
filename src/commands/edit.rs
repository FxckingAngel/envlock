use crate::commands::check;
use crate::config;
use crate::crypto;
use crate::gitignore;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Temp file used during editing. Never committed — gitignored by envlock.
const EDIT_TMP: &str = ".env.edit.tmp";

/// RAII guard that scrubs and deletes the temp file on drop.
/// Fires on normal returns, early returns, and panics.
/// (Does NOT fire on SIGKILL — that's an OS limitation.)
struct TempGuard {
    path: PathBuf,
}

impl TempGuard {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        // Overwrite with zeros before deleting to scrub secrets from disk
        if let Ok(metadata) = fs::metadata(&self.path) {
            let zeros = vec![0u8; metadata.len() as usize];
            let _ = fs::write(&self.path, &zeros);
        }
        let _ = fs::remove_file(&self.path);
    }
}

/// `envlock edit`
/// Decrypts the vault to a temp file, opens $EDITOR, then re-encrypts
/// on editor close. The temp file is scrubbed (zeroed + deleted) by a
/// Drop guard even on panic.
///
/// The plaintext `.env` is NEVER written to disk — only the temp file,
/// which is gitignored and scrubbed on exit.
pub fn execute(vault_path: Option<PathBuf>) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let vault = vault_path.unwrap_or_else(|| PathBuf::from(".env.vault"));

    // Pre-flight: warn if gitignore isn't covering secrets in this checkout
    let check_result = check::run_check(&project_root, false)?;
    if !check_result.problems.is_empty() {
        eprintln!("⚠ WARNING: .gitignore may not be protecting secrets in this checkout:");
        for problem in &check_result.problems {
            eprintln!("  {}", problem);
        }
        eprintln!("  Run `envlock init` or `envlock doctor` to fix this.");
        eprintln!();
    }

    // Read identity
    let identity_str = config::read_identity(&project_root)?;
    let identity = crypto::parse_identity(&identity_str)?;

    // Read and decrypt vault
    let ciphertext = std::fs::read(&vault)
        .with_context(|| format!("Failed to read vault file {}", vault.display()))?;
    let plaintext = crypto::decrypt_bytes(&ciphertext, &identity)?;

    if plaintext.is_empty() {
        bail!("Vault {} is empty — nothing to edit", vault.display());
    }

    // Write plaintext to temp file
    let tmp_path = project_root.join(EDIT_TMP);
    fs::write(&tmp_path, &plaintext)
        .with_context(|| format!("Failed to write temp file {}", tmp_path.display()))?;

    // Set 0600 on temp file
    config::set_private_permissions(&tmp_path)?;

    // Guard will scrub the temp file no matter how we exit
    let _guard = TempGuard::new(tmp_path.clone());

    // Ensure the temp file is gitignored
    gitignore::ensure_gitignore_entries(&project_root)?;

    // Open editor
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());

    let status = Command::new(&editor)
        .arg(&tmp_path)
        .status()
        .with_context(|| format!("Failed to launch editor '{}'", editor))?;

    if !status.success() {
        bail!(
            "Editor '{}' exited with non-zero status — vault left unchanged, temp file scrubbed",
            editor
        );
    }

    // Read the edited content back
    let edited = fs::read(&tmp_path)
        .with_context(|| format!("Failed to read edited temp file {}", tmp_path.display()))?;

    if edited.is_empty() {
        bail!("Edited file is empty — vault left unchanged. If this was a mistake, the vault is intact.");
    }

    // Validate that the edited content parses as valid .env
    let pairs: Vec<(String, String)> = dotenvy::from_read_iter(edited.as_slice())
        .map(|item| {
            let (key, val) = item.context("Failed to parse edited .env content")?;
            Ok((key, val))
        })
        .collect::<Result<Vec<_>>>()?;

    if pairs.is_empty() {
        bail!(
            "Edited file contains no key-value pairs — vault left unchanged"
        );
    }

    // Re-encrypt with the same recipients
    let recipient_strs = config::read_recipients(&project_root)?;
    let recipients: Vec<age::x25519::Recipient> = recipient_strs
        .iter()
        .map(|s| crypto::parse_recipient(s))
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to parse recipients")?;

    let new_ciphertext = crypto::encrypt_bytes(&edited, &recipients)?;

    // Write updated vault
    fs::write(&vault, &new_ciphertext)
        .with_context(|| format!("Failed to write updated vault {}", vault.display()))?;

    // Guard drops here and scrubs the temp file
    println!(
        "✓ Vault updated: {} ({} keys)",
        vault.display(),
        pairs.len()
    );

    Ok(())
}
