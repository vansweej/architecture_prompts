// Phase 2: orchestration engine for the multi-round architect debate.

use std::path::{Path, PathBuf};

use crate::debate_agent::{DebateContext, DebateRound, PeerReport};
use crate::debate_agent::{generate_debate_agent, generate_moderator_agent};
use crate::error::AppError;
use crate::launcher::ensure_agent_dir;
use crate::prompts::{ArchitectType, DebateRole};

/// Prompt passed as the user-turn to opencode in each round.
const ROUND1_PROMPT: &str = "Perform a thorough architectural review of this codebase.";
const ROUND2_PROMPT: &str =
    "Review your Round 1 findings in light of your peers' reports and produce your Round 2 response.";
const SYNTHESIS_PROMPT: &str = "Synthesize all architect reports into a final verdict.";

// ── config ────────────────────────────────────────────────────────────────────

/// Configuration for a multi-round architect debate run.
pub struct DebateConfig {
    /// Override the LLM model for all debate agents.
    /// If `None`, each persona uses its built-in default model.
    pub model: Option<String>,
    /// Maximum number of concurrent `opencode run` processes per round.
    /// Minimum effective value is 1 (zero is treated as 1).
    pub concurrency: usize,
    /// Working directory — debate output files are written relative to this path.
    pub base_dir: PathBuf,
    /// Reserved for Phase 4 — designates one architect as devil's advocate in
    /// Round 2 so it challenges all consensus findings instead of the standard
    /// challenge/endorse flow.
    pub devils_advocate: Option<ArchitectType>,
}

impl DebateConfig {
    /// Returns the model to use: the global override if set, otherwise the
    /// per-persona default.
    fn effective_model<'a>(&'a self, default: &'a str) -> &'a str {
        self.model.as_deref().unwrap_or(default)
    }
}

// ── process runner ────────────────────────────────────────────────────────────

/// Abstraction over subprocess spawning, enabling unit-testing without a live
/// opencode binary.
///
/// Both `RealRunner` (production) and `MockRunner` (tests) implement this
/// trait. The `Send + Sync` bound is required because `spawn_batch` runs
/// agents on multiple threads simultaneously.
pub trait ProcessRunner: Send + Sync {
    /// Runs `opencode run --agent <agent_name> "<prompt>"` (or a test
    /// equivalent).
    ///
    /// Returns `Err(AppError::DebateAgentFailed { round: 0, .. })` when the
    /// process exits non-zero; the calling orchestration function re-tags the
    /// `round` field with the correct value.
    fn run_agent(&self, agent_name: &str, prompt: &str) -> Result<(), AppError>;
}

/// Production runner — spawns real `opencode run` subprocesses.
// Excluded from coverage: requires a live opencode binary and a TUI session.
#[cfg(not(tarpaulin_include))]
pub struct RealRunner;

#[cfg(not(tarpaulin_include))]
impl ProcessRunner for RealRunner {
    fn run_agent(&self, agent_name: &str, prompt: &str) -> Result<(), AppError> {
        let status = std::process::Command::new("opencode")
            .args(["run", "--agent", agent_name, prompt])
            .status()
            .map_err(AppError::DebateSpawnFailed)?;

        if !status.success() {
            return Err(AppError::DebateAgentFailed {
                round: 0, // re-tagged by the calling orchestration function
                agent: agent_name.to_string(),
                code: status.code().unwrap_or(-1),
            });
        }
        Ok(())
    }
}

// ── directory helpers ─────────────────────────────────────────────────────────

/// Creates `<base>/reviews/round1/` and `<base>/reviews/round2/` (and any
/// intermediate paths) if they do not already exist.
///
/// The `reviews/` parent is created as a side-effect of `create_dir_all`.
pub fn ensure_round_dirs(base: &Path) -> Result<(), AppError> {
    for subdir in &["round1", "round2"] {
        let path = base.join("reviews").join(subdir);
        std::fs::create_dir_all(&path).map_err(AppError::DebateRoundDirCreation)?;
    }
    Ok(())
}

// ── orchestration functions ───────────────────────────────────────────────────

/// Round 1: each of the four architect personas independently reviews the
/// codebase and writes its findings to `reviews/round1/arch-<name>.md`.
///
/// - Generates and writes an agent file per architect.
/// - Spawns up to `config.concurrency` processes at a time.
/// - Verifies all four output files exist after each batch; fails fast if any
///   are missing.
pub fn run_round1<R: ProcessRunner>(config: &DebateConfig, runner: &R) -> Result<(), AppError> {
    let architects = ArchitectType::all();
    let concurrency = config.concurrency.max(1);

    let mut tasks: Vec<(String, String)> = Vec::with_capacity(architects.len());
    for &architect in architects {
        let model = config.effective_model(architect.default_model());
        let context = DebateContext {
            round: DebateRound::Round1,
            own_report: None,
            peer_reports: vec![],
            is_devils_advocate: false,
        };
        let content = generate_debate_agent(architect, &context, model);
        write_named_agent_file(&config.base_dir, architect.agent_name(), &content)?;
        tasks.push((architect.agent_name().to_string(), ROUND1_PROMPT.to_string()));
    }

    for chunk in tasks.chunks(concurrency) {
        spawn_batch(chunk, 1, runner)?;
    }

    verify_round_outputs(&config.base_dir, "round1", architects, 1)?;
    Ok(())
}

/// Round 2: each architect reads its own Round 1 report plus the three peer
/// Round 1 reports, then produces a challenge/endorsement response written to
/// `reviews/round2/arch-<name>.md`.
///
/// - Reads all four `reviews/round1/arch-*.md` files; fails fast on read error.
/// - Injects own report and peer reports into each agent's context.
/// - Spawns up to `config.concurrency` processes at a time.
/// - Verifies all four output files exist after each batch.
pub fn run_round2<R: ProcessRunner>(config: &DebateConfig, runner: &R) -> Result<(), AppError> {
    let architects = ArchitectType::all();
    let concurrency = config.concurrency.max(1);

    // Read all round-1 reports upfront.
    let round1_contents: Vec<String> = architects
        .iter()
        .map(|&architect| {
            let path = round_output_path(&config.base_dir, "round1", architect.agent_name());
            std::fs::read_to_string(&path).map_err(|source| AppError::DebateReportRead {
                path: path.display().to_string(),
                source,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut tasks: Vec<(String, String)> = Vec::with_capacity(architects.len());
    for (i, &architect) in architects.iter().enumerate() {
        let peers: Vec<PeerReport<'_>> = architects
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(j, &a)| PeerReport {
                agent_name: a.agent_name(),
                content: round1_contents[j].as_str(),
            })
            .collect();

        let model = config.effective_model(architect.default_model());
        let context = DebateContext {
            round: DebateRound::Round2,
            own_report: Some(round1_contents[i].as_str()),
            peer_reports: peers,
            is_devils_advocate: config.devils_advocate == Some(architect),
        };
        let content = generate_debate_agent(architect, &context, model);
        write_named_agent_file(&config.base_dir, architect.agent_name(), &content)?;
        tasks.push((architect.agent_name().to_string(), ROUND2_PROMPT.to_string()));
    }

    for chunk in tasks.chunks(concurrency) {
        spawn_batch(chunk, 2, runner)?;
    }

    verify_round_outputs(&config.base_dir, "round2", architects, 2)?;
    Ok(())
}

/// Synthesis: the moderator reads all eight reports (4 × Round 1 + 4 × Round 2)
/// and produces `reviews/final-report.md`.
///
/// - Reads all eight report files; fails fast on read error.
/// - Injects all reports inline into the moderator agent file.
/// - Spawns a single `opencode run` process.
/// - Verifies `reviews/final-report.md` exists.
pub fn run_synthesis<R: ProcessRunner>(config: &DebateConfig, runner: &R) -> Result<(), AppError> {
    let architects = ArchitectType::all();

    // Read all 8 reports: round1 × 4 then round2 × 4.
    let mut all_report_contents: Vec<(String, String)> = Vec::with_capacity(8);
    for round in &["round1", "round2"] {
        for &architect in architects {
            let label = format!("{}-{}", architect.agent_name(), round);
            let path = round_output_path(&config.base_dir, round, architect.agent_name());
            let content =
                std::fs::read_to_string(&path).map_err(|source| AppError::DebateReportRead {
                    path: path.display().to_string(),
                    source,
                })?;
            all_report_contents.push((label, content));
        }
    }

    let peer_reports: Vec<PeerReport<'_>> = all_report_contents
        .iter()
        .map(|(name, content)| PeerReport {
            agent_name: name.as_str(),
            content: content.as_str(),
        })
        .collect();

    let model = config.effective_model(DebateRole::Moderator.default_model());
    let moderator_content = generate_moderator_agent(
        &peer_reports,
        model,
        config.devils_advocate.as_ref().map(|a| a.agent_name()),
    );
    write_named_agent_file(
        &config.base_dir,
        DebateRole::Moderator.agent_name(),
        &moderator_content,
    )?;

    spawn_batch(
        &[(
            DebateRole::Moderator.agent_name().to_string(),
            SYNTHESIS_PROMPT.to_string(),
        )],
        3,
        runner,
    )?;

    let final_report = config.base_dir.join("reviews").join("final-report.md");
    if !final_report.exists() {
        return Err(AppError::DebateOutputMissing {
            round: 3,
            agent: DebateRole::Moderator.agent_name().to_string(),
            path: final_report.display().to_string(),
        });
    }

    Ok(())
}

/// Full debate pipeline:
/// 1. Create `reviews/round1/` and `reviews/round2/` directories.
/// 2. Run Round 1 — four independent assessments.
/// 3. Run Round 2 — peer challenge/endorsement.
/// 4. Run synthesis — moderator final report.
pub fn run_debate<R: ProcessRunner>(config: &DebateConfig, runner: &R) -> Result<(), AppError> {
    ensure_round_dirs(&config.base_dir)?;
    run_round1(config, runner)?;
    run_round2(config, runner)?;
    run_synthesis(config, runner)?;
    Ok(())
}

// ── private helpers ───────────────────────────────────────────────────────────

/// Spawns all agents in `batch` on parallel threads (bounded by `batch.len()`),
/// waits for all to complete, then returns the first error re-tagged with
/// `round`.  Non-zero thread panics are surfaced as `DebateAgentFailed`.
fn spawn_batch<R: ProcessRunner>(
    batch: &[(String, String)],
    round: u8,
    runner: &R,
) -> Result<(), AppError> {
    let results: Vec<Result<(), AppError>> = std::thread::scope(|s| {
        let handles: Vec<_> = batch
            .iter()
            .map(|(name, prompt)| s.spawn(|| runner.run_agent(name.as_str(), prompt.as_str())))
            .collect();

        handles
            .into_iter()
            .zip(batch.iter())
            .map(|(handle, (name, _))| {
                handle.join().unwrap_or_else(|_| {
                    Err(AppError::DebateAgentFailed {
                        round,
                        agent: name.clone(),
                        code: -1,
                    })
                })
            })
            .collect()
    });

    for result in results {
        result.map_err(|e| match e {
            AppError::DebateAgentFailed { agent, code, .. } => {
                AppError::DebateAgentFailed { round, agent, code }
            }
            other => other,
        })?;
    }

    Ok(())
}

/// Verifies that `<base>/reviews/<round_dir>/arch-<name>.md` exists for each
/// architect in `architects`.  Returns `DebateOutputMissing` on the first
/// missing file.
fn verify_round_outputs(
    base: &Path,
    round_dir: &str,
    architects: &[ArchitectType],
    round: u8,
) -> Result<(), AppError> {
    for &architect in architects {
        let path = round_output_path(base, round_dir, architect.agent_name());
        if !path.exists() {
            return Err(AppError::DebateOutputMissing {
                round,
                agent: architect.agent_name().to_string(),
                path: path.display().to_string(),
            });
        }
    }
    Ok(())
}

/// Writes `content` to `<base>/.opencode/agents/<name>.md`.
fn write_named_agent_file(base: &Path, name: &str, content: &str) -> Result<(), AppError> {
    let dir = ensure_agent_dir(base)?;
    let path = dir.join(format!("{name}.md"));
    std::fs::write(&path, content).map_err(AppError::AgentFileWrite)?;
    Ok(())
}

/// Returns `<base>/reviews/<round_dir>/<agent_name>.md`.
fn round_output_path(base: &Path, round_dir: &str, agent_name: &str) -> PathBuf {
    base.join("reviews").join(round_dir).join(format!("{agent_name}.md"))
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── test runners ──────────────────────────────────────────────────────────

    /// Writes the expected output file based on file-system state:
    /// - For `arch-moderator` → `reviews/final-report.md`
    /// - For other agents → `reviews/round1/<name>.md` if it doesn't exist yet,
    ///   otherwise `reviews/round2/<name>.md`
    struct MockRunner {
        base_dir: PathBuf,
    }

    impl ProcessRunner for MockRunner {
        fn run_agent(&self, agent_name: &str, _prompt: &str) -> Result<(), AppError> {
            if agent_name == DebateRole::Moderator.agent_name() {
                let path = self.base_dir.join("reviews").join("final-report.md");
                fs::write(&path, format!("# Mock final report by {agent_name}\n"))
                    .map_err(AppError::AgentFileWrite)?;
            } else {
                let r1 = round_output_path(&self.base_dir, "round1", agent_name);
                let r2 = round_output_path(&self.base_dir, "round2", agent_name);
                if !r1.exists() {
                    fs::write(&r1, format!("# Mock Round 1: {agent_name}\n"))
                        .map_err(AppError::AgentFileWrite)?;
                } else {
                    fs::write(&r2, format!("# Mock Round 2: {agent_name}\n"))
                        .map_err(AppError::AgentFileWrite)?;
                }
            }
            Ok(())
        }
    }

    /// Returns `Ok(())` without writing anything.
    struct NoOpRunner;

    impl ProcessRunner for NoOpRunner {
        fn run_agent(&self, _: &str, _: &str) -> Result<(), AppError> {
            Ok(())
        }
    }

    /// Always returns `DebateAgentFailed` (exit code 1).
    struct FailRunner;

    impl ProcessRunner for FailRunner {
        fn run_agent(&self, name: &str, _: &str) -> Result<(), AppError> {
            Err(AppError::DebateAgentFailed {
                round: 0,
                agent: name.to_string(),
                code: 1,
            })
        }
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    fn make_config(tmp: &TempDir) -> DebateConfig {
        DebateConfig {
            model: None,
            concurrency: 4,
            base_dir: tmp.path().to_path_buf(),
            devils_advocate: None,
        }
    }

    fn make_config_sequential(tmp: &TempDir) -> DebateConfig {
        DebateConfig {
            model: None,
            concurrency: 1,
            base_dir: tmp.path().to_path_buf(),
            devils_advocate: None,
        }
    }

    /// Pre-writes all 4 round1 report files so run_round2 can read them.
    fn seed_round1(base: &Path) {
        let dir = base.join("reviews").join("round1");
        fs::create_dir_all(&dir).unwrap();
        for arch in ArchitectType::all() {
            let path = round_output_path(base, "round1", arch.agent_name());
            fs::write(&path, format!("# Seeded round 1: {}\n", arch.agent_name())).unwrap();
        }
    }

    /// Pre-writes all 8 report files (round1 + round2) for run_synthesis.
    fn seed_all_reports(base: &Path) {
        seed_round1(base);
        let dir = base.join("reviews").join("round2");
        fs::create_dir_all(&dir).unwrap();
        for arch in ArchitectType::all() {
            let path = round_output_path(base, "round2", arch.agent_name());
            fs::write(&path, format!("# Seeded round 2: {}\n", arch.agent_name())).unwrap();
        }
    }

    // ── ensure_round_dirs ─────────────────────────────────────────────────────

    #[test]
    fn ensure_round_dirs_creates_round1_dir() {
        let tmp = TempDir::new().unwrap();
        ensure_round_dirs(tmp.path()).unwrap();
        assert!(tmp.path().join("reviews").join("round1").is_dir());
    }

    #[test]
    fn ensure_round_dirs_creates_round2_dir() {
        let tmp = TempDir::new().unwrap();
        ensure_round_dirs(tmp.path()).unwrap();
        assert!(tmp.path().join("reviews").join("round2").is_dir());
    }

    #[test]
    fn ensure_round_dirs_creates_reviews_parent() {
        let tmp = TempDir::new().unwrap();
        ensure_round_dirs(tmp.path()).unwrap();
        assert!(tmp.path().join("reviews").is_dir());
    }

    #[test]
    fn ensure_round_dirs_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        ensure_round_dirs(tmp.path()).unwrap();
        ensure_round_dirs(tmp.path()).unwrap(); // second call must not fail
    }

    // ── run_round1 ────────────────────────────────────────────────────────────

    #[test]
    fn run_round1_writes_four_agent_files() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        ensure_round_dirs(tmp.path()).unwrap();
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_round1(&config, &runner).unwrap();

        for arch in ArchitectType::all() {
            let agent_file = tmp
                .path()
                .join(".opencode")
                .join("agents")
                .join(format!("{}.md", arch.agent_name()));
            assert!(agent_file.exists(), "agent file missing: {}", arch.agent_name());
        }
    }

    #[test]
    fn run_round1_produces_four_output_files() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        ensure_round_dirs(tmp.path()).unwrap();
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_round1(&config, &runner).unwrap();

        for arch in ArchitectType::all() {
            let path = round_output_path(tmp.path(), "round1", arch.agent_name());
            assert!(path.exists(), "round1 output missing: {}", arch.agent_name());
        }
    }

    #[test]
    fn run_round1_concurrency_1_produces_same_outputs() {
        let tmp = TempDir::new().unwrap();
        let config = make_config_sequential(&tmp);
        ensure_round_dirs(tmp.path()).unwrap();
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_round1(&config, &runner).unwrap();

        for arch in ArchitectType::all() {
            let path = round_output_path(tmp.path(), "round1", arch.agent_name());
            assert!(path.exists());
        }
    }

    #[test]
    fn run_round1_uses_model_override() {
        let tmp = TempDir::new().unwrap();
        let config = DebateConfig {
            model: Some("openai/gpt-5".to_string()),
            concurrency: 4,
            base_dir: tmp.path().to_path_buf(),
            devils_advocate: None,
        };
        ensure_round_dirs(tmp.path()).unwrap();
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_round1(&config, &runner).unwrap();

        // The agent file for principal should contain the override model.
        let agent_file = tmp
            .path()
            .join(".opencode")
            .join("agents")
            .join("arch-principal.md");
        let content = fs::read_to_string(agent_file).unwrap();
        assert!(content.contains("openai/gpt-5"));
    }

    #[test]
    fn run_round1_uses_per_architect_default_when_no_override() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        ensure_round_dirs(tmp.path()).unwrap();
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_round1(&config, &runner).unwrap();

        let agent_file = tmp
            .path()
            .join(".opencode")
            .join("agents")
            .join("arch-principal.md");
        let content = fs::read_to_string(agent_file).unwrap();
        assert!(content.contains(ArchitectType::Principal.default_model()));
    }

    #[test]
    fn run_round1_returns_debate_output_missing_when_runner_writes_nothing() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        ensure_round_dirs(tmp.path()).unwrap();
        let err = run_round1(&config, &NoOpRunner).unwrap_err();
        assert!(
            matches!(err, AppError::DebateOutputMissing { round: 1, .. }),
            "expected DebateOutputMissing(round=1), got: {err:?}"
        );
    }

    #[test]
    fn run_round1_returns_debate_agent_failed_when_runner_errors() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        ensure_round_dirs(tmp.path()).unwrap();
        let err = run_round1(&config, &FailRunner).unwrap_err();
        assert!(
            matches!(err, AppError::DebateAgentFailed { round: 1, code: 1, .. }),
            "expected DebateAgentFailed(round=1, code=1), got: {err:?}"
        );
    }

    #[test]
    fn run_round1_zero_concurrency_treated_as_one() {
        let tmp = TempDir::new().unwrap();
        let config = DebateConfig {
            model: None,
            concurrency: 0,
            base_dir: tmp.path().to_path_buf(),
            devils_advocate: None,
        };
        ensure_round_dirs(tmp.path()).unwrap();
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        // Must not panic on chunks(0).
        run_round1(&config, &runner).unwrap();
    }

    // ── run_round2 ────────────────────────────────────────────────────────────

    #[test]
    fn run_round2_produces_four_output_files() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        ensure_round_dirs(tmp.path()).unwrap();
        seed_round1(tmp.path());
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_round2(&config, &runner).unwrap();

        for arch in ArchitectType::all() {
            let path = round_output_path(tmp.path(), "round2", arch.agent_name());
            assert!(path.exists(), "round2 output missing: {}", arch.agent_name());
        }
    }

    #[test]
    fn run_round2_injects_peer_context_into_agent_files() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        ensure_round_dirs(tmp.path()).unwrap();
        seed_round1(tmp.path());
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_round2(&config, &runner).unwrap();

        // The round2 principal agent file must contain the seeded complexity report.
        let agent_file = tmp
            .path()
            .join(".opencode")
            .join("agents")
            .join("arch-principal.md");
        let content = fs::read_to_string(agent_file).unwrap();
        assert!(
            content.contains("arch-complexity"),
            "round2 agent must inject peer arch-complexity content"
        );
    }

    #[test]
    fn run_round2_injects_own_round1_report() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        ensure_round_dirs(tmp.path()).unwrap();
        seed_round1(tmp.path());
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_round2(&config, &runner).unwrap();

        let agent_file = tmp
            .path()
            .join(".opencode")
            .join("agents")
            .join("arch-principal.md");
        let content = fs::read_to_string(agent_file).unwrap();
        assert!(
            content.contains("Seeded round 1: arch-principal"),
            "round2 agent must inject its own round1 report"
        );
    }

    #[test]
    fn run_round2_returns_debate_report_read_when_round1_missing() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        ensure_round_dirs(tmp.path()).unwrap();
        // Do NOT seed round1 files — they should be missing.
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        let err = run_round2(&config, &runner).unwrap_err();
        assert!(
            matches!(err, AppError::DebateReportRead { .. }),
            "expected DebateReportRead, got: {err:?}"
        );
    }

    #[test]
    fn run_round2_returns_debate_output_missing_when_runner_writes_nothing() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        ensure_round_dirs(tmp.path()).unwrap();
        seed_round1(tmp.path());
        let err = run_round2(&config, &NoOpRunner).unwrap_err();
        assert!(
            matches!(err, AppError::DebateOutputMissing { round: 2, .. }),
            "expected DebateOutputMissing(round=2), got: {err:?}"
        );
    }

    #[test]
    fn run_round2_returns_debate_agent_failed_when_runner_errors() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        ensure_round_dirs(tmp.path()).unwrap();
        seed_round1(tmp.path());
        let err = run_round2(&config, &FailRunner).unwrap_err();
        assert!(
            matches!(err, AppError::DebateAgentFailed { round: 2, .. }),
            "expected DebateAgentFailed(round=2), got: {err:?}"
        );
    }

    // ── run_synthesis ─────────────────────────────────────────────────────────

    #[test]
    fn run_synthesis_produces_final_report() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        seed_all_reports(tmp.path());
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_synthesis(&config, &runner).unwrap();
        assert!(tmp.path().join("reviews").join("final-report.md").exists());
    }

    #[test]
    fn run_synthesis_writes_moderator_agent_file() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        seed_all_reports(tmp.path());
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_synthesis(&config, &runner).unwrap();

        let agent_file = tmp
            .path()
            .join(".opencode")
            .join("agents")
            .join("arch-moderator.md");
        assert!(agent_file.exists());
    }

    #[test]
    fn run_synthesis_moderator_agent_contains_all_reports() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        seed_all_reports(tmp.path());
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_synthesis(&config, &runner).unwrap();

        let agent_file = tmp
            .path()
            .join(".opencode")
            .join("agents")
            .join("arch-moderator.md");
        let content = fs::read_to_string(agent_file).unwrap();
        for arch in ArchitectType::all() {
            assert!(
                content.contains(arch.agent_name()),
                "moderator agent file must reference {}",
                arch.agent_name()
            );
        }
    }

    #[test]
    fn run_synthesis_returns_debate_report_read_when_report_missing() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        // Only seed round1, not round2 — synthesis needs both.
        seed_round1(tmp.path());
        fs::create_dir_all(tmp.path().join("reviews").join("round2")).unwrap();
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        let err = run_synthesis(&config, &runner).unwrap_err();
        assert!(
            matches!(err, AppError::DebateReportRead { .. }),
            "expected DebateReportRead, got: {err:?}"
        );
    }

    #[test]
    fn run_synthesis_returns_debate_output_missing_when_runner_writes_nothing() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        seed_all_reports(tmp.path());
        let err = run_synthesis(&config, &NoOpRunner).unwrap_err();
        assert!(
            matches!(err, AppError::DebateOutputMissing { round: 3, .. }),
            "expected DebateOutputMissing(round=3), got: {err:?}"
        );
    }

    // ── run_debate ────────────────────────────────────────────────────────────

    #[test]
    fn run_debate_produces_all_nine_output_files() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_debate(&config, &runner).unwrap();

        for arch in ArchitectType::all() {
            assert!(
                round_output_path(tmp.path(), "round1", arch.agent_name()).exists(),
                "missing round1/{}",
                arch.agent_name()
            );
            assert!(
                round_output_path(tmp.path(), "round2", arch.agent_name()).exists(),
                "missing round2/{}",
                arch.agent_name()
            );
        }
        assert!(tmp.path().join("reviews").join("final-report.md").exists());
    }

    #[test]
    fn run_debate_sequential_concurrency_produces_all_outputs() {
        let tmp = TempDir::new().unwrap();
        let config = make_config_sequential(&tmp);
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_debate(&config, &runner).unwrap();
        assert!(tmp.path().join("reviews").join("final-report.md").exists());
    }

    #[test]
    fn run_debate_fails_fast_on_round1_agent_failure() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        let err = run_debate(&config, &FailRunner).unwrap_err();
        // Round 1 fails before round 2 is attempted.
        assert!(
            matches!(err, AppError::DebateAgentFailed { round: 1, .. }),
            "expected DebateAgentFailed(round=1), got: {err:?}"
        );
        // Round 2 output files must not exist.
        for arch in ArchitectType::all() {
            assert!(!round_output_path(tmp.path(), "round2", arch.agent_name()).exists());
        }
    }

    // ── DebateConfig helpers ──────────────────────────────────────────────────

    #[test]
    fn effective_model_returns_override_when_set() {
        let tmp = TempDir::new().unwrap();
        let config = DebateConfig {
            model: Some("openai/gpt-5".to_string()),
            concurrency: 1,
            base_dir: tmp.path().to_path_buf(),
            devils_advocate: None,
        };
        assert_eq!(config.effective_model("default/model"), "openai/gpt-5");
    }

    #[test]
    fn effective_model_returns_default_when_no_override() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        assert_eq!(config.effective_model("default/model"), "default/model");
    }

    // ── devil's advocate — run_round2 ─────────────────────────────────────────

    #[test]
    fn run_round2_da_agent_file_contains_da_template_content() {
        let tmp = TempDir::new().unwrap();
        let config = DebateConfig {
            model: None,
            concurrency: 4,
            base_dir: tmp.path().to_path_buf(),
            devils_advocate: Some(ArchitectType::Complexity),
        };
        ensure_round_dirs(tmp.path()).unwrap();
        seed_round1(tmp.path());
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_round2(&config, &runner).unwrap();

        // The arch-complexity agent file must use the DA template.
        let agent_file = tmp
            .path()
            .join(".opencode")
            .join("agents")
            .join("arch-complexity.md");
        let content = fs::read_to_string(agent_file).unwrap();
        assert!(
            content.contains("devil's advocate"),
            "DA-designated agent file must contain DA template wording"
        );
    }

    #[test]
    fn run_round2_non_da_agents_do_not_use_da_template() {
        let tmp = TempDir::new().unwrap();
        let config = DebateConfig {
            model: None,
            concurrency: 4,
            base_dir: tmp.path().to_path_buf(),
            devils_advocate: Some(ArchitectType::Complexity),
        };
        ensure_round_dirs(tmp.path()).unwrap();
        seed_round1(tmp.path());
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_round2(&config, &runner).unwrap();

        // Non-DA agents (principal, design, security) must use the standard template.
        for arch in &[ArchitectType::Principal, ArchitectType::Design, ArchitectType::Security] {
            let agent_file = tmp
                .path()
                .join(".opencode")
                .join("agents")
                .join(format!("{}.md", arch.agent_name()));
            let content = fs::read_to_string(&agent_file).unwrap();
            assert!(
                !content.contains("devil's advocate"),
                "{} must use the standard Round 2 template, not DA",
                arch.agent_name()
            );
        }
    }

    #[test]
    fn run_synthesis_moderator_file_contains_da_notice_when_da_set() {
        let tmp = TempDir::new().unwrap();
        let config = DebateConfig {
            model: None,
            concurrency: 4,
            base_dir: tmp.path().to_path_buf(),
            devils_advocate: Some(ArchitectType::Security),
        };
        seed_all_reports(tmp.path());
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_synthesis(&config, &runner).unwrap();

        let agent_file = tmp
            .path()
            .join(".opencode")
            .join("agents")
            .join("arch-moderator.md");
        let content = fs::read_to_string(agent_file).unwrap();
        assert!(
            content.contains("Devil's Advocate Notice"),
            "moderator agent file must include DA notice when DA is configured"
        );
        assert!(
            content.contains(ArchitectType::Security.agent_name()),
            "moderator DA notice must name the designated advocate"
        );
    }

    #[test]
    fn run_synthesis_moderator_file_has_no_da_notice_without_da() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(&tmp);
        seed_all_reports(tmp.path());
        let runner = MockRunner { base_dir: tmp.path().to_path_buf() };
        run_synthesis(&config, &runner).unwrap();

        let agent_file = tmp
            .path()
            .join(".opencode")
            .join("agents")
            .join("arch-moderator.md");
        let content = fs::read_to_string(agent_file).unwrap();
        assert!(
            !content.contains("Devil's Advocate Notice"),
            "moderator agent file must NOT include DA notice when no DA configured"
        );
    }

    // ── error display ─────────────────────────────────────────────────────────

    #[test]
    fn debate_agent_failed_error_message_contains_round_and_agent() {
        let err = AppError::DebateAgentFailed {
            round: 1,
            agent: "arch-principal".to_string(),
            code: 2,
        };
        let msg = err.to_string();
        assert!(msg.contains("1"), "must mention round number");
        assert!(msg.contains("arch-principal"), "must mention agent name");
        assert!(msg.contains("2"), "must mention exit code");
    }

    #[test]
    fn debate_output_missing_error_message_contains_path() {
        let err = AppError::DebateOutputMissing {
            round: 2,
            agent: "arch-design".to_string(),
            path: "reviews/round2/arch-design.md".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("reviews/round2/arch-design.md"));
    }
}
