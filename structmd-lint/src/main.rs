//! structmd-lint — Schema-driven linter for structured markdown files.
//!
//! Validates a structmd document against a schema
//! containing ```grammar, ```types, and ```table blocks.
//!
//! Usage: structmd-lint --schema <schema.md> <file.md>

mod validate;

use clap::Parser;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "structmd-lint", about = "Schema-driven linter for structured markdown files")]
struct Cli {
    /// Path to the structmd file to lint
    file: PathBuf,

    /// Path to the schema file (a structmd document with ```grammar, ```types, ```table blocks)
    #[arg(long)]
    schema: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    let text = match std::fs::read_to_string(&cli.file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", cli.file.display(), e);
            process::exit(2);
        }
    };

    let schema_text = match std::fs::read_to_string(&cli.schema) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", cli.schema.display(), e);
            process::exit(2);
        }
    };

    let schema_name = cli
        .schema
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("schema")
        .to_string();

    let schema = match structmd::schema::load_schema(&schema_text, &schema_name) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: invalid schema {}: {}", cli.schema.display(), e);
            process::exit(2);
        }
    };

    let doc = structmd::parse::parse(&text);
    let path_str = cli.file.display().to_string();
    let schema_str = cli.schema.display().to_string();
    let errors = validate::validate(&doc, &schema, &path_str);

    if !errors.is_empty() {
        print!("{}", structmd::errors::format_errors("structmd-lint", &schema_str, &errors));
        process::exit(1);
    }
}
