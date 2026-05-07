use clap::Parser;

use crate::prompts::ArchitectType;

#[derive(Debug, Parser)]
#[command(
    name = "architecture_prompts",
    about = "Activate an architect system prompt for an opencode session",
    long_about = "Writes a project-local opencode agent file with the selected architect \
                  system prompt embedded at compile time, then launches opencode with \
                  that agent active.\n\n\
                  The agent file is written to .opencode/agents/ in the current directory. \
                  Add .opencode/agents/arch-*.md to your .gitignore to avoid committing \
                  auto-generated files.\n\n\
                  Run --clean after a review session to remove all generated agent files \
                  and clean up empty directories."
)]
pub struct Cli {
    /// The architect persona to activate.
    ///
    /// One of: principal, design, complexity, security
    #[arg(value_enum, required_unless_present_any = ["list", "clean", "debate"])]
    pub architect: Option<ArchitectType>,

    /// Launch opencode with full permissions (default: read-only).
    ///
    /// By default the agent denies file edits and restricts bash to read-only
    /// git commands. Pass --full to allow all edits and bash commands.
    #[arg(long, default_value_t = false)]
    pub full: bool,

    /// List all available architect prompts with descriptions, then exit.
    #[arg(long, default_value_t = false)]
    pub list: bool,

    /// Print the generated agent .md content to stdout without writing any
    /// files or launching opencode.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Override the default LLM model for this persona.
    ///
    /// Format: provider/model (e.g., github-copilot/claude-opus-4.6).
    /// If omitted, each persona uses its built-in default model.
    #[arg(long, short = 'm')]
    pub model: Option<String>,

    /// Run in review mode: read-only for all repo files, but the persona may
    /// write its findings to `reviews/arch-<persona>-<date>.md`.
    ///
    /// The `reviews/` directory is created automatically before launching
    /// opencode. Mutually exclusive with --full.
    #[arg(long, default_value_t = false, conflicts_with = "full")]
    pub review: bool,

    /// Remove all arch-*.md agent files from .opencode/agents/ in the current
    /// directory, then remove the directory (and .opencode/) if empty.
    ///
    /// Does not launch opencode. Useful for cleaning up after a review session.
    /// The reviews/ directory is left untouched.
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["full", "review", "dry_run"]
    )]
    pub clean: bool,

    /// Run the multi-round architect debate pipeline.
    ///
    /// Launches all four architect personas in a coordinated multi-round review:
    /// Round 1 (independent assessments) → Round 2 (peer challenge/endorsement)
    /// → Synthesis (moderator final report). All output is written to
    /// `reviews/round1/`, `reviews/round2/`, and `reviews/final-report.md`.
    ///
    /// Mutually exclusive with all single-agent flags and the positional
    /// architect argument.
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["full", "review", "clean", "dry_run", "architect"]
    )]
    pub debate: bool,

    /// Maximum number of concurrent opencode processes per debate round.
    ///
    /// Controls how many architect agents run in parallel during Round 1 and
    /// Round 2. Must be used together with `--debate`. Minimum effective value
    /// is 1 (zero is treated as 1).
    #[arg(long, default_value_t = 4, requires = "debate")]
    pub concurrency: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("architecture_prompts").chain(args.iter().copied()))
    }

    #[test]
    fn parses_principal() {
        let cli = parse(&["principal"]).unwrap();
        assert!(matches!(cli.architect, Some(ArchitectType::Principal)));
        assert!(!cli.full);
        assert!(!cli.dry_run);
        assert!(!cli.list);
    }

    #[test]
    fn parses_design() {
        let cli = parse(&["design"]).unwrap();
        assert!(matches!(cli.architect, Some(ArchitectType::Design)));
    }

    #[test]
    fn parses_complexity() {
        let cli = parse(&["complexity"]).unwrap();
        assert!(matches!(cli.architect, Some(ArchitectType::Complexity)));
    }

    #[test]
    fn parses_security() {
        let cli = parse(&["security"]).unwrap();
        assert!(matches!(cli.architect, Some(ArchitectType::Security)));
    }

    #[test]
    fn parses_full_flag() {
        let cli = parse(&["principal", "--full"]).unwrap();
        assert!(cli.full);
    }

    #[test]
    fn parses_dry_run_flag() {
        let cli = parse(&["principal", "--dry-run"]).unwrap();
        assert!(cli.dry_run);
    }

    #[test]
    fn parses_list_flag_without_architect() {
        let cli = parse(&["--list"]).unwrap();
        assert!(cli.list);
        assert!(cli.architect.is_none());
    }

    #[test]
    fn rejects_invalid_architect() {
        assert!(parse(&["invalid"]).is_err());
    }

    #[test]
    fn rejects_missing_architect_without_list() {
        assert!(parse(&[]).is_err());
    }

    #[test]
    fn parses_model_long_flag() {
        let cli = parse(&["principal", "--model", "github-copilot/claude-opus-4.6"]).unwrap();
        assert_eq!(cli.model.as_deref(), Some("github-copilot/claude-opus-4.6"));
    }

    #[test]
    fn parses_model_short_flag() {
        let cli = parse(&["principal", "-m", "openai/gpt-5"]).unwrap();
        assert_eq!(cli.model.as_deref(), Some("openai/gpt-5"));
    }

    #[test]
    fn model_defaults_to_none() {
        let cli = parse(&["principal"]).unwrap();
        assert!(cli.model.is_none());
    }

    #[test]
    fn parses_review_flag() {
        let cli = parse(&["principal", "--review"]).unwrap();
        assert!(cli.review);
        assert!(!cli.full);
    }

    #[test]
    fn review_defaults_to_false() {
        let cli = parse(&["principal"]).unwrap();
        assert!(!cli.review);
    }

    #[test]
    fn review_and_full_are_mutually_exclusive() {
        assert!(parse(&["principal", "--review", "--full"]).is_err());
    }

    #[test]
    fn parses_clean_flag_without_architect() {
        let cli = parse(&["--clean"]).unwrap();
        assert!(cli.clean);
        assert!(cli.architect.is_none());
    }

    #[test]
    fn clean_defaults_to_false() {
        let cli = parse(&["principal"]).unwrap();
        assert!(!cli.clean);
    }

    #[test]
    fn clean_conflicts_with_full() {
        assert!(parse(&["--clean", "--full"]).is_err());
    }

    #[test]
    fn clean_conflicts_with_review() {
        assert!(parse(&["--clean", "--review"]).is_err());
    }

    #[test]
    fn clean_conflicts_with_dry_run() {
        assert!(parse(&["--clean", "--dry-run"]).is_err());
    }

    // ── --debate ──────────────────────────────────────────────────────────────

    #[test]
    fn debate_flag_parses() {
        let cli = parse(&["--debate"]).unwrap();
        assert!(cli.debate);
    }

    #[test]
    fn debate_flag_is_false_by_default() {
        let cli = parse(&["principal"]).unwrap();
        assert!(!cli.debate);
    }

    #[test]
    fn debate_requires_no_architect() {
        let cli = parse(&["--debate"]).unwrap();
        assert!(cli.architect.is_none());
    }

    #[test]
    fn debate_conflicts_with_full() {
        assert!(parse(&["--debate", "--full"]).is_err());
    }

    #[test]
    fn debate_conflicts_with_review() {
        assert!(parse(&["--debate", "--review"]).is_err());
    }

    #[test]
    fn debate_conflicts_with_clean() {
        assert!(parse(&["--debate", "--clean"]).is_err());
    }

    #[test]
    fn debate_conflicts_with_dry_run() {
        assert!(parse(&["--debate", "--dry-run"]).is_err());
    }

    #[test]
    fn debate_conflicts_with_architect() {
        assert!(parse(&["--debate", "principal"]).is_err());
    }

    // ── --concurrency ─────────────────────────────────────────────────────────

    #[test]
    fn concurrency_parses_with_debate() {
        let cli = parse(&["--debate", "--concurrency", "2"]).unwrap();
        assert_eq!(cli.concurrency, 2);
    }

    #[test]
    fn concurrency_defaults_to_four() {
        let cli = parse(&["--debate"]).unwrap();
        assert_eq!(cli.concurrency, 4);
    }

    #[test]
    fn concurrency_requires_debate() {
        assert!(parse(&["--concurrency", "2"]).is_err());
    }
}
