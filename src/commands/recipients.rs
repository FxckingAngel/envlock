use crate::config;
use crate::crypto;
use anyhow::{bail, Context, Result};

/// `envlock recipients add <public-key>`
/// Validates the key, appends it to .envlock/recipients.txt.
pub fn add(public_key: &str) -> Result<()> {
    let project_root = std::env::current_dir()?;

    // Validate that it parses as a real age recipient
    let _recipient = crypto::parse_recipient(public_key)
        .with_context(|| format!("'{}' is not a valid age recipient (expected age1...)", public_key))?;

    // Read existing recipients
    let mut recipients = config::read_recipients(&project_root)?;

    // Check for duplicates
    if recipients.iter().any(|r| r.trim() == public_key.trim()) {
        bail!("Recipient {} is already in .envlock/recipients.txt", public_key.trim());
    }

    recipients.push(public_key.trim().to_string());

    config::write_recipients_list(&project_root, &recipients)?;

    println!("✓ Added recipient: {}", public_key.trim());
    println!("  {} recipient(s) total", recipients.len());

    Ok(())
}

/// `envlock recipients list`
/// Prints all recipients from .envlock/recipients.txt.
pub fn list() -> Result<()> {
    let project_root = std::env::current_dir()?;
    let recipients = config::read_recipients(&project_root)?;

    println!("Recipients in .envlock/recipients.txt ({}):", recipients.len());
    for (i, r) in recipients.iter().enumerate() {
        println!("  {}. {}", i + 1, r);
    }

    Ok(())
}

/// `envlock recipients remove <public-key>`
/// Removes the matching recipient from .envlock/recipients.txt.
pub fn remove(public_key: &str) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let mut recipients = config::read_recipients(&project_root)?;

    let key = public_key.trim();
    let original_len = recipients.len();
    recipients.retain(|r| r.trim() != key);

    if recipients.len() == original_len {
        bail!("Recipient {} not found in .envlock/recipients.txt", key);
    }

    if recipients.is_empty() {
        bail!(
            "Cannot remove the last recipient — at least one recipient is required \
             so that .env files remain decryptable. Add a new recipient first."
        );
    }

    config::write_recipients_list(&project_root, &recipients)?;

    println!("✓ Removed recipient: {}", key);
    println!("  {} recipient(s) remaining", recipients.len());

    Ok(())
}
