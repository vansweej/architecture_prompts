/// Integration tests that run the compiled binary and inspect stdout/stderr.
///
/// These tests require the binary to be built first. `cargo test` handles
/// this automatically via the test harness.
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
