use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("opencode binary not found in PATH — install opencode and ensure it is on your PATH")]
    OpenCodeNotFound,

    #[error("failed to create .opencode/agents/ directory: {0}")]
    AgentDirCreation(#[source] std::io::Error),

    #[error("failed to write agent file: {0}")]
    AgentFileWrite(#[source] std::io::Error),

    #[error("failed to determine current working directory: {0}")]
    CurrentDir(#[source] std::io::Error),

    #[error("failed to launch opencode: {0}")]
    LaunchFailed(#[source] std::io::Error),
}
