use crate::commands::check;
use crate::gitignore;
use anyhow::Result;

/// `envlock doctor`
/// Comprehensive diagnostic — run after `git clone` to verify everything
/// is set up correctly before doing anything else.
pub fn execute() -> Result<()> {
    let project_root = std::env::current_dir()?;

    println!("envlock doctor — checking your setup...");
    println!();

    let mut all_ok = true;

    // 1. Git repo check
    let git_dir = project_root.join(".git");
    if git_dir.exists() {
        println!("✓ Git repository detected");
    } else {
        println!("✗ No git repository found — envlock requires git");
        all_ok = false;
    }

    // 2. .envlock directory
    let envlock_dir = project_root.join(".envlock");
    if envlock_dir.exists() {
        println!("✓ .envlock/ directory exists");
    } else {
        println!("✗ No .envlock/ directory — run `envlock init` first");
        all_ok = false;
    }

    // 3. Identity file
    let identity_file = envlock_dir.join("identity.txt");
    if identity_file.exists() {
        println!("✓ Identity file exists (.envlock/identity.txt)");
        // Check permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(&identity_file) {
                let mode = metadata.permissions().mode() & 0o777;
                if mode == 0o600 {
                    println!("  ✓ Identity file permissions are 0600");
                } else {
                    println!("  ⚠ Identity file permissions are {:o}, should be 0600", mode);
                }
            }
        }
    } else if envlock_dir.exists() {
        println!("✗ No identity file — run `envlock init` to generate one");
        all_ok = false;
    }

    // 4. Recipients file
    let recipients_file = envlock_dir.join("recipients.txt");
    if recipients_file.exists() {
        let content = std::fs::read_to_string(&recipients_file).unwrap_or_default();
        let count = content.lines().filter(|l| !l.trim().is_empty()).count();
        println!("✓ Recipients file exists ({} recipient(s))", count);
    } else if envlock_dir.exists() {
        println!("✗ No recipients file — run `envlock init` to create one");
        all_ok = false;
    }

    // 5. .gitignore check
    let missing = gitignore::missing_entries(&project_root)?;
    if missing.is_empty() {
        println!("✓ .gitignore covers all sensitive files");
    } else {
        println!("✗ .gitignore is missing entries: {}", missing.join(", "));
        all_ok = false;
    }

    // 6. Vault file
    let vault_file = project_root.join(".env.vault");
    if vault_file.exists() {
        println!("✓ .env.vault exists (safe to commit)");
    } else {
        println!("  ℹ No .env.vault yet — run `envlock encrypt` after adding secrets to .env");
    }

    // 7. Pre-commit hook
    let hook_path = project_root.join(".git").join("hooks").join("pre-commit");
    let hook_installed = if hook_path.exists() {
        let content = std::fs::read_to_string(&hook_path).unwrap_or_default();
        content.contains("envlock check")
    } else {
        false
    };
    if hook_installed {
        println!("✓ Pre-commit hook is installed");
    } else {
        println!("✗ Pre-commit hook is NOT installed — secrets could be committed accidentally");
        println!("  → Run `envlock hook install` (or `envlock init` which installs it by default)");
        all_ok = false;
    }

    // 8. Full security check (using the same logic as `envlock check`)
    let check_result = check::run_check(&project_root, false)?;
    if !check_result.problems.is_empty() {
        println!();
        println!("⚠ Active security issues:");
        for problem in &check_result.problems {
            println!("  {}", problem);
        }
        all_ok = false;
    }

    // Summary
    println!();
    if all_ok && check_result.problems.is_empty() {
        println!("✓ All checks passed — your envlock setup is healthy.");
        println!("  You're ready to use `envlock edit`, `envlock run`, or `envlock encrypt`.");
    } else {
        println!("⚠ Issues found — fix them before working with secrets.");
        println!("  Start with: `envlock init`");
        std::process::exit(1);
    }

    Ok(())
}
