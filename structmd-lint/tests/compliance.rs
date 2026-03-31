/// Compliance test suite for conf.md
///
/// Runs mdlint against fixture files in tests/fixtures/.
/// Valid fixtures must lint with 0 errors.
/// Invalid fixtures must produce errors matching the expected .errors.md file.
///
/// The .errors.md files use a subset of conf.md: each H2 error section lists
/// the expected code/section/key/got fields. The test checks that every expected
/// error appears in the actual output (by matching those fields).

use std::path::{Path, PathBuf};
use std::process::Command;

fn mdlint_binary() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary name
    path.pop(); // remove deps/
    path.push("structmd-lint");
    path
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests/fixtures")
}

fn run_mdlint(schema: &Path, input: &Path) -> (i32, String) {
    let output = Command::new(mdlint_binary())
        .arg("--schema")
        .arg(schema)
        .arg(input)
        .output()
        .expect("failed to run mdlint");

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, format!("{}{}", stdout, stderr))
}

/// Parse a .errors.md file into expected error checks.
/// Each H2 section is an error. We extract the code/section/key/got properties.
struct ExpectedError {
    code: Option<String>,
    section: Option<String>,
    key: Option<String>,
    got: Option<String>,
}

fn parse_expected_errors(text: &str) -> Vec<ExpectedError> {
    let doc = structmd::parse::parse(text);
    let mut expected = Vec::new();

    let sections = doc
        .nodes
        .first()
        .map(|n| n.sections.as_slice())
        .unwrap_or(&[]);

    for sec in sections {
        let prop = |k: &str| {
            sec.properties
                .iter()
                .find(|p| p.key == k)
                .map(|p| p.value.clone())
        };
        expected.push(ExpectedError {
            code: prop("code"),
            section: prop("section"),
            key: prop("key"),
            got: prop("got"),
        });
    }

    expected
}

/// Check that an expected error appears in the actual mdlint output.
/// The output is a markdown table — match by checking each row contains the expected values.
fn actual_contains_expected(actual_text: &str, expected: &ExpectedError) -> bool {
    // Simple text matching on table rows — each row is a | delimited line
    for line in actual_text.lines() {
        if !line.starts_with('|') || line.contains("---") {
            continue;
        }
        // Skip header row
        if line.contains("code") && line.contains("fix") {
            continue;
        }

        let matches_code = expected.code.as_ref()
            .map_or(true, |c| line.contains(c.as_str()));
        let matches_section = expected.section.as_ref()
            .map_or(true, |s| line.contains(s.as_str()));
        let matches_key = expected.key.as_ref()
            .map_or(true, |k| line.contains(k.as_str()));
        let matches_got = expected.got.as_ref()
            .map_or(true, |g| line.contains(g.as_str()));

        if matches_code && matches_section && matches_key && matches_got {
            return true;
        }
    }
    false
}

// ── Valid fixtures ──

fn test_valid(name: &str) {
    let dir = fixtures_dir().join("valid");
    let input = dir.join(format!("{}.md", name));
    let schema = dir.join(format!("{}.schema.md", name));

    assert!(input.exists(), "fixture not found: {}", input.display());
    assert!(schema.exists(), "schema not found: {}", schema.display());

    let (code, output) = run_mdlint(&schema, &input);
    assert_eq!(
        code, 0,
        "expected valid file to pass lint.\nFile: {}\nOutput:\n{}",
        name, output
    );
}

#[test]
fn valid_simple() {
    test_valid("simple");
}

#[test]
fn valid_multi_h1() {
    test_valid("multi-h1");
}

#[test]
fn valid_nested_h3() {
    test_valid("nested-h3");
}

#[test]
fn valid_prose() {
    test_valid("prose");
}

#[test]
fn valid_code_blocks() {
    test_valid("code-blocks");
}

#[test]
fn valid_lists() {
    test_valid("lists");
}

#[test]
fn valid_tables() {
    test_valid("tables");
}

#[test]
fn valid_scoped_types() {
    test_valid("scoped-types");
}

#[test]
fn valid_optional_h1() {
    test_valid("optional-h1");
}

#[test]
fn valid_scoped_leak() {
    test_valid("scoped-leak");
}

// ── Invalid fixtures ──

fn test_invalid(name: &str) {
    let dir = fixtures_dir().join("invalid");
    let input = dir.join(format!("{}.md", name));
    let schema = dir.join(format!("{}.schema.md", name));
    let errors_file = dir.join(format!("{}.errors.md", name));

    assert!(input.exists(), "fixture not found: {}", input.display());
    assert!(schema.exists(), "schema not found: {}", schema.display());
    assert!(
        errors_file.exists(),
        "errors file not found: {}",
        errors_file.display()
    );

    let (code, output) = run_mdlint(&schema, &input);
    assert_eq!(
        code, 1,
        "expected invalid file to fail lint.\nFile: {}\nOutput:\n{}",
        name, output
    );

    let expected_text = std::fs::read_to_string(&errors_file).unwrap();
    let expected = parse_expected_errors(&expected_text);

    for (i, exp) in expected.iter().enumerate() {
        assert!(
            actual_contains_expected(&output, exp),
            "expected error {} not found in output.\n\
             Expected: code={:?} section={:?} key={:?} got={:?}\n\
             Actual output:\n{}",
            i + 1,
            exp.code,
            exp.section,
            exp.key,
            exp.got,
            output
        );
    }
}

#[test]
fn invalid_missing_property() {
    test_invalid("missing-property");
}

#[test]
fn invalid_bad_enum() {
    test_invalid("bad-enum");
}

#[test]
fn invalid_bad_bool() {
    test_invalid("bad-bool");
}

#[test]
fn invalid_missing_section() {
    test_invalid("missing-section");
}

#[test]
fn invalid_missing_prose() {
    test_invalid("missing-prose");
}
