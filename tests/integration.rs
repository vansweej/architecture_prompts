/// Integration tests that run the compiled binary and inspect stdout/stderr.
///
/// These tests require the binary to be built first. `cargo test` handles
/// this automatically via the test harness.
use std::fs;
use std::process::Command;

fn binary() -> Command {
    let bin = env!("CARGO_BIN_EXE_architecture_prompts");
    Command::new(bin)
}

// ── --list ───────────────────────────────────────────────────────────────────

#[test]
fn list_outputs_all_four_architect_names() {
    let output = binary().arg("--list").output().unwrap();
    assert!(output.status.success(), "exit code: {}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("principal"),
        "missing 'principal' in:\n{stdout}"
    );
    assert!(stdout.contains("design"), "missing 'design' in:\n{stdout}");
    assert!(
        stdout.contains("complexity"),
        "missing 'complexity' in:\n{stdout}"
    );
    assert!(
        stdout.contains("security"),
        "missing 'security' in:\n{stdout}"
    );
}

#[test]
fn list_exits_successfully_without_architect_arg() {
    let status = binary().arg("--list").status().unwrap();
    assert!(status.success());
}

// ── --dry-run ─────────────────────────────────────────────────────────────────

#[test]
fn dry_run_principal_outputs_valid_frontmatter() {
    let output = binary().args(["principal", "--dry-run"]).output().unwrap();
    assert!(output.status.success(), "exit code: {}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("---\n"),
        "must start with frontmatter delimiter"
    );
    assert!(
        stdout.contains("edit: deny"),
        "must contain edit: deny in readonly mode"
    );
    assert!(stdout.contains("mode: primary"));
    assert!(stdout.contains("temperature: 0.3"));
}

#[test]
fn dry_run_full_security_outputs_full_permissions() {
    let output = binary()
        .args(["security", "--dry-run", "--full"])
        .output()
        .unwrap();
    assert!(output.status.success(), "exit code: {}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("edit: allow"),
        "must contain edit: allow in full mode"
    );
    assert!(
        stdout.contains("\"*\": allow"),
        "must allow all bash in full mode"
    );
}

#[test]
fn dry_run_design_contains_prompt_body() {
    let output = binary().args(["design", "--dry-run"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The design prompt mentions the review board
    assert!(stdout.contains("architecture review board"));
}

#[test]
fn dry_run_complexity_contains_prompt_body() {
    let output = binary().args(["complexity", "--dry-run"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("simplicity"));
}

#[test]
fn dry_run_does_not_write_any_files() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output = binary()
        .args(["principal", "--dry-run"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    // .opencode/agents/ must not have been created
    assert!(!tmp.path().join(".opencode").exists());
}

// ── error cases ───────────────────────────────────────────────────────────────

#[test]
fn missing_architect_without_list_exits_with_error() {
    let status = binary().status().unwrap();
    assert!(!status.success());
}

#[test]
fn invalid_architect_name_exits_with_error() {
    let status = binary().arg("unknown-architect").status().unwrap();
    assert!(!status.success());
}

// ── model defaults ────────────────────────────────────────────────────────────

#[test]
fn dry_run_principal_contains_opus_model() {
    let output = binary().args(["principal", "--dry-run"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("model: github-copilot/claude-opus-4.6"),
        "principal must default to opus model, got:\n{stdout}"
    );
}

#[test]
fn dry_run_complexity_contains_sonnet_model() {
    let output = binary().args(["complexity", "--dry-run"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("model: github-copilot/claude-sonnet-4.6"),
        "complexity must default to sonnet model, got:\n{stdout}"
    );
}

#[test]
fn dry_run_with_model_override() {
    let output = binary()
        .args(["principal", "--dry-run", "--model", "openai/gpt-5"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("model: openai/gpt-5"),
        "--model override must appear verbatim in frontmatter, got:\n{stdout}"
    );
}

// ── review mode ───────────────────────────────────────────────────────────────

#[test]
fn dry_run_review_mode_contains_scoped_edit_permission() {
    let output = binary()
        .args(["principal", "--dry-run", "--review"])
        .output()
        .unwrap();
    assert!(output.status.success(), "exit code: {}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"reviews/arch-*.md\": allow"),
        "review mode must allow writes to reviews/arch-*.md, got:\n{stdout}"
    );
    assert!(
        stdout.contains("\"*\": deny"),
        "review mode must deny wildcard edits, got:\n{stdout}"
    );
}

#[test]
fn dry_run_review_mode_contains_review_output_instruction() {
    let output = binary()
        .args(["principal", "--dry-run", "--review"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("## Review Output"),
        "review mode must append the review-output instruction, got:\n{stdout}"
    );
    assert!(
        stdout.contains("arch-principal"),
        "review-output instruction must reference the persona file name, got:\n{stdout}"
    );
}

#[test]
fn dry_run_review_mode_does_not_allow_full_edit() {
    let output = binary()
        .args(["security", "--dry-run", "--review"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("edit: allow"),
        "review mode must not grant full edit permission, got:\n{stdout}"
    );
}

#[test]
fn review_and_full_flags_conflict() {
    let status = binary()
        .args(["principal", "--review", "--full"])
        .status()
        .unwrap();
    assert!(
        !status.success(),
        "--review and --full must be mutually exclusive"
    );
}

// ── --clean ───────────────────────────────────────────────────────────────────
#[test]
fn clean_exits_successfully_when_nothing_to_clean() {
    let tmp = tempfile::TempDir::new().unwrap();
    let status = binary()
        .arg("--clean")
        .current_dir(tmp.path())
        .status()
        .unwrap();
    assert!(status.success(), "exit code: {}", status);
}

#[test]
fn clean_does_not_require_architect_arg() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output = binary()
        .arg("--clean")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "--clean must not require an architect argument"
    );
}

#[test]
fn clean_removes_generated_agent_files() {
    let tmp = tempfile::TempDir::new().unwrap();
    // Manually create the directory structure the tool would produce.
    let agents_dir = tmp.path().join(".opencode").join("agents");
    fs::create_dir_all(&agents_dir).unwrap();
    fs::write(agents_dir.join("arch-principal.md"), "content").unwrap();
    fs::write(agents_dir.join("arch-security.md"), "content").unwrap();

    let output = binary()
        .arg("--clean")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "exit code: {}", output.status);

    // Both arch files must be gone.
    assert!(!agents_dir.join("arch-principal.md").exists());
    assert!(!agents_dir.join("arch-security.md").exists());
    // The now-empty directories must also be gone.
    assert!(!agents_dir.exists());
    assert!(!tmp.path().join(".opencode").exists());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Cleaned 2 agent file(s)"),
        "expected cleaned count in stderr, got:\n{stderr}"
    );
}

// ── --debate ──────────────────────────────────────────────────────────────────

/// Verifies that `--debate` alone satisfies the CLI parser (no "required
/// argument" error from clap). The binary will fail downstream because opencode
/// is not in the (deliberately emptied) PATH, but it must not fail with a clap
/// parse error.
#[test]
fn debate_flag_does_not_require_architect_arg() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output = binary()
        .arg("--debate")
        .current_dir(tmp.path())
        // Empty PATH so opencode can't be found → binary exits quickly
        // with "opencode not found" rather than actually running the pipeline.
        .env("PATH", "")
        .output()
        .unwrap();
    // The process fails with the opencode-not-found error, NOT with a clap
    // "required argument" error.  Verify by checking stderr.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("required"),
        "--debate must not trigger a clap 'required argument' error, stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("opencode") || !output.status.success(),
        "binary should fail on opencode check, not on clap parsing, stderr:\n{stderr}"
    );
}

/// Verifies `--debate --devils-advocate complexity` parses successfully; the
/// binary still fails (no opencode) but the failure must not be a clap error.
#[test]
fn debate_with_devils_advocate_parses_without_clap_error() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output = binary()
        .args(["--debate", "--devils-advocate", "complexity"])
        .current_dir(tmp.path())
        .env("PATH", "")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("error: unexpected argument"),
        "--devils-advocate must be a recognised flag, stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("required"),
        "no 'required argument' clap error expected, stderr:\n{stderr}"
    );
}

/// `--devils-advocate` without `--debate` must be rejected by clap.
#[test]
fn devils_advocate_without_debate_is_rejected() {
    let output = binary()
        .args(["--devils-advocate", "security"])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "--devils-advocate without --debate must fail"
    );
}

/// Full end-to-end debate pipeline smoke test.
///
/// Requires a live `opencode` binary in PATH and a repository to review.
/// Skipped in CI — run manually with `cargo test -- --ignored`.
#[test]
#[ignore]
fn debate_pipeline_runs_end_to_end() {
    let tmp = tempfile::TempDir::new().unwrap();
    let status = binary()
        .arg("--debate")
        .current_dir(tmp.path())
        .status()
        .unwrap();
    assert!(status.success(), "debate pipeline exited with: {}", status);
    assert!(
        tmp.path().join("reviews").join("final-report.md").exists(),
        "final-report.md must exist after a successful debate run"
    );
}
