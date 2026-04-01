//! structmd — a structured document format for machine-generated and human-reviewed content.
//!
//! Structure comes from line prefixes (`#`, `##`, `-`), not punctuation.
//! Documents are deterministically parsable without a schema; a schema adds validation.
//!
//! # Quick start
//!
//! ```rust
//! let doc = structmd::parse::parse("# Config\n\n## server\n- host: localhost\n- port: 8080\n");
//! let section = &doc.nodes[0].sections[0];
//! assert_eq!(section.heading.text, "server");
//! assert_eq!(section.properties[0].key, "host");
//! ```
//!
//! See [`parse`] for the document model, [`schema`] for schema loading,
//! and [`errors`] for structured error output.

pub mod errors;
pub mod parse;
pub mod schema;
pub mod validate;

/// Validate a parsed document against a schema and return any errors.
///
/// Convenience wrapper around [`validate::validate`].
pub fn lint(
    doc: &parse::Document,
    schema: &schema::Schema,
    file: &str,
) -> Vec<errors::Error> {
    validate::validate(doc, schema, file)
}
