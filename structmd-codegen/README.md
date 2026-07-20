# structmd-codegen

Generate typed Rust config loaders from [structmd](https://crates.io/crates/structmd) schemas.

Reads a structmd schema and emits Rust source containing typed structs matching the grammar productions, plus a `parse()` function that walks a `structmd::parse::Document` into those structs.

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
