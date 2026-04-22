use clap::ValueEnum;

const PRINCIPAL: &str = include_str!("../prompts/system/principal.md");
const DESIGN: &str = include_str!("../prompts/system/design.md");
const COMPLEXITY: &str = include_str!("../prompts/system/complexity.md");
const SECURITY: &str = include_str!("../prompts/system/security.md");

/// The four available architect personas, embedded at compile time.
#[derive(Debug, Clone, Copy, ValueEnum)]
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
}
