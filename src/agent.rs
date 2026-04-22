use crate::prompts::ArchitectType;

/// Generates the full content of an opencode agent `.md` file for the given
/// architect persona.
///
/// The file consists of YAML frontmatter followed by the embedded system
/// prompt. The `full_permissions` flag controls whether the agent is
/// restricted to read-only operations (default) or has full access.
/// The `model` string is written verbatim to the `model:` frontmatter field.
pub fn generate_agent_content(
    architect: ArchitectType,
    full_permissions: bool,
    model: &str,
) -> String {
    let frontmatter = if full_permissions {
        full_frontmatter(architect, model)
    } else {
        readonly_frontmatter(architect, model)
    };

    format!("{}\n{}", frontmatter, architect.prompt())
}

fn readonly_frontmatter(architect: ArchitectType, model: &str) -> String {
    format!(
        "---\n\
         description: {}\n\
         mode: primary\n\
         model: {}\n\
         temperature: 0.3\n\
         permission:\n\
         \x20 edit: deny\n\
         \x20 write: deny\n\
         \x20 bash:\n\
         \x20   \"*\": deny\n\
         \x20   \"git log*\": allow\n\
         \x20   \"git diff*\": allow\n\
         \x20   \"git status\": allow\n\
         \x20 webfetch: ask\n\
         ---",
        architect.description(),
        model
    )
}

fn full_frontmatter(architect: ArchitectType, model: &str) -> String {
    format!(
        "---\n\
         description: {}\n\
         mode: primary\n\
         model: {}\n\
         temperature: 0.3\n\
         permission:\n\
         \x20 edit: allow\n\
         \x20 write: allow\n\
         \x20 bash:\n\
         \x20   \"*\": allow\n\
         \x20 webfetch: allow\n\
         ---",
        architect.description(),
        model
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_model(architect: ArchitectType) -> &'static str {
        architect.default_model()
    }

    #[test]
    fn readonly_content_starts_with_frontmatter_delimiter() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            false,
            default_model(ArchitectType::Principal),
        );
        assert!(content.starts_with("---\n"));
    }

    #[test]
    fn readonly_content_contains_closing_delimiter() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            false,
            default_model(ArchitectType::Principal),
        );
        // There must be a closing --- after the opening one
        let after_first = &content[4..];
        assert!(after_first.contains("---"));
    }

    #[test]
    fn readonly_content_denies_edit() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            false,
            default_model(ArchitectType::Principal),
        );
        assert!(content.contains("edit: deny"));
    }

    #[test]
    fn readonly_content_denies_write() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            false,
            default_model(ArchitectType::Principal),
        );
        assert!(content.contains("write: deny"));
    }

    #[test]
    fn readonly_content_allows_git_log() {
        let content = generate_agent_content(
            ArchitectType::Security,
            false,
            default_model(ArchitectType::Security),
        );
        assert!(content.contains("\"git log*\": allow"));
    }

    #[test]
    fn full_content_allows_edit() {
        let content = generate_agent_content(
            ArchitectType::Design,
            true,
            default_model(ArchitectType::Design),
        );
        assert!(content.contains("edit: allow"));
    }

    #[test]
    fn full_content_allows_bash_wildcard() {
        let content = generate_agent_content(
            ArchitectType::Design,
            true,
            default_model(ArchitectType::Design),
        );
        assert!(content.contains("\"*\": allow"));
    }

    #[test]
    fn content_contains_prompt_body() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            false,
            default_model(ArchitectType::Principal),
        );
        // The prompt body must appear after the closing ---
        let closing = content.find("---\n").unwrap();
        let after_opening = &content[closing + 4..];
        let second_closing = after_opening.find("---").unwrap();
        let body = &after_opening[second_closing + 3..];
        assert!(!body.trim().is_empty());
    }

    #[test]
    fn description_appears_in_frontmatter() {
        let content = generate_agent_content(
            ArchitectType::Complexity,
            false,
            default_model(ArchitectType::Complexity),
        );
        assert!(content.contains(ArchitectType::Complexity.description()));
    }

    #[test]
    fn all_architects_produce_non_empty_content() {
        for architect in ArchitectType::all() {
            let content = generate_agent_content(*architect, false, architect.default_model());
            assert!(!content.is_empty());
        }
    }

    #[test]
    fn model_line_appears_in_readonly_frontmatter() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            false,
            "github-copilot/claude-opus-4.6",
        );
        assert!(content.contains("model: github-copilot/claude-opus-4.6"));
    }

    #[test]
    fn model_line_appears_in_full_frontmatter() {
        let content = generate_agent_content(
            ArchitectType::Design,
            true,
            "github-copilot/claude-opus-4.6",
        );
        assert!(content.contains("model: github-copilot/claude-opus-4.6"));
    }

    #[test]
    fn custom_model_override_appears_verbatim() {
        let content = generate_agent_content(ArchitectType::Security, false, "openai/gpt-5");
        assert!(content.contains("model: openai/gpt-5"));
    }
}

