use crate::gitignore;
use anyhow::Result;

/// Check result returned by the core check logic.
pub struct CheckResult {
    pub problems: Vec<String>,
    pub warnings: Vec<String>,
    pub vault_exists: bool,
    pub envlock_dir_exists: bool,
    pub gitignore_ok: bool,
    pub hook_installed: bool,
}

/// Core check logic — reusable by `check`, `doctor`, `encrypt`, `edit`, and the pre-commit hook.
/// Returns a structured result instead of printing/exiting, so callers can decide
/// what to do with the findings.
pub fn run_check(project_root: &std::path::Path, strict: bool) -> Result<CheckResult> {
    let mut problems = Vec::new();
    let mut warnings = Vec::new();

    // 1. Check if .gitignore entries are missing
    let missing = gitignore::missing_entries(project_root)?;
    if !missing.is_empty() {
        for entry in &missing {
            problems.push(format!(
                "✗ '{}' is missing from .gitignore — could be accidentally committed",
                entry
            ));
        }
    }

    // 2. Check if the sensitive files would be tracked by git
    let sensitive_files = [".env", ".envlock/identity.txt", ".env.edit.tmp"];
    for file in &sensitive_files {
        if gitignore::is_git_tracked(project_root, file) {
            problems.push(format!(
                "✗ '{}' exists and would be tracked by git — add it to .gitignore immediately",
                file
            ));
        }
    }

    // 3. Strict mode: check for any secret-shaped files that are tracked
    if strict {
        let extra_sensitive = scan_for_secret_files(project_root)?;
        for file in &extra_sensitive {
            if gitignore::is_git_tracked(project_root, file) {
                problems.push(format!(
                    "✗ '{}' looks like a secrets file and would be tracked by git (strict mode)",
                    file
                ));
            }
        }
    }

    // 4. Check hook installation
    let hook_path = project_root.join(".git").join("hooks").join("pre-commit");
    let hook_installed = if hook_path.exists() {
        let content = std::fs::read_to_string(&hook_path).unwrap_or_default();
        content.contains("envlock check")
    } else {
        false
    };

    if !hook_installed {
        warnings.push(
            "⊘ Pre-commit hook is not installed. Run `envlock hook install` to block insecure commits.".to_string()
        );
    }

    // 5. Status checks
    let vault_exists = project_root.join(".env.vault").exists();
    let envlock_dir_exists = project_root.join(".envlock").exists();
    let gitignore_ok = gitignore::missing_entries(project_root)?.is_empty();

    Ok(CheckResult {
        problems,
        warnings,
        vault_exists,
        envlock_dir_exists,
        gitignore_ok,
        hook_installed,
    })
}

/// `envlock check`
/// Scans .gitignore and git status to warn if secrets would be trackable.
pub fn execute(strict: bool) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let result = run_check(&project_root, strict)?;

    if result.problems.is_empty() && result.warnings.is_empty() {
        println!("✓ All checks passed — secrets are protected.");
        if result.vault_exists {
            println!("  ✓ .env.vault exists (safe to commit)");
        }
        if result.envlock_dir_exists {
            println!("  ✓ .envlock/ directory exists");
        }
        if result.gitignore_ok {
            println!("  ✓ .gitignore is up to date");
        }
        if result.hook_installed {
            println!("  ✓ pre-commit hook is installed");
        }
    } else {
        if !result.problems.is_empty() {
            println!("⚠ Security issues found:");
            for problem in &result.problems {
                println!("  {}", problem);
            }
        }
        if !result.warnings.is_empty() {
            for warning in &result.warnings {
                println!("  {}", warning);
            }
        }
        println!();
        println!("Run `envlock init` to fix missing .gitignore entries.");
        std::process::exit(1);
    }

    Ok(())
}

/// Scan the project root for files that look like they might contain secrets
/// (beyond the standard .env / identity.txt). Used by --strict mode.
fn scan_for_secret_files(project_root: &std::path::Path) -> Result<Vec<String>> {
    let mut found = Vec::new();
    let patterns = [".env.local", ".env.production", ".env.staging", ".env.development"];

    for pattern in &patterns {
        if project_root.join(pattern).exists() {
            found.push(pattern.to_string());
        }
    }

    Ok(found)
}
