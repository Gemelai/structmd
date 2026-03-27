/// Structured error reporting in conf.md format.
///
/// Any program can report errors as structured markdown by:
/// 1. Defining an error enum
/// 2. Implementing `Diagnostic` on it
/// 3. Calling `render()` or `render_all()` in main
///
/// See ERROR-HANDLING.md for the full guide.

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

/// Render a single error as conf.md.
pub fn render(source: &str, error: &dyn Diagnostic) -> String {
    render_all(source, &[error])
}

/// Render multiple errors as conf.md.
pub fn render_all(source: &str, errors: &[&dyn Diagnostic]) -> String {
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

/// Render a Vec of errors implementing Diagnostic.
pub fn render_vec<E: Diagnostic>(source: &str, errors: &[E]) -> String {
    let refs: Vec<&dyn Diagnostic> = errors.iter().map(|e| e as &dyn Diagnostic).collect();
    render_all(source, &refs)
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

/// Format a list of Error structs as conf.md.
/// Convenience wrapper for mdlint compatibility.
pub fn format_errors(source: &str, schema: &str, errors: &[Error]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", source));
    out.push_str(&format!("{} error(s) against {}\n", errors.len(), schema));

    for (i, error) in errors.iter().enumerate() {
        let mut f = ErrorFormatter::new();
        error.render(&mut f);
        out.push_str(&format!("\n## error-{}\n\n", i + 1));
        f.write_to(&mut out);
    }

    out
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

    // ── Trait and formatter tests ──

    #[test]
    fn render_single_error() {
        let err = AppError::MissingProperty {
            line: 3,
            section: "Container".into(),
            key: "image".into(),
        };
        let output = render("myapp", &err);
        assert!(output.contains("# myapp"));
        assert!(output.contains("1 error(s)"));
        assert!(output.contains("## error-1"));
        assert!(output.contains("- line: 3"));
        assert!(output.contains("- section: Container"));
        assert!(output.contains("- key: image"));
        assert!(output.contains("- code: missing_property"));
    }

    #[test]
    fn render_multiple_errors() {
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
        assert!(output.contains("2 error(s)"));
        assert!(output.contains("## error-1"));
        assert!(output.contains("## error-2"));
        assert!(output.contains("- key: image"));
        assert!(output.contains("- got: maybe"));
    }

    #[test]
    fn fix_auto_derived_missing_property() {
        let err = AppError::MissingProperty {
            line: 3,
            section: "Container".into(),
            key: "image".into(),
        };
        let output = render("myapp", &err);
        assert!(output.contains("- fix: add `- image: <value>`"));
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
        assert!(output.contains("- fix: change `color` to red, green, or blue"));
    }

    #[test]
    fn fix_auto_derived_io_error() {
        let err = AppError::Io {
            path: "/tmp/missing.md".into(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let output = render("myapp", &err);
        assert!(output.contains("- code: io_error"));
        assert!(output.contains("- fix: check that the file exists and is readable"));
    }

    #[test]
    fn fix_override() {
        let err = AppError::BadSchedule {
            line: 14,
            section: "backup".into(),
            got: "every tuesday".into(),
        };
        let output = render("myapp", &err);
        // Should use the override, not the auto-derived one
        assert!(output.contains("- fix: use `every 5m`, `every 2h`, or 5-field cron"));
    }

    #[test]

    #[test]
    fn legacy_error_struct_implements_diagnostic() {
        let err = Error::new("test.md", 5, "Container", "missing_property")
            .with_key("image")
            .with_expected("string");
        let output = render("mdlint", &err);
        assert!(output.contains("- code: missing_property"));
        assert!(output.contains("- key: image"));
        assert!(output.contains("- fix: add `- image: <value>`"));
    }

    #[test]
    fn render_vec_empty() {
        let errors: Vec<AppError> = vec![];
        let output = render_vec("myapp", &errors);
        assert!(output.contains("0 error(s)"));
        assert!(!output.contains("## error-"));
    }
}
