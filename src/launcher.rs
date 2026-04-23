use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::AppError;
use crate::prompts::ArchitectType;

/// Creates `<base>/.opencode/agents/` if it does not already exist, and
/// returns the path to the directory.
pub fn ensure_agent_dir(base: &Path) -> Result<PathBuf, AppError> {
    let dir = base.join(".opencode").join("agents");
    std::fs::create_dir_all(&dir).map_err(AppError::AgentDirCreation)?;
    Ok(dir)
}

/// Writes the agent `.md` file to `<base>/.opencode/agents/arch-<name>.md`.
/// Returns the path to the written file.
pub fn write_agent_file(
    base: &Path,
    architect: ArchitectType,
    content: &str,
) -> Result<PathBuf, AppError> {
    let dir = ensure_agent_dir(base)?;
    let file_name = format!("{}.md", architect.agent_name());
    let path = dir.join(file_name);
    std::fs::write(&path, content).map_err(AppError::AgentFileWrite)?;
    Ok(path)
}

/// Creates `<base>/reviews/` if it does not already exist, and
/// returns the path to the directory.
pub fn ensure_reviews_dir(base: &Path) -> Result<PathBuf, AppError> {
    let dir = base.join("reviews");
    std::fs::create_dir_all(&dir).map_err(AppError::ReviewsDirCreation)?;
    Ok(dir)
}

/// Replaces the current process with `opencode --agent <agent_name>`.
///
/// On success this function never returns — the Rust process is replaced by
/// opencode, which inherits the terminal and all file descriptors.
/// On failure it returns `AppError::LaunchFailed`.
///
/// # Platform note
/// Uses `std::os::unix::process::CommandExt::exec()` which is Unix-only.
/// The `flake.nix` targets Linux only, so this is acceptable.
#[cfg(not(tarpaulin_include))]
pub fn launch_opencode(agent_name: &str) -> Result<(), AppError> {
    let err = Command::new("opencode")
        .args(["--agent", agent_name])
        .exec();
    // exec() only returns on failure
    Err(AppError::LaunchFailed(err))
}

/// Verifies that `opencode` is available in `PATH`.
#[cfg(not(tarpaulin_include))]
pub fn check_opencode_in_path() -> Result<(), AppError> {
    Command::new("opencode")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|_| AppError::OpenCodeNotFound)?;
    Ok(())
}

/// Removes all `arch-*.md` files from `<base>/.opencode/agents/`.
///
/// After removing matching files, also removes the `agents/` directory if it
/// is empty, and then `.opencode/` if that too becomes empty. Uses
/// `remove_dir` (not `remove_dir_all`) so non-empty directories are never
/// accidentally deleted.
///
/// Returns the list of file paths that were removed. Returns `Ok(vec![])` if
/// `.opencode/agents/` does not exist — this is not an error.
pub fn clean_agent_files(base: &Path) -> Result<Vec<PathBuf>, AppError> {
    let agents_dir = base.join(".opencode").join("agents");

    if !agents_dir.exists() {
        return Ok(vec![]);
    }

    let entries = std::fs::read_dir(&agents_dir).map_err(AppError::CleanReadDir)?;

    let mut removed = Vec::new();
    for entry in entries {
        let entry = entry.map_err(AppError::CleanReadDir)?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name.starts_with("arch-") && name.ends_with(".md") {
            let path = entry.path();
            std::fs::remove_file(&path).map_err(|source| AppError::CleanRemoveFile {
                path: path.display().to_string(),
                source,
            })?;
            removed.push(path);
        }
    }

    // Remove agents/ if now empty.
    if is_dir_empty(&agents_dir) {
        std::fs::remove_dir(&agents_dir).map_err(|source| AppError::CleanRemoveDir {
            path: agents_dir.display().to_string(),
            source,
        })?;

        // Remove .opencode/ if now empty.
        let opencode_dir = base.join(".opencode");
        if is_dir_empty(&opencode_dir) {
            std::fs::remove_dir(&opencode_dir).map_err(|source| AppError::CleanRemoveDir {
                path: opencode_dir.display().to_string(),
                source,
            })?;
        }
    }

    Ok(removed)
}

/// Returns `true` if `dir` exists and contains no entries.
fn is_dir_empty(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|mut d| d.next().is_none())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn ensure_agent_dir_creates_directory() {
        let tmp = TempDir::new().unwrap();
        let dir = ensure_agent_dir(tmp.path()).unwrap();
        assert!(dir.exists());
        assert!(dir.is_dir());
        assert_eq!(dir, tmp.path().join(".opencode").join("agents"));
    }

    #[test]
    fn ensure_agent_dir_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        ensure_agent_dir(tmp.path()).unwrap();
        // Second call must not fail even when the directory already exists.
        ensure_agent_dir(tmp.path()).unwrap();
    }

    #[test]
    fn write_agent_file_creates_file_with_correct_name() {
        let tmp = TempDir::new().unwrap();
        let content = "test content";
        let path = write_agent_file(tmp.path(), ArchitectType::Principal, content).unwrap();
        assert_eq!(
            path,
            tmp.path()
                .join(".opencode")
                .join("agents")
                .join("arch-principal.md")
        );
        assert!(path.exists());
    }

    #[test]
    fn write_agent_file_writes_correct_content() {
        let tmp = TempDir::new().unwrap();
        let content = "# My agent content\nsome text";
        let path = write_agent_file(tmp.path(), ArchitectType::Security, content).unwrap();
        let written = fs::read_to_string(path).unwrap();
        assert_eq!(written, content);
    }

    #[test]
    fn write_agent_file_overwrites_existing_file() {
        let tmp = TempDir::new().unwrap();
        write_agent_file(tmp.path(), ArchitectType::Design, "first").unwrap();
        let path = write_agent_file(tmp.path(), ArchitectType::Design, "second").unwrap();
        let written = fs::read_to_string(path).unwrap();
        assert_eq!(written, "second");
    }

    #[test]
    fn ensure_reviews_dir_creates_directory() {
        let tmp = TempDir::new().unwrap();
        let dir = ensure_reviews_dir(tmp.path()).unwrap();
        assert!(dir.exists());
        assert!(dir.is_dir());
        assert_eq!(dir, tmp.path().join("reviews"));
    }

    #[test]
    fn ensure_reviews_dir_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        ensure_reviews_dir(tmp.path()).unwrap();
        // Second call must not fail even when the directory already exists.
        ensure_reviews_dir(tmp.path()).unwrap();
    }

    // ── clean_agent_files ─────────────────────────────────────────────────────

    fn create_agent_file(base: &std::path::Path, name: &str) -> PathBuf {
        let dir = base.join(".opencode").join("agents");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        fs::write(&path, "content").unwrap();
        path
    }

    #[test]
    fn clean_removes_arch_files() {
        let tmp = TempDir::new().unwrap();
        let p1 = create_agent_file(tmp.path(), "arch-principal.md");
        let p2 = create_agent_file(tmp.path(), "arch-security.md");
        let removed = clean_agent_files(tmp.path()).unwrap();
        assert_eq!(removed.len(), 2);
        assert!(!p1.exists());
        assert!(!p2.exists());
    }

    #[test]
    fn clean_leaves_non_arch_files() {
        let tmp = TempDir::new().unwrap();
        create_agent_file(tmp.path(), "arch-principal.md");
        let custom = create_agent_file(tmp.path(), "my-custom-agent.md");
        let removed = clean_agent_files(tmp.path()).unwrap();
        assert_eq!(removed.len(), 1);
        assert!(custom.exists());
    }

    #[test]
    fn clean_returns_empty_vec_when_dir_missing() {
        let tmp = TempDir::new().unwrap();
        let removed = clean_agent_files(tmp.path()).unwrap();
        assert!(removed.is_empty());
    }

    #[test]
    fn clean_removes_empty_agents_dir() {
        let tmp = TempDir::new().unwrap();
        create_agent_file(tmp.path(), "arch-principal.md");
        clean_agent_files(tmp.path()).unwrap();
        assert!(!tmp.path().join(".opencode").join("agents").exists());
    }

    #[test]
    fn clean_removes_empty_opencode_dir() {
        let tmp = TempDir::new().unwrap();
        create_agent_file(tmp.path(), "arch-principal.md");
        clean_agent_files(tmp.path()).unwrap();
        assert!(!tmp.path().join(".opencode").exists());
    }

    #[test]
    fn clean_preserves_opencode_dir_if_not_empty() {
        let tmp = TempDir::new().unwrap();
        create_agent_file(tmp.path(), "arch-principal.md");
        // Put an extra file directly in .opencode/ so it is not empty after
        // agents/ is removed.
        let extra = tmp.path().join(".opencode").join("config.json");
        fs::write(&extra, "{}").unwrap();
        clean_agent_files(tmp.path()).unwrap();
        assert!(tmp.path().join(".opencode").exists());
        assert!(extra.exists());
    }

    #[test]
    fn clean_preserves_agents_dir_if_not_empty() {
        let tmp = TempDir::new().unwrap();
        create_agent_file(tmp.path(), "arch-principal.md");
        // Leave a non-arch file so agents/ is not empty after cleanup.
        create_agent_file(tmp.path(), "my-agent.md");
        clean_agent_files(tmp.path()).unwrap();
        assert!(tmp.path().join(".opencode").join("agents").exists());
    }
}
