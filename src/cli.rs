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
                  auto-generated files."
)]
pub struct Cli {
    /// The architect persona to activate.
    ///
    /// One of: principal, design, complexity, security
    #[arg(value_enum, required_unless_present = "list")]
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
}
