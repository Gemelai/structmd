# structmd-codegen

Generate typed Rust config loaders from [structmd](https://crates.io/crates/structmd) schemas.

Reads a structmd schema and emits Rust source containing typed structs matching the grammar productions, plus a `parse()` function that validates input against the embedded schema and walks the validated document into those structs.

Validation runs through `structmd::validate` — the same validator `structmd-lint` uses — so the generated loader reports the same structured errors as the linter: `Result<Config, Vec<structmd::errors::Error>>`, renderable as a structmd error table via `structmd::errors::render_vec`. A `parse_with_source(text, source)` variant fills the `file` field of errors with a source name.

## As a build dependency

```toml
[dependencies]
structmd = "0.1"

[build-dependencies]
structmd-codegen = "0.1"
```

`build.rs`:

```rust,ignore
fn main() {
    let schema = include_str!("schema/workflow.schema.md");
    let code = structmd_codegen::generate_from_text(schema, "workflow");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::write(format!("{}/workflow_config.rs", out_dir), code).unwrap();
    println!("cargo:rerun-if-changed=schema/workflow.schema.md");
}
```

Then include the generated module:

```rust,ignore
include!(concat!(env!("OUT_DIR"), "/workflow_config.rs"));
```

## As a CLI

```sh
structmd-codegen <schema.md> <output.rs>
```

See the [structmd repository](https://github.com/Gemelai/structmd) for the format specification.

## License

MIT. Copyright Susan Roylance and Stephen Roylance.
