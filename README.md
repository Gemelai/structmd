# structmd

A structured document format with no punctuation syntax.

Structure in structmd comes from line prefixes — `#`, `##`, `-` — not from brackets, quotes, or commas. That single property is why the format works for three different audiences for the same reason:

- **Easy to generate**, even for weak models. Schemas are written once by a developer or capable model — that encodes the domain expertise. Documents written against a known schema have no brackets to match, no quotes to escape, no commas to forget. Fill in the fields, write the prose. The linter validates the output and returns structured fix text; the model corrects and retries without human intervention.
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

## Where it fits

Structured formats usually trade off between machine-parseability and human-readability. Formats optimized for machines are precise but require tooling to read and write comfortably. Formats optimized for humans are readable but tend to be ambiguous or fragile to parse.

structmd sits at the intersection of both. The structure is deterministic — every line classifies itself, there is no ambiguity, a conformant parser produces the same AST from the same input every time. The syntax is readable without tooling — the same document a parser processes is the document a human edits in a text editor and reviews in a diff.

The complexity ceiling that makes this possible is deliberate. structmd handles shallow, bounded structure well: a few levels of headings, typed properties, optional prose. Data that needs deeper nesting or more expressive type structure belongs in a format built for that. structmd is the right choice when the document needs to be correct enough to validate, simple enough to read at a glance, and writable by a human or a model without special tooling.

Good fits: config files, error reports, tool registries, evaluation documents, agent-authored forms that a human needs to review.

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
impl structmd::errors::Diagnostic for AppError {
    fn render(&self, f: &mut structmd::errors::ErrorFormatter) {
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
let output = structmd::errors::render_vec("myapp", &errors);
```

The output is a structmd document. A human can read it. An agent can parse it. Another process can validate it against the errors schema.

This pattern works because explicit error enums are machine-readable in a way that string messages are not. Every variant is a named, typed statement about what can go wrong — documentation that the compiler enforces, and structure that an agent can reason about without reading the function bodies that produce it.

The crate does not prescribe how errors propagate internally. Use `?`, `thiserror`, manual matching — whatever fits. The `Diagnostic` trait is a presentation contract for the process boundary, not a propagation mechanism.

## Crates

| Crate | Purpose |
|-------|---------|
| [`structmd`](https://crates.io/crates/structmd) | Parser, schema loader, `Diagnostic` trait, error renderer |
| [`structmd-lint`](https://crates.io/crates/structmd-lint) | CLI validator: `structmd-lint --schema schema.md file.md` |
| [`structmd-codegen`](https://crates.io/crates/structmd-codegen) | Code generator: emits typed Rust structs from a schema, as a library for `build.rs` or a CLI |

```sh
cargo add structmd              # library: parse, validate, render errors
cargo install structmd-lint     # CLI validator
```

## Specification

See [specification.md](https://github.com/Gemelai/structmd/blob/main/specification.md) for the full format specification. The compliance test suite in [tests/fixtures/](https://github.com/Gemelai/structmd/tree/main/tests/fixtures) is normative — it defines expected behavior for any conformant implementation.

## License

MIT. Copyright Susan Roylance and Stephen Roylance.
