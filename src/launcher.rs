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
}
