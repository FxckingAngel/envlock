use anyhow::{bail, Context, Result};
use std::fs;

/// The pre-commit hook content.
const HOOK_CONTENT: &str = r#"#!/bin/sh
# envlock pre-commit hook — prevents committing secrets
envlock check
"#;

/// `envlock hook install`
/// Installs a pre-commit hook that runs `envlock check`.
/// If a pre-commit hook already exists, appends envlock check
/// rather than overwriting.
pub fn install() -> Result<()> {
    let project_root = std::env::current_dir()?;
    let hooks_dir = project_root.join(".git").join("hooks");

    if !hooks_dir.exists() {
        bail!(
            "No .git/hooks directory found. Run this from the root of a git repository."
        );
    }

    let hook_path = hooks_dir.join("pre-commit");

    if hook_path.exists() {
        let existing = fs::read_to_string(&hook_path)
            .with_context(|| format!("Failed to read {}", hook_path.display()))?;

        // Already has envlock check?
        if existing.contains("envlock check") {
            println!("✓ Pre-commit hook already contains 'envlock check' — nothing to do.");
            return Ok(());
        }

        // Append to existing hook
        let mut updated = existing;
        if !updated.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str("\n# envlock pre-commit hook\nenvlock check\n");

        fs::write(&hook_path, updated)
            .with_context(|| format!("Failed to update {}", hook_path.display()))?;

        println!("✓ Appended 'envlock check' to existing pre-commit hook.");
    } else {
        // Create new hook
        fs::write(&hook_path, HOOK_CONTENT)
            .with_context(|| format!("Failed to write {}", hook_path.display()))?;

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&hook_path, perms)
                .with_context(|| "Failed to set hook file permissions")?;
        }

        println!("✓ Created pre-commit hook at .git/hooks/pre-commit");
    }

    println!("  The hook will run `envlock check` before every commit.");
    println!("  If secrets would be trackable, the commit is blocked.");

    Ok(())
}

/// `envlock hook uninstall`
/// Removes the envlock section from the pre-commit hook.
/// If the hook only contained envlock content, deletes it entirely.
pub fn uninstall() -> Result<()> {
    let project_root = std::env::current_dir()?;
    let hook_path = project_root.join(".git").join("hooks").join("pre-commit");

    if !hook_path.exists() {
        println!("No pre-commit hook found — nothing to uninstall.");
        return Ok(());
    }

    let existing = fs::read_to_string(&hook_path)
        .with_context(|| format!("Failed to read {}", hook_path.display()))?;

    if !existing.contains("envlock check") {
        println!("Pre-commit hook doesn't contain 'envlock check' — nothing to uninstall.");
        return Ok(());
    }

    // If the hook is entirely our content, just delete it
    if existing.trim() == HOOK_CONTENT.trim() {
        fs::remove_file(&hook_path)
            .with_context(|| format!("Failed to delete {}", hook_path.display()))?;
        println!("✓ Removed pre-commit hook (was entirely envlock-managed).");
        return Ok(());
    }

    // Otherwise, remove just our lines
    let updated: String = existing
        .lines()
        .filter(|line| {
            !line.contains("envlock check")
                && !line.contains("# envlock pre-commit hook")
        })
        .collect::<Vec<&str>>()
        .join("\n");

    let updated = updated.trim_end().to_string() + "\n";

    fs::write(&hook_path, &updated)
        .with_context(|| format!("Failed to update {}", hook_path.display()))?;

    println!("✓ Removed 'envlock check' from pre-commit hook.");
    Ok(())
}
