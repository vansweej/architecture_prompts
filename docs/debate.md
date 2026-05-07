# Debate Mode

`architecture_prompts` supports a **multi-round architect debate**: all four
architect personas review the codebase independently, challenge each other's
findings in a second round, and a synthesis moderator produces a final verdict.

The debate produces richer, more reliable architectural feedback than a single
persona run because it surfaces disagreements, prevents premature consensus, and
forces each persona to justify its claims against peer scrutiny.

---

## Concept

A standard single-agent review is a monologue: one persona, one pass, no
challenge. The debate is a structured protocol:

```
Round 1  →  four independent reviews, no peer context
Round 2  →  each agent reads the other three Round 1 reports and responds
Synthesis →  the moderator reads all eight reports and renders a final verdict
```

Each round is run by a freshly generated opencode agent file with permissions
scoped to its expected output path. No agent can read or modify another agent's
output file directly — all cross-agent context is injected into the agent body
by the orchestrator.

---

## Protocol

### Round 1 — Independent Assessment

Each of the four architect personas (`principal`, `design`, `complexity`,
`security`) is given its standard system prompt and told to write its findings
to `reviews/round1/arch-<name>.md`.

No peer context is injected. The goal is an unbiased first pass.

**Permission model:**
- Read-only for all repo files and the codebase
- Write allowed only to `reviews/round1/arch-*.md`
- Bash restricted to read-only `git` commands (`git log*`, `git diff*`, `git status`)
- `webfetch: ask` — agents may look up library documentation

### Round 2 — Peer Challenge

Each persona is given:
1. Its own Round 1 report
2. The three peer Round 1 reports
3. The Round 2 challenge instruction template

The challenge template instructs the agent to:
- Re-read its own findings critically
- Challenge peer claims it disagrees with (with reasons)
- Explicitly endorse peer claims it agrees with
- Surface new observations hinted at by the peer reports

Output goes to `reviews/round2/arch-<name>.md`.

**Permission model:**
- Write allowed only to `reviews/round2/arch-*.md`
- `webfetch: deny` — all context is already injected inline; external fetches would be noise

### Synthesis — Moderator Report

The moderator agent receives all eight reports (4 Round 1 + 4 Round 2) injected
inline and produces `reviews/final-report.md`.

The moderator:
- Identifies **Confirmed Findings** (three or more personas agree)
- Identifies **Contested Findings** (substantial disagreement)
- Lists **Unresolved Questions**
- Produces a **Risk Register** with severity ratings
- Recommends **Next Steps** in priority order

**Permission model:**
- Write allowed only to `reviews/final-report.md`
- No bash access
- `webfetch: deny`

---

## Output Structure

After a full debate run, the `reviews/` directory contains:

```
reviews/
├── round1/
│   ├── arch-principal.md
│   ├── arch-design.md
│   ├── arch-complexity.md
│   └── arch-security.md
├── round2/
│   ├── arch-principal.md
│   ├── arch-design.md
│   ├── arch-complexity.md
│   └── arch-security.md
└── final-report.md
```

The `final-report.md` is the primary deliverable. The per-round files are
retained for auditability and to allow manual inspection of how findings evolved
across rounds.

---

## Type Model

The debate pipeline is built on three core types in `src/debate_agent.rs`:

### `DebateRound`

```rust
pub enum DebateRound {
    Round1,
    Round2,
}
```

Controls which agent file variant `generate_debate_agent()` produces.

### `PeerReport<'a>`

```rust
pub struct PeerReport<'a> {
    pub agent_name: &'a str,  // e.g., "arch-principal"
    pub content: &'a str,     // full report text
}
```

A named report slice used for context injection. The lifetime ties the report
to the string data it borrows — no heap allocation for context that already
lives in memory.

### `DebateContext<'a>`

```rust
pub struct DebateContext<'a> {
    pub round: DebateRound,
    pub own_report: Option<&'a str>,
    pub peer_reports: Vec<PeerReport<'a>>,
}
```

| Round  | `own_report` | `peer_reports`         |
|--------|-------------|------------------------|
| Round1 | `None`      | empty                  |
| Round2 | `Some(…)`   | three peer reports     |

### `DebateRole`

```rust
pub enum DebateRole {
    Moderator,
}
```

Intentionally separate from `ArchitectType`. `ArchitectType` derives
`clap::ValueEnum` and is exposed as a user-facing CLI argument. The moderator
is never invoked standalone — it exists only inside the debate pipeline.
Mixing it into `ArchitectType` would pollute the help text and require
special-casing in existing single-agent code paths.

---

## Context Injection

```
Round 1                          Round 2
───────────────────────          ────────────────────────────────
DebateContext {                  DebateContext {
  round: Round1,                   round: Round2,
  own_report: None,                own_report: Some(own_r1),
  peer_reports: [],                peer_reports: [3 peers],
}                                }
     │                                 │
     ▼                                 ▼
System prompt only            System prompt
                              + Round 2 challenge template
                                (own_report injected at {own_report})
                                (peer_reports injected at {peer_reports})
```

The `{own_report}` and `{peer_reports}` placeholders in
`prompts/debate/round2_challenge.md` are substituted at generation time by
`generate_debate_agent()`. No template engine is used — plain `str::replace`
on the two known placeholder strings.

---

## Moderator Token Budget

The moderator receives all eight reports inline. With reports averaging ~2,000
tokens each, the input is roughly 16,000–40,000 tokens depending on report
verbosity. This fits comfortably within the context window of `claude-opus-4.6`
(the moderator's default model).

If reports are significantly longer than expected, future versions may add a
`## Key Claims` summary header to each report to reduce the effective input
size.

---

## Files Added in Phase 1

| File | Purpose |
|------|---------|
| `src/debate_agent.rs` | Core types (`DebateRound`, `DebateContext`, `PeerReport`) and agent generation functions |
| `src/prompts.rs` | `DebateRole` enum added (moderator agent name, description, model, prompt) |
| `prompts/system/moderator.md` | Moderator system prompt embedded at compile time |
| `prompts/debate/round2_challenge.md` | Round 2 challenge instruction template with `{own_report}` and `{peer_reports}` placeholders |
| `docs/debate.md` | This document |
