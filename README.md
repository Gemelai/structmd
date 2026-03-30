# structmd

Structured documents for systems where humans, agents, and programs all need to read and write the same things.

A structmd document is a markdown file with a schema. The schema defines what headings appear, in what order, with what properties and types. A document that conforms to its schema can be parsed into a typed structure, validated for correctness, and rendered to human readers — all without separate representations for each audience.

The format is deliberately not general-purpose markdown. It is a structured document format whose syntax is borrowed from markdown for readability. A markdown renderer will display a structmd document sensibly. The renderer is not the authority on what the document means. The schema is.

## What it looks like

A config file:

```markdown
# Servers

## fetch

- command: ./target/debug/weid-mcp-fetch
- timeout: 30

## brave-search

- command: ./target/debug/weid-mcp-brave
- timeout: 30
```

Its schema:

    # Server Config Schema

    ```grammar
    document ::= H1("Servers") server+
    server   ::= H2(IDENTIFIER) property+
    ```

    ```types
    @command : string, required
    @timeout : integer, default("60")
    ```

The schema is also a structmd document. No separate schema language.

## Why structured text

Programs exchange data at boundaries — config files, error reports, API responses, log output. The usual choices are human-readable formats that are hard to parse reliably (freeform text, ad-hoc markdown) or machine-readable formats that are hard for humans to read and write (JSON, TOML, protobuf). Agents add a third party to that exchange who can work with either, but work best when the structure is explicit and the format is close to natural language.

structmd is an attempt to hold all three at once. The structure is explicit enough for a parser. The syntax is close enough to natural language for a human or an agent to write without consulting a reference. The schema is in the same format as the document, so the same tools read both.

## Error handling

structmd includes an opinionated pattern for structured error output at process boundaries.

The opinion: errors should be defined as explicit enum variants, each carrying the data relevant to that failure mode. Not strings. Not opaque boxes. Named, typed variants — one per thing that can go wrong.

```rust
enum SozuError {
    MissingProperty { line: usize, section: String, key: String },
    InvalidValue { line: usize, section: String, key: String, got: String, expected: String },
    IoError { path: String, source: std::io::Error },
}
```

Implement `Diagnostic` on the enum. Each variant writes its fields to an `ErrorFormatter`:

```rust
impl Diagnostic for SozuError {
    fn render(&self, f: &mut ErrorFormatter) {
        match self {
            SozuError::MissingProperty { line, section, key } => {
                f.code("missing_property");
                f.line(*line);
                f.field("section", section);
                f.field("key", key);
            }
            // ...
        }
    }
}
```

At the process boundary, render the errors as a structmd table:

```rust
let output = structmd::render_vec("myapp", &errors);
```

The output is a structmd document. A human can read it. An agent can parse it. Another process can validate it against the errors schema.

This pattern works because explicit error enums are machine-readable in a way that string messages are not. Every variant is a named, typed statement about what can go wrong — documentation that the compiler enforces, and structure that an agent can reason about without reading the function bodies that produce it.

The crate does not prescribe how errors propagate internally. Use `?`, `thiserror`, manual matching — whatever fits. The `Diagnostic` trait is a presentation contract for the process boundary, not a propagation mechanism.

## Crates

| Crate | Purpose |
|-------|---------|
| `structmd` | Parser, schema loader, `Diagnostic` trait, error renderer |
| `structmd-lint` | CLI validator: `structmd-lint --schema schema.md file.md` |
| `structmd-codegen` | CLI code generator: emits typed Rust structs from a schema |

## Specification

See `specification.md` for the full format specification. The compliance test suite in `tests/fixtures/` is normative — it defines expected behavior for any conformant implementation.

## License

MIT. Copyright Susan Roylance and Stephen Roylance.
