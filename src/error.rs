use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("opencode binary not found in PATH — install opencode and ensure it is on your PATH")]
    OpenCodeNotFound,

    #[error("failed to create .opencode/agents/ directory: {0}")]
    AgentDirCreation(#[source] std::io::Error),

    #[error("failed to write agent file: {0}")]
    AgentFileWrite(#[source] std::io::Error),

    #[error("failed to create reviews/ directory: {0}")]
    ReviewsDirCreation(#[source] std::io::Error),

    #[error("failed to determine current working directory: {0}")]
    CurrentDir(#[source] std::io::Error),

    #[error("failed to launch opencode: {0}")]
    LaunchFailed(#[source] std::io::Error),

    #[error("failed to read .opencode/agents/ directory: {0}")]
    CleanReadDir(#[source] std::io::Error),

    #[error("failed to remove agent file {path}: {source}")]
    CleanRemoveFile {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to remove empty directory {path}: {source}")]
    CleanRemoveDir {
        path: String,
        #[source]
        source: std::io::Error,
    },
}
