# structmd

A structured document format with no punctuation syntax.

Structure in structmd comes from line prefixes — `#`, `##`, `-` — not from brackets, quotes, or commas. That single property is why the format works for three different audiences for the same reason:

- **Easy to generate**, even for weak models. There are no brackets to match, no quotes to escape, no commas to forget. A model that can write a markdown list can emit valid structmd.
- **Easy to review** for humans. The structure is visually obvious. No punctuation noise to read past.
- **Deterministic to parse**. Every line classifies itself from its prefix. No ambiguity, no backtracking, no context-sensitive tokenization.

A structmd document has a schema. The schema defines which headings appear, in what order, with what typed properties. A document that passes validation has exactly the structure the schema promised — the consumer, whether an agent or a process, can trust it without defensive parsing.

The format is not general-purpose markdown. It is a structured document format whose syntax is borrowed from markdown. A markdown renderer will display it sensibly. The renderer is not the authority on what the document means. The schema is.

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

## Error handling

structmd includes an opinionated pattern for structured error output at process boundaries.

The opinion: errors should be defined as explicit enum variants, each carrying the data relevant to that failure mode. Not strings. Not opaque boxes. Named, typed variants — one per thing that can go wrong.

```rust
enum AppError {
    MissingProperty { line: usize, section: String, key: String },
    InvalidValue { line: usize, section: String, key: String, got: String, expected: String },
    IoError { path: String, source: std::io::Error },
}
```

Implement `Diagnostic` on the enum. Each variant writes its fields to an `ErrorFormatter`:

```rust
impl Diagnostic for AppError {
    fn render(&self, f: &mut ErrorFormatter) {
        match self {
            AppError::MissingProperty { line, section, key } => {
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
