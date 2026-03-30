/// Structured error reporting in conf.md format.
///
/// Any program can report errors as structured markdown by:
/// 1. Defining an error enum
/// 2. Implementing `Diagnostic` on it
/// 3. Calling `render()` or `render_all()` in main
///
/// See error-handling.md for the full guide.

use std::fmt;

// ── Diagnostic trait ──

/// Implement this on your error enum to enable conf.md error output.
/// Each variant writes its fields to the formatter.
pub trait Diagnostic {
    fn render(&self, f: &mut ErrorFormatter);
}

// ── ErrorFormatter ──

/// Collects error fields and renders them as conf.md properties.
/// Used inside `Diagnostic::render()` implementations.
pub struct ErrorFormatter {
    code: String,
    line: Option<usize>,
    fields: Vec<(String, String)>,
    fix_override: Option<String>,
}

impl ErrorFormatter {
    fn new() -> Self {
        Self {
            code: String::new(),
            line: None,
            fields: Vec::new(),
            fix_override: None,
        }
    }

    /// Set the error code (e.g. "missing_property", "invalid_value", "io_error").
    pub fn code(&mut self, code: &str) {
        self.code = code.to_string();
    }

    /// Set the line number where the error occurred.
    pub fn line(&mut self, line: usize) {
        self.line = Some(line);
    }

    /// Add a named field to the error output.
    pub fn field(&mut self, key: &str, value: &str) {
        self.fields.push((key.to_string(), value.to_string()));
    }

    /// Override the auto-generated fix text.
    pub fn fix(&mut self, fix: &str) {
        self.fix_override = Some(fix.to_string());
    }

    fn get_field(&self, key: &str) -> Option<&str> {
        self.fields.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    fn derive_fix(&self) -> String {
        if let Some(ref fix) = self.fix_override {
            return fix.clone();
        }
        let key = self.get_field("key").unwrap_or("");
        match self.code.as_str() {
            "missing_property" => format!("add `- {}: <value>`", key),
            "missing_section" => {
                let expected = self.get_field("expected").unwrap_or("section");
                format!("add `{}` section", expected)
            }
            "missing_prose" => {
                let section = self.get_field("section").unwrap_or("heading");
                format!("add text after `## {}`", section)
            }
            "missing_table" => "add a markdown table".into(),
            "invalid_value" => {
                let expected = self.get_field("expected").unwrap_or("");
                if !key.is_empty() && !expected.is_empty() {
                    format!("change `{}` to {}", key, expected)
                } else if !key.is_empty() {
                    format!("fix the value of `{}`", key)
                } else {
                    "fix the value".into()
                }
            }
            "invalid_name" => "rename to match the expected pattern".into(),
            "io_error" => "check that the file exists and is readable".into(),
            "column_mismatch" => {
                let expected = self.get_field("expected").unwrap_or("column");
                format!("rename column to `{}`", expected)
            }
            "row_count" => "add at least one data row to the table".into(),
            "section_count" => "add the required sections".into(),
            _ => String::new(),
        }
    }

    fn write_to(&self, out: &mut String) {
        if let Some(line) = self.line {
            out.push_str(&format!("- line: {}\n", line));
        }
        for (key, value) in &self.fields {
            out.push_str(&format!("- {}: {}\n", key, value));
        }
        out.push_str(&format!("- code: {}\n", self.code));
        let fix = self.derive_fix();
        if !fix.is_empty() {
            out.push_str(&format!("- fix: {}\n", fix));
        }
    }
}

// ── Render functions ──

/// Render a single error as a dense markdown table (default).
pub fn render(source: &str, error: &dyn Diagnostic) -> String {
    render_all(source, &[error])
}

/// Render multiple errors as a dense markdown table (default).
pub fn render_all(source: &str, errors: &[&dyn Diagnostic]) -> String {
    let formatted: Vec<ErrorFormatter> = errors
        .iter()
        .map(|e| {
            let mut f = ErrorFormatter::new();
            e.render(&mut f);
            f
        })
        .collect();

    // Determine which columns have data
    let has_line = formatted.iter().any(|f| f.line.is_some());
    let has = |key: &str| formatted.iter().any(|f| f.get_field(key).is_some());
    let has_file = has("file");
    let has_section = has("section");
    let has_key = has("key");
    let has_got = has("got");

    let mut out = String::new();
    out.push_str(&format!("# {} — {} error(s)\n\n", source, errors.len()));

    // Header
    let mut cols = Vec::new();
    if has_line { cols.push("line"); }
    if has_file { cols.push("file"); }
    if has_section { cols.push("section"); }
    cols.push("code");
    if has_key { cols.push("key"); }
    if has_got { cols.push("got"); }
    cols.push("fix");

    out.push_str(&format!("| {} |\n", cols.join(" | ")));
    out.push_str(&format!("|{}|\n", cols.iter().map(|c| "-".repeat(c.len() + 2)).collect::<Vec<_>>().join("|")));

    // Rows
    for f in &formatted {
        let mut cells: Vec<String> = Vec::new();
        if has_line {
            cells.push(f.line.map_or(String::new(), |l| l.to_string()));
        }
        if has_file {
            cells.push(f.get_field("file").unwrap_or("").to_string());
        }
        if has_section {
            cells.push(f.get_field("section").unwrap_or("").to_string());
        }
        cells.push(f.code.clone());
        if has_key {
            cells.push(f.get_field("key").unwrap_or("").to_string());
        }
        if has_got {
            cells.push(f.get_field("got").unwrap_or("").to_string());
        }
        cells.push(f.derive_fix());

        out.push_str(&format!("| {} |\n", cells.join(" | ")));
    }

    out
}

/// Render a Vec of errors as a dense markdown table (default).
pub fn render_vec<E: Diagnostic>(source: &str, errors: &[E]) -> String {
    let refs: Vec<&dyn Diagnostic> = errors.iter().map(|e| e as &dyn Diagnostic).collect();
    render_all(source, &refs)
}

/// Render a single error as verbose conf.md (one section per error).
pub fn render_verbose(source: &str, error: &dyn Diagnostic) -> String {
    render_all_verbose(source, &[error])
}

/// Render multiple errors as verbose conf.md (one section per error).
pub fn render_all_verbose(source: &str, errors: &[&dyn Diagnostic]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", source));
    out.push_str(&format!("{} error(s)\n", errors.len()));

    for (i, error) in errors.iter().enumerate() {
        let mut f = ErrorFormatter::new();
        error.render(&mut f);
        out.push_str(&format!("\n## error-{}\n\n", i + 1));
        f.write_to(&mut out);
    }

    out
}

/// Render a Vec of errors as verbose conf.md (one section per error).
pub fn render_vec_verbose<E: Diagnostic>(source: &str, errors: &[E]) -> String {
    let refs: Vec<&dyn Diagnostic> = errors.iter().map(|e| e as &dyn Diagnostic).collect();
    render_all_verbose(source, &refs)
}

// ── Legacy Error struct (used by mdlint validate.rs) ──

/// Flat error struct used by mdlint's validator.
/// Implements Diagnostic so it can be rendered with the same formatter.
#[derive(Debug)]
pub struct Error {
    pub file: String,
    pub line: usize,
    pub section: String,
    pub code: &'static str,
    pub key: String,
    pub got: String,
    pub expected: String,
    pub fix: String,
}

impl Error {
    pub fn new(file: &str, line: usize, section: &str, code: &'static str) -> Self {
        Self {
            file: file.to_string(),
            line,
            section: section.to_string(),
            code,
            key: String::new(),
            got: String::new(),
            expected: String::new(),
            fix: String::new(),
        }
    }

    pub fn with_key(mut self, key: &str) -> Self {
        self.key = key.to_string();
        self
    }

    pub fn with_got(mut self, got: &str) -> Self {
        self.got = got.to_string();
        self
    }

    pub fn with_expected(mut self, expected: &str) -> Self {
        self.expected = expected.to_string();
        self
    }

    pub fn with_fix(mut self, fix: &str) -> Self {
        self.fix = fix.to_string();
        self
    }
}

impl Diagnostic for Error {
    fn render(&self, f: &mut ErrorFormatter) {
        f.code(self.code);
        if self.line > 0 {
            f.line(self.line);
        }
        if !self.file.is_empty() {
            f.field("file", &self.file);
        }
        if !self.section.is_empty() {
            f.field("section", &self.section);
        }
        if !self.key.is_empty() {
            f.field("key", &self.key);
        }
        if !self.got.is_empty() {
            f.field("got", &self.got);
        }
        if !self.expected.is_empty() {
            f.field("expected", &self.expected);
        }
        if !self.fix.is_empty() {
            f.fix(&self.fix);
        }
    }
}

/// Format a list of Error structs as a dense markdown table.
/// Convenience wrapper for structmd-lint.
pub fn format_errors(source: &str, _schema: &str, errors: &[Error]) -> String {
    render_vec(source, errors)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: [{}] {}", self.file, self.line, self.section, self.code)?;
        if !self.key.is_empty() {
            write!(f, " key={}", self.key)?;
        }
        if !self.got.is_empty() {
            write!(f, " got={}", self.got)?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Sample error enum (what a service would define) ──

    #[derive(Debug)]
    enum AppError {
        Io { path: String, source: std::io::Error },
        MissingProperty { line: usize, section: String, key: String },
        InvalidValue { line: usize, section: String, key: String, got: String, expected: String },
        BadSchedule { line: usize, section: String, got: String },
    }

    impl Diagnostic for AppError {
        fn render(&self, f: &mut ErrorFormatter) {
            match self {
                AppError::Io { path, source } => {
                    f.code("io_error");
                    f.field("file", path);
                    f.field("got", &source.to_string());
                }
                AppError::MissingProperty { line, section, key } => {
                    f.code("missing_property");
                    f.line(*line);
                    f.field("section", section);
                    f.field("key", key);
                }
                AppError::InvalidValue { line, section, key, got, expected } => {
                    f.code("invalid_value");
                    f.line(*line);
                    f.field("section", section);
                    f.field("key", key);
                    f.field("got", got);
                    f.field("expected", expected);
                }
                AppError::BadSchedule { line, section, got } => {
                    f.code("invalid_value");
                    f.line(*line);
                    f.field("section", section);
                    f.field("key", "schedule");
                    f.field("got", got);
                    f.fix("use `every 5m`, `every 2h`, or 5-field cron");
                }
            }
        }
    }

    // ── Dense table format tests ──

    #[test]
    fn render_single_error_table() {
        let err = AppError::MissingProperty {
            line: 3,
            section: "Container".into(),
            key: "image".into(),
        };
        let output = render("myapp", &err);
        assert!(output.contains("# myapp — 1 error(s)"), "missing header:\n{}", output);
        assert!(output.contains("| line"), "missing table header:\n{}", output);
        assert!(output.contains("missing_property"), "missing code:\n{}", output);
        assert!(output.contains("image"), "missing key:\n{}", output);
        assert!(output.contains("Container"), "missing section:\n{}", output);
    }

    #[test]
    fn render_multiple_errors_table() {
        let errors = vec![
            AppError::MissingProperty {
                line: 3,
                section: "Container".into(),
                key: "image".into(),
            },
            AppError::InvalidValue {
                line: 15,
                section: "backup".into(),
                key: "log".into(),
                got: "maybe".into(),
                expected: "true or false".into(),
            },
        ];
        let output = render_vec("myapp", &errors);
        assert!(output.contains("2 error(s)"), "wrong count:\n{}", output);
        // Both rows present
        assert!(output.contains("image"), "missing first error:\n{}", output);
        assert!(output.contains("maybe"), "missing second error:\n{}", output);
    }

    #[test]
    fn fix_auto_derived_missing_property() {
        let err = AppError::MissingProperty {
            line: 3,
            section: "Container".into(),
            key: "image".into(),
        };
        let output = render("myapp", &err);
        assert!(output.contains("add `- image: <value>`"), "missing fix:\n{}", output);
    }

    #[test]
    fn fix_auto_derived_invalid_value() {
        let err = AppError::InvalidValue {
            line: 10,
            section: "item".into(),
            key: "color".into(),
            got: "purple".into(),
            expected: "red, green, or blue".into(),
        };
        let output = render("myapp", &err);
        assert!(output.contains("change `color` to red, green, or blue"), "missing fix:\n{}", output);
    }

    #[test]
    fn fix_auto_derived_io_error() {
        let err = AppError::Io {
            path: "/tmp/missing.md".into(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let output = render("myapp", &err);
        assert!(output.contains("io_error"), "missing code:\n{}", output);
        assert!(output.contains("check that the file exists"), "missing fix:\n{}", output);
    }

    #[test]
    fn fix_override() {
        let err = AppError::BadSchedule {
            line: 14,
            section: "backup".into(),
            got: "every tuesday".into(),
        };
        let output = render("myapp", &err);
        assert!(output.contains("use `every 5m`, `every 2h`, or 5-field cron"), "missing fix override:\n{}", output);
    }

    #[test]
    fn legacy_error_struct_renders() {
        let err = Error::new("test.md", 5, "Container", "missing_property")
            .with_key("image")
            .with_expected("string");
        let output = render("mdlint", &err);
        assert!(output.contains("missing_property"), "missing code:\n{}", output);
        assert!(output.contains("image"), "missing key:\n{}", output);
        assert!(output.contains("add `- image: <value>`"), "missing fix:\n{}", output);
    }

    #[test]
    fn render_vec_empty() {
        let errors: Vec<AppError> = vec![];
        let output = render_vec("myapp", &errors);
        assert!(output.contains("0 error(s)"));
    }

    // ── Verbose format tests ──

    #[test]
    fn verbose_has_sections() {
        let err = AppError::MissingProperty {
            line: 3,
            section: "Container".into(),
            key: "image".into(),
        };
        let output = render_verbose("myapp", &err);
        assert!(output.contains("## error-1"), "verbose should have sections:\n{}", output);
        assert!(output.contains("- code: missing_property"), "verbose should have properties:\n{}", output);
    }

    #[test]
    fn dense_has_no_sections() {
        let err = AppError::MissingProperty {
            line: 3,
            section: "Container".into(),
            key: "image".into(),
        };
        let output = render("myapp", &err);
        assert!(!output.contains("## error-"), "dense should not have sections:\n{}", output);
    }
}
