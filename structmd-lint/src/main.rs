/// mdlint — Schema-driven linter for conf.md files.
///
/// Validates a markdown config file against a conf.md schema
/// containing ```bnf, ```types, and ```table blocks.
///
/// Usage: mdlint --schema <schema.conf.md> <file.conf.md>

mod validate;

use clap::Parser;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "mdlint", about = "Schema-driven linter for conf.md files")]
struct Cli {
    /// Path to the conf.md file to lint
    file: PathBuf,

    /// Path to the schema file (conf.md with ```bnf, ```types, ```table blocks)
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

    if errors.is_empty() {
        let section_count: usize = doc.nodes.iter().map(|n| n.sections.len()).sum();
        println!("ok: {} ({} sections, 0 errors)", path_str, section_count);
    } else {
        print!("{}", structmd::errors::format_errors("mdlint", &schema_str, &errors));
        process::exit(1);
    }
}
