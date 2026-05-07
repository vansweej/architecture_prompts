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

    // ── debate errors ──────────────────────────────────────────────────────────

    #[error("debate round {round} agent {agent} failed with exit code {code}")]
    DebateAgentFailed { round: u8, agent: String, code: i32 },

    #[error("debate round {round} agent {agent} did not produce expected output at {path}")]
    DebateOutputMissing { round: u8, agent: String, path: String },

    #[error("failed to read debate report {path}: {source}")]
    DebateReportRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to create debate round directory: {0}")]
    DebateRoundDirCreation(#[source] std::io::Error),

    #[error("failed to spawn opencode for debate: {0}")]
    DebateSpawnFailed(#[source] std::io::Error),
}
