use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Per-project envlock directory name.
const ENVLOCK_DIR: &str = ".envlock";

/// Return the path to the `.envlock/` directory inside `project_root`.
pub fn envlock_dir(project_root: &Path) -> PathBuf {
    project_root.join(ENVLOCK_DIR)
}

/// Ensure the `.envlock/` directory exists; create it if missing.
pub fn ensure_envlock_dir(project_root: &Path) -> Result<PathBuf> {
    let dir = envlock_dir(project_root);
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create {}", dir.display()))?;
    }
    Ok(dir)
}

/// Path to the private identity file inside `.envlock/`.
pub fn identity_path(project_root: &Path) -> PathBuf {
    envlock_dir(project_root).join("identity.txt")
}

/// Path to the recipients file inside `.envlock/`.
pub fn recipients_path(project_root: &Path) -> PathBuf {
    envlock_dir(project_root).join("recipients.txt")
}

/// Read the private identity file. Returns a specific error if missing.
pub fn read_identity(project_root: &Path) -> Result<String> {
    let path = identity_path(project_root);
    if !path.exists() {
        bail!("No identity found. Run 'envlock init' first.");
    }
    fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))
}

/// Read all recipients (one per line) from `.envlock/recipients.txt`.
pub fn read_recipients(project_root: &Path) -> Result<Vec<String>> {
    let path = recipients_path(project_root);
    if !path.exists() {
        bail!(
            "No recipients file found at {}. Run 'envlock init' first.",
            path.display()
        );
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let recipients: Vec<String> = content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if recipients.is_empty() {
        bail!("Recipients file is empty. Something went wrong during init.");
    }
    Ok(recipients)
}

/// Write the private key to `.envlock/identity.txt` with mode 0600 on Unix.
pub fn write_identity(project_root: &Path, key_text: &str) -> Result<()> {
    let dir = ensure_envlock_dir(project_root)?;
    let path = dir.join("identity.txt");
    fs::write(&path, key_text).with_context(|| "Failed to write identity file")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, perms)
            .with_context(|| "Failed to set identity file permissions to 0600")?;
    }

    Ok(())
}

/// Overwrite `.envlock/recipients.txt` with the given list of recipient strings.
pub fn write_recipients_list(project_root: &Path, recipients: &[String]) -> Result<()> {
    let dir = ensure_envlock_dir(project_root)?;
    let path = dir.join("recipients.txt");
    let content = recipients.join("\n") + "\n";
    fs::write(&path, content).with_context(|| "Failed to write recipients file")?;
    Ok(())
}

/// Write a single recipient to `.envlock/recipients.txt` (used by init).
pub fn write_recipients(project_root: &Path, recipient: &str) -> Result<()> {
    write_recipients_list(project_root, &[recipient.to_string()])
}

/// Derive the default vault path from a plaintext path.
/// E.g. `.env` → `.env.vault`
pub fn vault_path(plaintext_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.vault", plaintext_path.display()))
}

/// Derive the plaintext path from a vault path.
/// E.g. `.env.vault` → `.env`
pub fn plaintext_path(vault_path: &Path) -> Result<PathBuf> {
    let s = vault_path
        .to_str()
        .context("Vault path is not valid UTF-8")?;
    if !s.ends_with(".vault") {
        bail!("Vault file must end with .vault, got: {}", s);
    }
    Ok(PathBuf::from(&s[..s.len() - ".vault".len()]))
}

/// Set file mode 0600 on Unix, no-op elsewhere.
pub fn set_private_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(path, perms)
            .with_context(|| format!("Failed to set permissions on {}", path.display()))?;
    }
    Ok(())
}
