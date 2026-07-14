use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Entries that envlock manages in .gitignore.
const ENTRIES: &[&str] = &[".env", ".envlock/identity.txt"];

/// Ensure the given entries are present in `.gitignore`, appending only
/// what's missing. Creates the file if it doesn't exist.
///
/// Deduplication: if an entry already exists on any line (even if it was
/// added by another tool or manually), we don't add it again.
pub fn ensure_gitignore_entries(project_root: &Path) -> Result<()> {
    let gitignore_path = project_root.join(".gitignore");

    let existing = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)
            .with_context(|| format!("Failed to read {}", gitignore_path.display()))?
    } else {
        String::new()
    };

    let mut to_append: Vec<&str> = Vec::new();

    for entry in ENTRIES {
        if !line_exists(&existing, entry) {
            to_append.push(entry);
        }
    }

    if to_append.is_empty() {
        return Ok(());
    }

    let mut content = existing;
    // Ensure we start on a new line if there's existing content
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }

    // Add a section header comment
    content.push_str("# envlock managed\n");
    for entry in &to_append {
        content.push_str(entry);
        content.push('\n');
    }

    fs::write(&gitignore_path, content)
        .with_context(|| format!("Failed to write {}", gitignore_path.display()))?;

    Ok(())
}

/// Check whether a specific entry already exists as its own line in
/// the .gitignore content. Trims whitespace so leading/trailing
/// spaces don't cause false negatives.
fn line_exists(content: &str, entry: &str) -> bool {
    content.lines().any(|l| l.trim() == entry)
}

/// Return which managed entries are currently missing from .gitignore.
pub fn missing_entries(project_root: &Path) -> Result<Vec<&'static str>> {
    let gitignore_path = project_root.join(".gitignore");

    let content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)
            .with_context(|| format!("Failed to read {}", gitignore_path.display()))?
    } else {
        String::new()
    };

    let missing: Vec<&str> = ENTRIES
        .iter()
        .filter(|entry| !line_exists(&content, entry))
        .copied()
        .collect();

    Ok(missing)
}

/// Check whether a file would be tracked by git (i.e. git would include it
/// in a commit). Returns true if the file exists AND is not gitignored.
/// Returns false if the file doesn't exist or is properly ignored.
pub fn is_git_tracked(project_root: &Path, file_path: &str) -> bool {
    // Quick check: does the file even exist?
    let full_path = project_root.join(file_path);
    if !full_path.exists() {
        return false;
    }

    // Use git check-ignore to see if git would ignore this file.
    // If git check-ignore exits 0, the file IS ignored (safe).
    // If it exits 1, the file is NOT ignored (danger — would be tracked).
    // If it exits anything else, git may not be initialized or the file
    // doesn't exist in the index — treat as not-tracked for safety.
    let result = std::process::Command::new("git")
        .args(["check-ignore", "--quiet", file_path])
        .current_dir(project_root)
        .status();

    match result {
        Ok(status) => !status.success(), // success = ignored = safe = not tracked
        Err(_) => false,                 // git not available, assume not tracked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_exists_finds_entry() {
        let content = ".env\n*.log\n.envlock/identity.txt\n";
        assert!(line_exists(content, ".env"));
        assert!(line_exists(content, ".envlock/identity.txt"));
        assert!(!line_exists(content, "other"));
    }

    #[test]
    fn line_exists_trims_whitespace() {
        let content = "  .env  \n*.log\n";
        assert!(line_exists(content, ".env"));
    }

    #[test]
    fn line_exists_handles_empty() {
        assert!(!line_exists("", ".env"));
    }

    #[test]
    fn missing_entries_empty_gitignore() {
        let dir = std::env::temp_dir().join("envlock_test_missing_empty");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let missing = missing_entries(&dir).unwrap();
        assert_eq!(missing, vec![".env", ".envlock/identity.txt"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_entries_partial_gitignore() {
        let dir = std::env::temp_dir().join("envlock_test_missing_partial");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(".gitignore"), ".env\n").unwrap();
        let missing = missing_entries(&dir).unwrap();
        assert_eq!(missing, vec![".envlock/identity.txt"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_gitignore_no_duplicates() {
        let dir = std::env::temp_dir().join("envlock_test_dedup");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // Pre-existing .gitignore with .env already in it
        fs::write(dir.join(".gitignore"), ".env\n*.log\n").unwrap();

        ensure_gitignore_entries(&dir).unwrap();

        let content = fs::read_to_string(dir.join(".gitignore")).unwrap();
        let env_count = content.lines().filter(|l| l.trim() == ".env").count();
        assert_eq!(env_count, 1, ".env should not be duplicated");

        let id_count = content
            .lines()
            .filter(|l| l.trim() == ".envlock/identity.txt")
            .count();
        assert_eq!(id_count, 1, "identity.txt should be added once");

        let _ = fs::remove_dir_all(&dir);
    }
}
