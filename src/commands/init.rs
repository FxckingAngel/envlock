use crate::commands::hook;
use crate::config;
use crate::crypto;
use crate::gitignore;
use anyhow::Result;

/// Run `envlock init`.
/// Generates an age keypair, saves the private key, prints the public key,
/// appends entries to .gitignore, and installs the pre-commit hook.
pub fn execute(no_hook: bool) -> Result<()> {
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
    println!("✓ .gitignore updated");
    println!();

    // Install pre-commit hook by default
    if !no_hook {
        match hook::install() {
            Ok(()) => {}
            Err(e) => {
                // Don't fail init if hook install fails (e.g. not a git repo)
                eprintln!("⚠ Could not install pre-commit hook: {}", e);
                eprintln!("  Run `envlock hook install` later to set it up.");
            }
        }
    } else {
        println!("⊘ Pre-commit hook skipped (--no-hook).");
        println!("  Run `envlock hook install` later to set it up.");
    }

    println!();
    println!("Public key (recipient):");
    println!("  {}", recipient_str);
    println!();
    println!("Share this public key with teammates so they can encrypt .env files for you.");

    Ok(())
}
