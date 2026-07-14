use crate::config;
use crate::crypto;
use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Run `envlock run -- <command...>`.
/// Decrypts .env.vault in memory, parses key-value pairs, and spawns
/// the given command with those vars injected into its environment.
/// The decrypted content never touches disk.
///
/// **Vault values override existing env vars.** This is intentional:
/// the whole point of envlock is that the vault is the source of truth
/// for secrets. If you need the parent env to win, set the var after
/// `envlock run` in your shell.
pub fn execute(command: Vec<String>, vault_path: Option<PathBuf>) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let vault = vault_path.unwrap_or_else(|| PathBuf::from(".env.vault"));

    if command.is_empty() {
        bail!("No command provided. Usage: envlock run -- <command...>");
    }

    // Read ciphertext
    let ciphertext = std::fs::read(&vault)
        .with_context(|| format!("Failed to read vault file {}", vault.display()))?;

    // Read identity
    let identity_str = config::read_identity(&project_root)?;
    let identity = crypto::parse_identity(&identity_str)?;

    // Decrypt in memory
    let plaintext = crypto::decrypt_bytes(&ciphertext, &identity)?;

    // Reject empty vault content — nothing to inject, likely a mistake
    if plaintext.is_empty() {
        bail!(
            "Vault {} decrypted to empty content. \
             Add keys to your .env and run `envlock encrypt`.",
            vault.display()
        );
    }

    // Parse into key-value pairs with line-number reporting
    let pairs = parse_env_lines(&plaintext)?;

    if pairs.is_empty() {
        bail!(
            "Vault {} contains no key-value pairs. \
             Add keys to your .env and run `envlock encrypt`.",
            vault.display()
        );
    }

    // Build the subprocess
    let mut cmd = Command::new(&command[0]);
    if command.len() > 1 {
        cmd.args(&command[1..]);
    }

    // Inject env vars — vault values OVERRIDE existing parent env vars
    for (key, val) in &pairs {
        cmd.env(key, val);
    }

    // Stream stdout/stderr through, forward exit code
    let status = cmd.status().context("Failed to spawn subprocess")?;

    // Forward the child's exit code
    let code = status.code().unwrap_or(1);
    std::process::exit(code);
}

/// Parse decrypted .env content into key-value pairs, surfacing
/// the offending line number on parse failure instead of swallowing
/// dotenvy's generic error.
fn parse_env_lines(plaintext: &[u8]) -> Result<Vec<(String, String)>> {
    let text = std::str::from_utf8(plaintext).context(
        "Decrypted vault content is not valid UTF-8 — the vault may be corrupted",
    )?;

    let mut pairs = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line_num = i + 1;
        let trimmed = line.trim();

        // Skip blanks and comments — these are fine
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // dotenvy handles the parsing rules; we just give a line-number
        // on failure rather than letting it silently skip or produce a
        // vague error.
        if !trimmed.contains('=') {
            bail!(
                "Parse error on line {}: '{}' — expected KEY=VALUE",
                line_num,
                trimmed
            );
        }

        match dotenvy::from_read_iter(trimmed.as_bytes()).next() {
            Some(Ok((key, val))) => pairs.push((key, val)),
            Some(Err(e)) => bail!(
                "Parse error on line {}: '{}' — {}",
                line_num,
                trimmed,
                e
            ),
            None => {
                // Empty iterator from a line that dotenvy couldn't parse at all
                bail!(
                    "Parse error on line {}: '{}' — not a valid KEY=VALUE assignment",
                    line_num,
                    trimmed
                );
            }
        }
    }

    Ok(pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_env() {
        let input = b"MY_VAR=hello\nDB_URL=postgres://localhost\n";
        let pairs = parse_env_lines(input).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("MY_VAR".into(), "hello".into()));
        assert_eq!(pairs[1], ("DB_URL".into(), "postgres://localhost".into()));
    }

    #[test]
    fn parse_with_comments_and_blanks() {
        let input = b"# comment\n\nMY_VAR=val\n  \nOTHER=2\n";
        let pairs = parse_env_lines(input).unwrap();
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn parse_missing_equals_reports_line() {
        let input = b"GOOD=val\nBAD_LINE_NO_EQUALS\n";
        let result = parse_env_lines(input);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("line 2"), "Error should mention line 2: {}", err);
        assert!(err.contains("BAD_LINE_NO_EQUALS"), "Error should quote the bad line: {}", err);
    }

    #[test]
    fn parse_non_utf8_fails() {
        let input = &[0xFF, 0xFE, 0x00];
        let result = parse_env_lines(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("UTF-8"));
    }
}
