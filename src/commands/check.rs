use crate::gitignore;
use anyhow::Result;

/// `envlock check`
/// Scans .gitignore and git status to warn if secrets would be trackable.
pub fn execute() -> Result<()> {
    let project_root = std::env::current_dir()?;
    let mut problems = Vec::new();

    // 1. Check if .gitignore entries are missing
    let missing = gitignore::missing_entries(&project_root)?;
    if !missing.is_empty() {
        for entry in &missing {
            problems.push(format!(
                "✗ '{}' is missing from .gitignore — could be accidentally committed",
                entry
            ));
        }
    }

    // 2. Check if the sensitive files would be tracked by git
    let sensitive_files = [".env", ".envlock/identity.txt"];
    for file in &sensitive_files {
        if gitignore::is_git_tracked(&project_root, file) {
            problems.push(format!(
                "✗ '{}' exists and would be tracked by git — add it to .gitignore immediately",
                file
            ));
        }
    }

    // 3. Check if .env.vault exists (good sign)
    let vault_exists = project_root.join(".env.vault").exists();
    let envlock_dir_exists = project_root.join(".envlock").exists();

    // Report results
    if problems.is_empty() {
        println!("✓ All checks passed — secrets are protected.");
        if vault_exists {
            println!("  ✓ .env.vault exists (safe to commit)");
        }
        if envlock_dir_exists {
            println!("  ✓ .envlock/ directory exists");
        }
        let present = gitignore::missing_entries(&project_root)?;
        if present.is_empty() {
            println!("  ✓ .gitignore is up to date");
        }
    } else {
        println!("⚠ Security issues found:");
        for problem in &problems {
            println!("  {}", problem);
        }
        println!();
        println!("Run `envlock init` to fix missing .gitignore entries.");
        std::process::exit(1);
    }

    Ok(())
}
