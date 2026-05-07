use clap::ValueEnum;

const PRINCIPAL: &str = include_str!("../prompts/system/principal.md");
const DESIGN: &str = include_str!("../prompts/system/design.md");
const COMPLEXITY: &str = include_str!("../prompts/system/complexity.md");
const SECURITY: &str = include_str!("../prompts/system/security.md");
const MODERATOR: &str = include_str!("../prompts/system/moderator.md");

/// The four available architect personas, embedded at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ArchitectType {
    /// Principal software architect — system-level evaluation
    Principal,
    /// Architecture review board — formal Accept/Reject verdict
    Design,
    /// Simplicity-biased principal engineer — complexity audit
    Complexity,
    /// Security-conscious system architect — trust boundary review
    Security,
}

impl ArchitectType {
    /// Returns the embedded system prompt content.
    pub fn prompt(self) -> &'static str {
        match self {
            Self::Principal => PRINCIPAL,
            Self::Design => DESIGN,
            Self::Complexity => COMPLEXITY,
            Self::Security => SECURITY,
        }
    }

    /// Returns the opencode agent name (used for --agent flag and file name).
    pub fn agent_name(self) -> &'static str {
        match self {
            Self::Principal => "arch-principal",
            Self::Design => "arch-design",
            Self::Complexity => "arch-complexity",
            Self::Security => "arch-security",
        }
    }

    /// Returns a short human-readable description of this architect persona.
    pub fn description(self) -> &'static str {
        match self {
            Self::Principal => {
                "Evaluates architecture at system level: scalability, reliability, trade-offs, failure modes"
            }
            Self::Design => {
                "Architecture review board: renders Accept / Accept with concerns / Reject verdict"
            }
            Self::Complexity => {
                "Simplicity-biased audit: identifies accidental complexity and unjustified component count"
            }
            Self::Security => {
                "Reviews trust boundaries, blast radius, AuthN/AuthZ, and failure impact on C-I-A"
            }
        }
    }

    /// Returns the default LLM model for this persona.
    ///
    /// Broad/decisive personas (principal, design) use Opus for maximum
    /// reasoning depth. Focused/fast personas (complexity, security) use
    /// Sonnet, which is sufficient for their narrower scope.
    ///
    /// This default can be overridden at invocation time with `--model`.
    pub fn default_model(self) -> &'static str {
        match self {
            Self::Principal => "github-copilot/claude-opus-4.6",
            Self::Design => "github-copilot/claude-opus-4.6",
            Self::Complexity => "github-copilot/claude-sonnet-4.6",
            Self::Security => "github-copilot/claude-sonnet-4.6",
        }
    }

    /// Returns all variants in display order.
    pub fn all() -> &'static [Self] {
        &[
            Self::Principal,
            Self::Design,
            Self::Complexity,
            Self::Security,
        ]
    }
}

/// The moderator role used inside the multi-round debate pipeline.
///
/// `DebateRole` is intentionally separate from `ArchitectType`. `ArchitectType`
/// derives `clap::ValueEnum` and is exposed as a CLI argument for single-agent
/// invocations. The moderator is never invoked standalone by the user — it
/// exists only inside the debate pipeline — so mixing it into `ArchitectType`
/// would pollute the user-facing help text and require special-casing in
/// existing code paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebateRole {
    /// Synthesis moderator — reads all round reports and produces the final verdict.
    Moderator,
}

impl DebateRole {    /// Returns the opencode agent name for this role.
    pub fn agent_name(self) -> &'static str {
        match self {
            Self::Moderator => "arch-moderator",
        }
    }

    /// Returns a short human-readable description of this role.
    pub fn description(self) -> &'static str {
        match self {
            Self::Moderator => {
                "Synthesis moderator — weighs all architect reports and renders a final verdict"
            }
        }
    }

    /// Returns the default LLM model for this role.
    ///
    /// The moderator synthesises up to eight reports (~40k input tokens) and
    /// must reason about contested claims carefully — Opus is the right choice.
    pub fn default_model(self) -> &'static str {
        match self {
            Self::Moderator => "github-copilot/claude-opus-4.6",
        }
    }

    /// Returns the embedded system prompt for this role.
    pub fn prompt(self) -> &'static str {
        match self {
            Self::Moderator => MODERATOR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_prompts_are_non_empty() {
        for architect in ArchitectType::all() {
            assert!(
                !architect.prompt().is_empty(),
                "{} prompt must not be empty",
                architect.agent_name()
            );
        }
    }

    #[test]
    fn principal_prompt_contains_expected_keyword() {
        assert!(
            ArchitectType::Principal
                .prompt()
                .contains("principal software architect")
        );
    }

    #[test]
    fn design_prompt_contains_expected_keyword() {
        assert!(
            ArchitectType::Design
                .prompt()
                .contains("architecture review board")
        );
    }

    #[test]
    fn complexity_prompt_contains_expected_keyword() {
        assert!(ArchitectType::Complexity.prompt().contains("simplicity"));
    }

    #[test]
    fn security_prompt_contains_expected_keyword() {
        assert!(ArchitectType::Security.prompt().contains("security"));
    }

    #[test]
    fn agent_names_have_arch_prefix() {
        for architect in ArchitectType::all() {
            assert!(
                architect.agent_name().starts_with("arch-"),
                "agent name '{}' must start with 'arch-'",
                architect.agent_name()
            );
        }
    }

    #[test]
    fn descriptions_are_non_empty() {
        for architect in ArchitectType::all() {
            assert!(
                !architect.description().is_empty(),
                "{} description must not be empty",
                architect.agent_name()
            );
        }
    }

    #[test]
    fn all_returns_four_variants() {
        assert_eq!(ArchitectType::all().len(), 4);
    }

    #[test]
    fn default_models_are_non_empty() {
        for architect in ArchitectType::all() {
            assert!(
                !architect.default_model().is_empty(),
                "{} default_model must not be empty",
                architect.agent_name()
            );
        }
    }

    #[test]
    fn principal_and_design_use_opus() {
        assert_eq!(
            ArchitectType::Principal.default_model(),
            "github-copilot/claude-opus-4.6"
        );
        assert_eq!(
            ArchitectType::Design.default_model(),
            "github-copilot/claude-opus-4.6"
        );
    }

    #[test]
    fn complexity_and_security_use_sonnet() {
        assert_eq!(
            ArchitectType::Complexity.default_model(),
            "github-copilot/claude-sonnet-4.6"
        );
        assert_eq!(
            ArchitectType::Security.default_model(),
            "github-copilot/claude-sonnet-4.6"
        );
    }

    // ── DebateRole ────────────────────────────────────────────────────────────

    #[test]
    fn moderator_agent_name_is_arch_moderator() {
        assert_eq!(DebateRole::Moderator.agent_name(), "arch-moderator");
    }

    #[test]
    fn moderator_description_is_non_empty() {
        assert!(!DebateRole::Moderator.description().is_empty());
    }

    #[test]
    fn moderator_default_model_is_opus() {
        assert_eq!(
            DebateRole::Moderator.default_model(),
            "github-copilot/claude-opus-4.6"
        );
    }

    #[test]
    fn moderator_prompt_is_non_empty() {
        assert!(!DebateRole::Moderator.prompt().is_empty());
    }

    #[test]
    fn moderator_prompt_contains_moderator_keyword() {
        assert!(
            DebateRole::Moderator.prompt().contains("moderator")
                || DebateRole::Moderator.prompt().contains("synthesis")
                || DebateRole::Moderator.prompt().contains("synthesize")
                || DebateRole::Moderator.prompt().contains("synthesise"),
            "moderator prompt must mention synthesis or moderator"
        );
    }
}
