use crate::prompts::ArchitectType;

/// Controls the permission model written to the agent frontmatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionMode {
    /// Deny all edits, restrict bash to read-only git commands (default).
    ReadOnly,
    /// Allow all edits and bash commands.
    Full,
    /// Read-only for all repo files, but allow writing to `reviews/arch-*.md`.
    Review,
}

/// Generates the full content of an opencode agent `.md` file for the given
/// architect persona.
///
/// The file consists of YAML frontmatter followed by the embedded system
/// prompt. `mode` controls the permission model written to the frontmatter.
/// The `model` string is written verbatim to the `model:` frontmatter field.
///
/// In `Review` mode a review-output instruction is appended after the prompt
/// body, directing the persona to save its findings to `reviews/arch-<name>-<date>.md`.
pub fn generate_agent_content(
    architect: ArchitectType,
    mode: PermissionMode,
    model: &str,
) -> String {
    match mode {
        PermissionMode::ReadOnly => {
            format!(
                "{}\n{}",
                readonly_frontmatter(architect, model),
                architect.prompt()
            )
        }
        PermissionMode::Full => {
            format!(
                "{}\n{}",
                full_frontmatter(architect, model),
                architect.prompt()
            )
        }
        PermissionMode::Review => {
            let persona = architect.agent_name().trim_start_matches("arch-");
            format!(
                "{}\n{}\n\n## Review Output\n\n\
                 When you have completed your review, save your findings to \
                 `reviews/arch-{}-YYYY-MM-DD.md` (use today's date). \
                 The `reviews/` directory already exists. \
                 Use the write tool to create the file.",
                review_frontmatter(architect, model),
                architect.prompt(),
                persona
            )
        }
    }
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

fn review_frontmatter(architect: ArchitectType, model: &str) -> String {
    format!(
        "---\n\
         description: {}\n\
         mode: primary\n\
         model: {}\n\
         temperature: 0.3\n\
         permission:\n\
         \x20 edit:\n\
         \x20   \"*\": deny\n\
         \x20   \"reviews/arch-*.md\": allow\n\
         \x20 write:\n\
         \x20   \"*\": deny\n\
         \x20   \"reviews/arch-*.md\": allow\n\
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ───────────────────────────────────────────────────────────────

    fn dm(architect: ArchitectType) -> &'static str {
        architect.default_model()
    }

    // ── readonly mode ─────────────────────────────────────────────────────────

    #[test]
    fn readonly_content_starts_with_frontmatter_delimiter() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            PermissionMode::ReadOnly,
            dm(ArchitectType::Principal),
        );
        assert!(content.starts_with("---\n"));
    }

    #[test]
    fn readonly_content_contains_closing_delimiter() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            PermissionMode::ReadOnly,
            dm(ArchitectType::Principal),
        );
        let after_first = &content[4..];
        assert!(after_first.contains("---"));
    }

    #[test]
    fn readonly_content_denies_edit() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            PermissionMode::ReadOnly,
            dm(ArchitectType::Principal),
        );
        assert!(content.contains("edit: deny"));
    }

    #[test]
    fn readonly_content_denies_write() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            PermissionMode::ReadOnly,
            dm(ArchitectType::Principal),
        );
        assert!(content.contains("write: deny"));
    }

    #[test]
    fn readonly_content_allows_git_log() {
        let content = generate_agent_content(
            ArchitectType::Security,
            PermissionMode::ReadOnly,
            dm(ArchitectType::Security),
        );
        assert!(content.contains("\"git log*\": allow"));
    }

    // ── full mode ─────────────────────────────────────────────────────────────

    #[test]
    fn full_content_allows_edit() {
        let content = generate_agent_content(
            ArchitectType::Design,
            PermissionMode::Full,
            dm(ArchitectType::Design),
        );
        assert!(content.contains("edit: allow"));
    }

    #[test]
    fn full_content_allows_bash_wildcard() {
        let content = generate_agent_content(
            ArchitectType::Design,
            PermissionMode::Full,
            dm(ArchitectType::Design),
        );
        assert!(content.contains("\"*\": allow"));
    }

    // ── review mode ───────────────────────────────────────────────────────────

    #[test]
    fn review_content_denies_edit_wildcard() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            PermissionMode::Review,
            dm(ArchitectType::Principal),
        );
        assert!(
            content.contains("\"*\": deny"),
            "review mode must deny wildcard edits"
        );
    }

    #[test]
    fn review_content_allows_reviews_glob() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            PermissionMode::Review,
            dm(ArchitectType::Principal),
        );
        assert!(
            content.contains("\"reviews/arch-*.md\": allow"),
            "review mode must allow writes to reviews/arch-*.md"
        );
    }

    #[test]
    fn review_content_allows_git_log() {
        let content = generate_agent_content(
            ArchitectType::Security,
            PermissionMode::Review,
            dm(ArchitectType::Security),
        );
        assert!(content.contains("\"git log*\": allow"));
    }

    #[test]
    fn review_content_does_not_allow_full_edit() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            PermissionMode::Review,
            dm(ArchitectType::Principal),
        );
        assert!(
            !content.contains("edit: allow"),
            "review mode must not grant full edit permission"
        );
    }

    #[test]
    fn review_content_contains_review_output_instruction() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            PermissionMode::Review,
            dm(ArchitectType::Principal),
        );
        assert!(
            content.contains("## Review Output"),
            "review mode must append the review-output instruction"
        );
    }

    #[test]
    fn review_content_contains_persona_name_in_instruction() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            PermissionMode::Review,
            dm(ArchitectType::Principal),
        );
        assert!(
            content.contains("arch-principal"),
            "review-output instruction must reference the persona file name"
        );
    }

    // ── shared ────────────────────────────────────────────────────────────────

    #[test]
    fn content_contains_prompt_body() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            PermissionMode::ReadOnly,
            dm(ArchitectType::Principal),
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
            PermissionMode::ReadOnly,
            dm(ArchitectType::Complexity),
        );
        assert!(content.contains(ArchitectType::Complexity.description()));
    }

    #[test]
    fn all_architects_produce_non_empty_content() {
        for architect in ArchitectType::all() {
            let content = generate_agent_content(
                *architect,
                PermissionMode::ReadOnly,
                architect.default_model(),
            );
            assert!(!content.is_empty());
        }
    }

    #[test]
    fn model_line_appears_in_readonly_frontmatter() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            PermissionMode::ReadOnly,
            "github-copilot/claude-opus-4.6",
        );
        assert!(content.contains("model: github-copilot/claude-opus-4.6"));
    }

    #[test]
    fn model_line_appears_in_full_frontmatter() {
        let content = generate_agent_content(
            ArchitectType::Design,
            PermissionMode::Full,
            "github-copilot/claude-opus-4.6",
        );
        assert!(content.contains("model: github-copilot/claude-opus-4.6"));
    }

    #[test]
    fn model_line_appears_in_review_frontmatter() {
        let content = generate_agent_content(
            ArchitectType::Principal,
            PermissionMode::Review,
            "github-copilot/claude-opus-4.6",
        );
        assert!(content.contains("model: github-copilot/claude-opus-4.6"));
    }

    #[test]
    fn custom_model_override_appears_verbatim() {
        let content = generate_agent_content(
            ArchitectType::Security,
            PermissionMode::ReadOnly,
            "openai/gpt-5",
        );
        assert!(content.contains("model: openai/gpt-5"));
    }
}
