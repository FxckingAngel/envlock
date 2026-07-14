use crate::config;
use crate::crypto;
use crate::gitignore;
use anyhow::Result;

/// Run `envlock init`.
/// Generates an age keypair, saves the private key, prints the public key,
/// and appends entries to .gitignore.
pub fn execute() -> Result<()> {
    let project_root = std::env::current_dir()?;

    // Generate identity
    let identity = crypto::generate_identity();
    let identity_str = crypto::identity_to_string(&identity);
    let recipient = crypto::identity_to_recipient(&identity);
    let recipient_str = recipient.to_string();

    // Write private key
    config::write_identity(&project_root, &identity_str)?;

    // Write recipients file (the public key for now; supports multiple later)
    config::write_recipients(&project_root, &recipient_str)?;

    // Update .gitignore
    gitignore::ensure_gitignore_entries(&project_root)?;

    println!("✓ Identity generated and saved to .envlock/identity.txt");
    println!("✓ Recipient saved to .envlock/recipients.txt");
    println!();
    println!("Public key (recipient):");
    println!("  {}", recipient_str);
    println!();
    println!("Share this public key with teammates so they can encrypt .env files for you.");

    Ok(())
}
