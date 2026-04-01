use std::path::PathBuf;
use std::process::Command;

fn binary() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.pop();
    path.push("structmd-workflow");
    path
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

fn run(fixture: &str) -> (i32, String) {
    let path = fixtures_dir().join(fixture);
    let out = Command::new(binary())
        .arg(&path)
        .output()
        .expect("failed to run structmd-workflow");
    let code = out.status.code().unwrap_or(-1);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (code, text)
}

// ── Valid fixtures ──

#[test]
fn valid_hello_exits_zero() {
    let (code, output) = run("valid/hello.workflow.md");
    assert_eq!(code, 0, "expected exit 0:\n{}", output);
}

#[test]
fn valid_hello_runs_steps() {
    let (_, output) = run("valid/hello.workflow.md");
    assert!(output.contains("step: setup"), "missing setup output:\n{}", output);
    assert!(output.contains("step: greet"), "missing greet output:\n{}", output);
}

#[test]
fn valid_multi_step_runs_in_order() {
    let (code, output) = run("valid/multi-step.workflow.md");
    assert_eq!(code, 0, "expected exit 0:\n{}", output);
    let fetch_pos = output.find("fetching").unwrap_or(usize::MAX);
    let compile_pos = output.find("compiling").unwrap_or(usize::MAX);
    let test_pos = output.find("testing").unwrap_or(usize::MAX);
    let report_pos = output.find("done").unwrap_or(usize::MAX);
    assert!(fetch_pos < compile_pos, "fetch should run before compile:\n{}", output);
    assert!(compile_pos < test_pos, "compile should run before test:\n{}", output);
    assert!(test_pos < report_pos, "test should run before report:\n{}", output);
}

// ── Invalid fixtures ──

#[test]
fn invalid_missing_command_exits_nonzero() {
    let (code, output) = run("invalid/missing-command.workflow.md");
    assert_ne!(code, 0, "expected non-zero exit:\n{}", output);
}

#[test]
fn invalid_missing_command_reports_error_code() {
    let (_, output) = run("invalid/missing-command.workflow.md");
    assert!(output.contains("missing_property"), "expected missing_property error:\n{}", output);
    assert!(output.contains("command"), "expected 'command' in error:\n{}", output);
}

#[test]
fn invalid_no_steps_exits_nonzero() {
    let (code, output) = run("invalid/no-steps.workflow.md");
    assert_ne!(code, 0, "expected non-zero exit:\n{}", output);
}

#[test]
fn invalid_error_output_is_structmd() {
    let (_, output) = run("invalid/missing-command.workflow.md");
    // Output should be a structmd table — starts with a heading
    assert!(output.contains("# structmd-workflow"), "expected structmd header:\n{}", output);
}

// ── Dependency handling ──

#[test]
fn dep_on_failed_step_skips_dependent() {
    // Write a temp fixture where the first step fails
    let dir = tempfile::tempdir().unwrap();
    let fixture = dir.path().join("dep-fail.workflow.md");
    std::fs::write(&fixture, "\
# test

## first
- command: exit 1

## second
- command: echo should-not-run
- depends: first
").unwrap();

    let out = Command::new(binary())
        .arg(&fixture)
        .output()
        .expect("failed to run");

    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_ne!(out.status.code().unwrap_or(0), 0, "expected failure:\n{}", text);
    assert!(!text.contains("should-not-run"), "second step should not have run:\n{}", text);
}
