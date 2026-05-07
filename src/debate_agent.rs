// Phase 1: foundation types and generation functions for the debate pipeline.

use crate::prompts::{ArchitectType, DebateRole};

const ROUND2_CHALLENGE: &str = include_str!("../prompts/debate/round2_challenge.md");
const ROUND2_DEVILS_ADVOCATE: &str =
    include_str!("../prompts/debate/round2_devils_advocate.md");

/// Which round of the debate this agent participates in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebateRound {
    /// First round: initial assessment, no peer context.
    Round1,
    /// Second round: challenge/endorse peer findings, informed by peer Round 1 reports.
    Round2,
}

/// A single peer architect's report for use as debate context.
#[derive(Debug, Clone, Copy)]
pub struct PeerReport<'a> {
    /// The opencode agent name of the architect that produced this report
    /// (e.g., `"arch-principal"`).
    pub agent_name: &'a str,
    /// The full text content of the report.
    pub content: &'a str,
}

/// Context injected into a debate agent for a given round.
///
/// - Round 1: `own_report` is `None`, `peer_reports` is empty, and
///   `is_devils_advocate` is `false` — no context is available yet.
/// - Round 2 standard: `own_report` is the agent's own Round 1 output;
///   `peer_reports` contains the three peer Round 1 reports.
/// - Round 2 devil's advocate: same as Round 2 standard but
///   `is_devils_advocate` is `true`, which selects the adversarial
///   challenge template instead of the balanced one.
#[derive(Debug)]
pub struct DebateContext<'a> {
    /// The round this context is for.
    pub round: DebateRound,
    /// This agent's own report from the previous round (`None` in Round 1).
    pub own_report: Option<&'a str>,
    /// Peer agents' reports from the previous round (empty in Round 1).
    pub peer_reports: Vec<PeerReport<'a>>,
    /// When `true` the agent uses the devil's advocate challenge template
    /// instead of the standard balanced-challenge template.
    /// Only meaningful in Round 2; ignored in Round 1.
    pub is_devils_advocate: bool,
}

/// Generates the full content of an opencode agent `.md` file for a debate
/// round.
///
/// - **Round 1**: produces system-prompt + output instruction only (no peer
///   context).
/// - **Round 2**: produces system-prompt + the Round 2 challenge template with
///   `own_report` and `peer_reports` injected, + output instruction.
///
/// The generated file restricts writes to the expected output path for that
/// round: `reviews/round1/arch-<name>.md` or `reviews/round2/arch-<name>.md`.
pub fn generate_debate_agent(
    architect: ArchitectType,
    context: &DebateContext<'_>,
    model: &str,
) -> String {
    match context.round {
        DebateRound::Round1 => generate_round1_agent(architect, model),
        DebateRound::Round2 => generate_round2_agent(architect, context, model),
    }
}

/// Generates the full content of an opencode agent `.md` file for the
/// moderator (synthesis) step.
///
/// All eight debate reports (4× Round 1 + 4× Round 2) are injected inline
/// into the agent body so that no external fetching is required.
///
/// If `devils_advocate` is `Some(agent_name)`, a notice is prepended to the
/// reports section so the moderator can weight that agent's Round 2 challenges
/// appropriately.
pub fn generate_moderator_agent(
    all_reports: &[PeerReport<'_>],
    model: &str,
    devils_advocate: Option<&str>,
) -> String {
    let role = DebateRole::Moderator;
    let da_notice = match devils_advocate {
        Some(name) => format!(
            "## Devil's Advocate Notice\n\n\
             `{name}` was designated as the devil's advocate for Round 2. \
             Its Round 2 report contains adversarial challenges to the consensus — \
             not necessarily its genuine position. \
             Treat its Round 2 challenges as stress-tests rather than sincere disagreements \
             when weighting the panel's overall conclusions.\n\n"
        ),
        None => String::new(),
    };
    format!(
        "{}\n{}\n\n{}{}\n\n## Output Instructions\n\n\
         Save your synthesis report to `reviews/final-report.md`. \
         The directory already exists. \
         Use the write tool to create the file.",
        moderator_frontmatter(role, model),
        role.prompt(),
        da_notice,
        render_all_reports(all_reports),
    )
}

// ── round generators ──────────────────────────────────────────────────────────

fn generate_round1_agent(architect: ArchitectType, model: &str) -> String {
    let persona = architect.agent_name().trim_start_matches("arch-");
    format!(
        "{}\n{}\n\n## Output Instructions\n\n\
         When you have completed your review, save your findings to \
         `reviews/round1/arch-{}.md`. \
         The directory will be created before you run. \
         Use the write tool to create the file.",
        round1_frontmatter(architect, model),
        architect.prompt(),
        persona,
    )
}

fn generate_round2_agent(
    architect: ArchitectType,
    context: &DebateContext<'_>,
    model: &str,
) -> String {
    let persona = architect.agent_name().trim_start_matches("arch-");
    let own = context.own_report.unwrap_or("*(not available)*");
    let peer_block = render_peer_reports(&context.peer_reports);

    // Select the appropriate Round 2 template.
    let template = if context.is_devils_advocate {
        ROUND2_DEVILS_ADVOCATE
    } else {
        ROUND2_CHALLENGE
    };

    // Substitute the {own_report} and {peer_reports} placeholders.
    let challenge_body = template
        .replace("{own_report}", own)
        .replace("{peer_reports}", &peer_block);

    format!(
        "{}\n{}\n\n{}\n\n## Output Instructions\n\n\
         When you have completed your review, save your findings to \
         `reviews/round2/arch-{}.md`. \
         The directory will be created before you run. \
         Use the write tool to create the file.",
        round2_frontmatter(architect, model),
        architect.prompt(),
        challenge_body,
        persona,
    )
}

// ── frontmatter builders ──────────────────────────────────────────────────────

fn round1_frontmatter(architect: ArchitectType, model: &str) -> String {
    format!(
        "---\n\
         description: {}\n\
         mode: primary\n\
         model: {}\n\
         temperature: 0.3\n\
         permission:\n\
         \x20 edit:\n\
         \x20   \"*\": deny\n\
         \x20   \"reviews/round1/arch-*.md\": allow\n\
         \x20 write:\n\
         \x20   \"*\": deny\n\
         \x20   \"reviews/round1/arch-*.md\": allow\n\
         \x20 bash:\n\
         \x20   \"*\": deny\n\
         \x20   \"git log*\": allow\n\
         \x20   \"git diff*\": allow\n\
         \x20   \"git status\": allow\n\
         \x20 webfetch: ask\n\
         ---",
        architect.description(),
        model,
    )
}

fn round2_frontmatter(architect: ArchitectType, model: &str) -> String {
    format!(
        "---\n\
         description: {}\n\
         mode: primary\n\
         model: {}\n\
         temperature: 0.3\n\
         permission:\n\
         \x20 edit:\n\
         \x20   \"*\": deny\n\
         \x20   \"reviews/round2/arch-*.md\": allow\n\
         \x20 write:\n\
         \x20   \"*\": deny\n\
         \x20   \"reviews/round2/arch-*.md\": allow\n\
         \x20 bash:\n\
         \x20   \"*\": deny\n\
         \x20   \"git log*\": allow\n\
         \x20   \"git diff*\": allow\n\
         \x20   \"git status\": allow\n\
         \x20 webfetch: deny\n\
         ---",
        architect.description(),
        model,
    )
}

fn moderator_frontmatter(role: DebateRole, model: &str) -> String {
    format!(
        "---\n\
         description: {}\n\
         mode: primary\n\
         model: {}\n\
         temperature: 0.3\n\
         permission:\n\
         \x20 edit:\n\
         \x20   \"*\": deny\n\
         \x20   \"reviews/final-report.md\": allow\n\
         \x20 write:\n\
         \x20   \"*\": deny\n\
         \x20   \"reviews/final-report.md\": allow\n\
         \x20 bash:\n\
         \x20   \"*\": deny\n\
         \x20 webfetch: deny\n\
         ---",
        role.description(),
        model,
    )
}

// ── context rendering helpers ─────────────────────────────────────────────────

fn render_peer_reports(peers: &[PeerReport<'_>]) -> String {
    if peers.is_empty() {
        return String::from("*(no peer reports available)*");
    }
    peers
        .iter()
        .map(|p| format!("### {}\n\n{}", p.agent_name, p.content))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

fn render_all_reports(reports: &[PeerReport<'_>]) -> String {
    if reports.is_empty() {
        return String::from("*(no reports available)*");
    }
    let sections = reports
        .iter()
        .map(|r| format!("## Report: {}\n\n{}", r.agent_name, r.content))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");
    format!("## All Architect Reports\n\n{sections}")
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ───────────────────────────────────────────────────────────────

    fn round1_ctx<'a>() -> DebateContext<'a> {
        DebateContext {
            round: DebateRound::Round1,
            own_report: None,
            peer_reports: vec![],
            is_devils_advocate: false,
        }
    }

    fn round2_ctx<'a>(
        own: &'a str,
        peers: Vec<PeerReport<'a>>,
    ) -> DebateContext<'a> {
        DebateContext {
            round: DebateRound::Round2,
            own_report: Some(own),
            peer_reports: peers,
            is_devils_advocate: false,
        }
    }

    fn default_model(a: ArchitectType) -> &'static str {
        a.default_model()
    }

    // ── DebateRound ───────────────────────────────────────────────────────────

    #[test]
    fn debate_round_variants_are_distinct() {
        assert_ne!(DebateRound::Round1, DebateRound::Round2);
    }

    // ── generate_debate_agent — Round 1 ──────────────────────────────────────

    #[test]
    fn round1_content_starts_with_frontmatter_delimiter() {
        let ctx = round1_ctx();
        let content = generate_debate_agent(
            ArchitectType::Principal,
            &ctx,
            default_model(ArchitectType::Principal),
        );
        assert!(content.starts_with("---\n"));
    }

    #[test]
    fn round1_content_allows_round1_glob() {
        let ctx = round1_ctx();
        let content = generate_debate_agent(
            ArchitectType::Principal,
            &ctx,
            default_model(ArchitectType::Principal),
        );
        assert!(
            content.contains("\"reviews/round1/arch-*.md\": allow"),
            "round 1 agent must allow writes to reviews/round1/"
        );
    }

    #[test]
    fn round1_content_denies_round2_glob() {
        let ctx = round1_ctx();
        let content = generate_debate_agent(
            ArchitectType::Principal,
            &ctx,
            default_model(ArchitectType::Principal),
        );
        assert!(
            !content.contains("reviews/round2"),
            "round 1 agent must not reference reviews/round2/"
        );
    }

    #[test]
    fn round1_content_has_webfetch_ask() {
        let ctx = round1_ctx();
        let content = generate_debate_agent(
            ArchitectType::Security,
            &ctx,
            default_model(ArchitectType::Security),
        );
        assert!(
            content.contains("webfetch: ask"),
            "round 1 agent must set webfetch: ask"
        );
    }

    #[test]
    fn round1_content_contains_output_instruction() {
        let ctx = round1_ctx();
        let content = generate_debate_agent(
            ArchitectType::Principal,
            &ctx,
            default_model(ArchitectType::Principal),
        );
        assert!(
            content.contains("reviews/round1/arch-principal.md"),
            "round 1 agent must reference its expected output path"
        );
    }

    #[test]
    fn round1_content_contains_system_prompt() {
        let ctx = round1_ctx();
        let content = generate_debate_agent(
            ArchitectType::Principal,
            &ctx,
            default_model(ArchitectType::Principal),
        );
        // The system prompt must appear in the body (after the closing ---).
        let after_fm = content.split("---\n").nth(2).unwrap_or("");
        assert!(!after_fm.trim().is_empty(), "system prompt body must be present");
    }

    #[test]
    fn round1_content_contains_correct_model() {
        let ctx = round1_ctx();
        let model = "openai/gpt-5";
        let content = generate_debate_agent(ArchitectType::Design, &ctx, model);
        assert!(content.contains("model: openai/gpt-5"));
    }

    #[test]
    fn round1_all_architects_produce_non_empty_content() {
        for architect in ArchitectType::all() {
            let ctx = round1_ctx();
            let content = generate_debate_agent(*architect, &ctx, architect.default_model());
            assert!(
                !content.is_empty(),
                "{} must produce non-empty Round 1 content",
                architect.agent_name()
            );
        }
    }

    #[test]
    fn round1_content_denies_wildcard_edit() {
        let ctx = round1_ctx();
        let content = generate_debate_agent(
            ArchitectType::Complexity,
            &ctx,
            default_model(ArchitectType::Complexity),
        );
        assert!(content.contains("\"*\": deny"));
    }

    // ── generate_debate_agent — Round 2 ──────────────────────────────────────

    #[test]
    fn round2_content_allows_round2_glob() {
        let ctx = round2_ctx("my round 1 report", vec![]);
        let content = generate_debate_agent(
            ArchitectType::Principal,
            &ctx,
            default_model(ArchitectType::Principal),
        );
        assert!(
            content.contains("\"reviews/round2/arch-*.md\": allow"),
            "round 2 agent must allow writes to reviews/round2/"
        );
    }

    #[test]
    fn round2_content_denies_round1_glob() {
        let ctx = round2_ctx("my round 1 report", vec![]);
        let content = generate_debate_agent(
            ArchitectType::Principal,
            &ctx,
            default_model(ArchitectType::Principal),
        );
        assert!(
            !content.contains("reviews/round1"),
            "round 2 agent must not reference reviews/round1/"
        );
    }

    #[test]
    fn round2_content_has_webfetch_deny() {
        let ctx = round2_ctx("my round 1 report", vec![]);
        let content = generate_debate_agent(
            ArchitectType::Security,
            &ctx,
            default_model(ArchitectType::Security),
        );
        assert!(
            content.contains("webfetch: deny"),
            "round 2 agent must set webfetch: deny"
        );
    }

    #[test]
    fn round2_content_injects_own_report() {
        let own = "My own round 1 findings about scalability.";
        let ctx = round2_ctx(own, vec![]);
        let content = generate_debate_agent(
            ArchitectType::Principal,
            &ctx,
            default_model(ArchitectType::Principal),
        );
        assert!(
            content.contains(own),
            "round 2 agent must inject the own_report"
        );
    }

    #[test]
    fn round2_content_injects_peer_reports() {
        let peer_content = "Peer complexity report content.";
        let peers = vec![PeerReport {
            agent_name: "arch-complexity",
            content: peer_content,
        }];
        let ctx = round2_ctx("my report", peers);
        let content = generate_debate_agent(
            ArchitectType::Principal,
            &ctx,
            default_model(ArchitectType::Principal),
        );
        assert!(
            content.contains("arch-complexity"),
            "round 2 agent must include peer agent name"
        );
        assert!(
            content.contains(peer_content),
            "round 2 agent must include peer report content"
        );
    }

    #[test]
    fn round2_content_contains_output_instruction() {
        let ctx = round2_ctx("my report", vec![]);
        let content = generate_debate_agent(
            ArchitectType::Security,
            &ctx,
            default_model(ArchitectType::Security),
        );
        assert!(
            content.contains("reviews/round2/arch-security.md"),
            "round 2 agent must reference its expected output path"
        );
    }

    #[test]
    fn round2_missing_own_report_uses_fallback() {
        let ctx = DebateContext {
            round: DebateRound::Round2,
            own_report: None,
            peer_reports: vec![],
            is_devils_advocate: false,
        };
        let content = generate_debate_agent(
            ArchitectType::Design,
            &ctx,
            default_model(ArchitectType::Design),
        );
        assert!(
            content.contains("*(not available)*"),
            "missing own_report must use the not-available fallback"
        );
    }

    #[test]
    fn round2_multiple_peers_all_injected() {
        let peers = vec![
            PeerReport {
                agent_name: "arch-complexity",
                content: "complexity analysis",
            },
            PeerReport {
                agent_name: "arch-security",
                content: "security analysis",
            },
            PeerReport {
                agent_name: "arch-design",
                content: "design analysis",
            },
        ];
        let ctx = round2_ctx("principal r1", peers);
        let content = generate_debate_agent(
            ArchitectType::Principal,
            &ctx,
            default_model(ArchitectType::Principal),
        );
        assert!(content.contains("arch-complexity"));
        assert!(content.contains("arch-security"));
        assert!(content.contains("arch-design"));
        assert!(content.contains("complexity analysis"));
        assert!(content.contains("security analysis"));
        assert!(content.contains("design analysis"));
    }

    // ── generate_moderator_agent ──────────────────────────────────────────────

    #[test]
    fn moderator_content_starts_with_frontmatter_delimiter() {
        let reports = vec![PeerReport {
            agent_name: "arch-principal",
            content: "principal report",
        }];
        let content =
            generate_moderator_agent(&reports, DebateRole::Moderator.default_model(), None);
        assert!(content.starts_with("---\n"));
    }

    #[test]
    fn moderator_content_allows_final_report_glob() {
        let reports: Vec<PeerReport<'_>> = vec![];
        let content =
            generate_moderator_agent(&reports, DebateRole::Moderator.default_model(), None);
        assert!(
            content.contains("\"reviews/final-report.md\": allow"),
            "moderator agent must allow writes to reviews/final-report.md"
        );
    }

    #[test]
    fn moderator_content_has_webfetch_deny() {
        let reports: Vec<PeerReport<'_>> = vec![];
        let content =
            generate_moderator_agent(&reports, DebateRole::Moderator.default_model(), None);
        assert!(
            content.contains("webfetch: deny"),
            "moderator agent must set webfetch: deny"
        );
    }

    #[test]
    fn moderator_content_injects_all_reports() {
        let reports = vec![
            PeerReport {
                agent_name: "arch-principal",
                content: "principal round1",
            },
            PeerReport {
                agent_name: "arch-complexity",
                content: "complexity round1",
            },
            PeerReport {
                agent_name: "arch-principal-r2",
                content: "principal round2",
            },
            PeerReport {
                agent_name: "arch-complexity-r2",
                content: "complexity round2",
            },
        ];
        let content =
            generate_moderator_agent(&reports, DebateRole::Moderator.default_model(), None);
        for r in &reports {
            assert!(
                content.contains(r.agent_name),
                "moderator must include agent name: {}",
                r.agent_name
            );
            assert!(
                content.contains(r.content),
                "moderator must include content for: {}",
                r.agent_name
            );
        }
    }

    #[test]
    fn moderator_content_contains_output_instruction() {
        let reports: Vec<PeerReport<'_>> = vec![];
        let content =
            generate_moderator_agent(&reports, DebateRole::Moderator.default_model(), None);
        assert!(
            content.contains("reviews/final-report.md"),
            "moderator agent must reference its expected output path"
        );
    }

    #[test]
    fn moderator_content_contains_system_prompt() {
        let reports: Vec<PeerReport<'_>> = vec![];
        let content =
            generate_moderator_agent(&reports, DebateRole::Moderator.default_model(), None);
        // The moderator system prompt body must be present.
        assert!(
            !content.is_empty(),
            "moderator agent content must not be empty"
        );
        assert!(
            content.contains("synthesis") || content.contains("moderator"),
            "moderator agent must contain synthesis/moderator prompt"
        );
    }

    #[test]
    fn moderator_content_contains_correct_model() {
        let reports: Vec<PeerReport<'_>> = vec![];
        let model = "github-copilot/claude-opus-4.6";
        let content = generate_moderator_agent(&reports, model, None);
        assert!(content.contains("model: github-copilot/claude-opus-4.6"));
    }

    #[test]
    fn moderator_content_denies_bash_wildcard() {
        let reports: Vec<PeerReport<'_>> = vec![];
        let content =
            generate_moderator_agent(&reports, DebateRole::Moderator.default_model(), None);
        // Moderator has no bash allow rules — only the wildcard deny.
        assert!(content.contains("bash:"));
        assert!(content.contains("\"*\": deny"));
    }

    // ── render helpers ────────────────────────────────────────────────────────

    #[test]
    fn render_peer_reports_empty_returns_fallback() {
        let result = render_peer_reports(&[]);
        assert!(result.contains("no peer reports available"));
    }

    #[test]
    fn render_peer_reports_single_contains_agent_name_and_content() {
        let peers = vec![PeerReport {
            agent_name: "arch-security",
            content: "trust boundary analysis",
        }];
        let result = render_peer_reports(&peers);
        assert!(result.contains("arch-security"));
        assert!(result.contains("trust boundary analysis"));
    }

    #[test]
    fn render_all_reports_empty_returns_fallback() {
        let result = render_all_reports(&[]);
        assert!(result.contains("no reports available"));
    }

    #[test]
    fn render_all_reports_contains_section_header() {
        let reports = vec![PeerReport {
            agent_name: "arch-design",
            content: "design verdict",
        }];
        let result = render_all_reports(&reports);
        assert!(result.contains("## All Architect Reports"));
        assert!(result.contains("arch-design"));
        assert!(result.contains("design verdict"));
    }

    // ── devil's advocate (Round 2) ────────────────────────────────────────────

    fn round2_da_ctx<'a>(own: &'a str, peers: Vec<PeerReport<'a>>) -> DebateContext<'a> {
        DebateContext {
            round: DebateRound::Round2,
            own_report: Some(own),
            peer_reports: peers,
            is_devils_advocate: true,
        }
    }

    #[test]
    fn round2_da_selects_devils_advocate_template() {
        let ctx = round2_da_ctx("my r1 report", vec![]);
        let content = generate_debate_agent(
            ArchitectType::Complexity,
            &ctx,
            default_model(ArchitectType::Complexity),
        );
        // The DA template contains unique wording not in the standard template.
        assert!(
            content.contains("devil's advocate"),
            "DA agent must reference the devil's advocate role"
        );
    }

    #[test]
    fn round2_standard_does_not_contain_devils_advocate_wording() {
        let ctx = round2_ctx("my r1 report", vec![]);
        let content = generate_debate_agent(
            ArchitectType::Complexity,
            &ctx,
            default_model(ArchitectType::Complexity),
        );
        assert!(
            !content.contains("devil's advocate"),
            "Standard Round 2 agent must NOT contain DA wording"
        );
    }

    #[test]
    fn round2_da_injects_own_report() {
        let own = "My own DA round 1 findings.";
        let ctx = round2_da_ctx(own, vec![]);
        let content = generate_debate_agent(
            ArchitectType::Security,
            &ctx,
            default_model(ArchitectType::Security),
        );
        assert!(content.contains(own));
    }

    #[test]
    fn round2_da_injects_peer_reports() {
        let peer_content = "Design peer r1 content.";
        let peers = vec![PeerReport {
            agent_name: "arch-design",
            content: peer_content,
        }];
        let ctx = round2_da_ctx("own r1", peers);
        let content = generate_debate_agent(
            ArchitectType::Principal,
            &ctx,
            default_model(ArchitectType::Principal),
        );
        assert!(content.contains("arch-design"));
        assert!(content.contains(peer_content));
    }

    #[test]
    fn round2_da_output_path_same_as_standard_round2() {
        let ctx = round2_da_ctx("own r1", vec![]);
        let content = generate_debate_agent(
            ArchitectType::Complexity,
            &ctx,
            default_model(ArchitectType::Complexity),
        );
        assert!(
            content.contains("reviews/round2/arch-complexity.md"),
            "DA agent must still write to round2 output path"
        );
    }

    // ── generate_moderator_agent — DA notice ──────────────────────────────────

    #[test]
    fn moderator_with_da_contains_da_notice() {
        let reports = vec![PeerReport {
            agent_name: "arch-complexity",
            content: "complexity r1",
        }];
        let content =
            generate_moderator_agent(&reports, DebateRole::Moderator.default_model(), Some("arch-complexity"));
        assert!(
            content.contains("Devil's Advocate Notice"),
            "moderator content must include DA notice when DA is set"
        );
        assert!(
            content.contains("arch-complexity"),
            "moderator DA notice must name the designated advocate"
        );
    }

    #[test]
    fn moderator_without_da_has_no_notice() {
        let reports = vec![PeerReport {
            agent_name: "arch-complexity",
            content: "complexity r1",
        }];
        let content =
            generate_moderator_agent(&reports, DebateRole::Moderator.default_model(), None);
        assert!(
            !content.contains("Devil's Advocate Notice"),
            "moderator without DA must not contain a DA notice"
        );
    }

    #[test]
    fn moderator_da_notice_appears_before_reports_section() {
        let reports = vec![PeerReport {
            agent_name: "arch-principal",
            content: "principal r1",
        }];
        let content =
            generate_moderator_agent(&reports, DebateRole::Moderator.default_model(), Some("arch-principal"));
        let notice_pos = content.find("Devil's Advocate Notice").unwrap();
        let reports_pos = content.find("## All Architect Reports").unwrap();
        assert!(
            notice_pos < reports_pos,
            "DA notice must appear before the reports section"
        );
    }
}
