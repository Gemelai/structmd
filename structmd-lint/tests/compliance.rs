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
/// We parse the actual output as conf.md too and match fields.
fn actual_contains_expected(actual_text: &str, expected: &ExpectedError) -> bool {
    let doc = structmd::parse::parse(actual_text);
    let sections = doc
        .nodes
        .first()
        .map(|n| n.sections.as_slice())
        .unwrap_or(&[]);

    'outer: for sec in sections {
        let prop = |k: &str| {
            sec.properties
                .iter()
                .find(|p| p.key == k)
                .map(|p| p.value.as_str())
        };

        if let Some(ref code) = expected.code {
            if prop("code") != Some(code.as_str()) {
                continue 'outer;
            }
        }
        if let Some(ref section) = expected.section {
            if prop("section") != Some(section.as_str()) {
                continue 'outer;
            }
        }
        if let Some(ref key) = expected.key {
            if prop("key") != Some(key.as_str()) {
                continue 'outer;
            }
        }
        if let Some(ref got) = expected.got {
            if prop("got") != Some(got.as_str()) {
                continue 'outer;
            }
        }
        return true;
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
fn invalid_bad_cron() {
    test_invalid("bad-cron");
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
