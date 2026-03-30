# structmd — SME Reference

This is the complete context document for any agent taking over development of structmd. Read this before touching any code.

## What structmd is

A structured document format defined by BNF schemas. structmd documents are not markdown documents that happen to have structure — they are structured documents that happen to be readable as markdown. Markdown renderers will display them sensibly, but the renderer is not the authority on what they mean. The schema is.

Think ASN.1, not TOML. The syntax is borrowed from markdown for human readability and LLM-nativeness. The semantics are entirely structmd's own. Code blocks with info strings are always structured properties — the format owns them. Fenced code blocks, headings, and list items are structural tokens with schema-defined meaning, not presentation hints.

Target use cases: config files, error reports, tool registries, any structured document that needs to be human-readable, machine-parseable, and git-diff-friendly.

The name: **struct**ured **m**ark**d**own. MIT licensed, copyright Susan and Stephen Roylance.

## Repository layout

```
~/structmd/                        — working copy
/projects/structmd.git             — bare repo

structmd-errors/src/lib.rs         — Diagnostic trait, ErrorFormatter, render functions (495 lines)
structmd/src/parse.rs              — markdown parser → Document AST (708 lines)
structmd/src/schema.rs             — schema loader, BNF resolution (1050 lines)
structmd/src/lib.rs                — re-exports parse, schema, errors
structmd-lint/src/validate.rs      — schema validation engine (819 lines)
structmd-lint/src/main.rs          — CLI entry point (71 lines)
structmd-codegen/src/lib.rs        — Rust code generator (574 lines)
structmd-codegen/src/main.rs       — CLI entry point (11 lines)
tests/fixtures/valid/              — compliance fixtures (valid files + schemas)
tests/fixtures/invalid/            — compliance fixtures (invalid files + schemas + expected errors)
```

## Dependency graph

```
structmd-errors          ← zero deps, standalone
    ↑
structmd                 ← depends on structmd-errors
    ↑         ↑
structmd-lint  structmd-codegen
    ↑
(weyd services via submodule)
```

`structmd-errors` is intentionally separate so consumers that only need error reporting (the Diagnostic trait) don't pull in the parser.

## The format

Markdown elements map to config concepts:

| Markdown | Config concept |
|----------|---------------|
| `# Heading` | Section / namespace |
| `## Heading` | Named entry |
| `### Heading` | Nested structure |
| `- key: value` | Property |
| `- **key:** value` | Property (bold style) |
| `` ```tag `` code block | Multi-line property (tag = property name) |
| Markdown table | Structured data |
| Paragraph text | Prose — captured if schema says `prose`, otherwise documentation |
| Comma-separated values | Lists (when schema type is `list(T)`) |

### Schemas

Schemas are themselves structmd files containing fenced code blocks:

- `` ```grammar `` — structural grammar
- `` ```types `` or `` ```types:production `` — property type constraints
- `` ```table `` or `` ```table:production `` — table column constraints
- Everything else — documentation (ignored by the loader)

### Example schema

```markdown
# Tool Registry Schema

```grammar
document ::= H1("Tools") tool+
tool     ::= H2(SNAKE_CASE) prose property* table
```

```types
@server     : string, required
@parameters : label
```

```table
@name : string
@type : enum(string, integer, number, boolean, array, object)
@required : enum(yes, no)
@description : string
```
```

### BNF tokens

| Token | Meaning |
|-------|---------|
| `H1("text")` | Required H1 with exact text |
| `H1` | Required H1, any text |
| `H2(SNAKE_CASE)` | H2 with snake_case name pattern |
| `H2(IDENTIFIER)` | H2 with identifier pattern (alphanumeric + _ + -) |
| `H2("Container")` | H2 with exact text |
| `H3(...)` | Same patterns at H3 level |
| `prose` | Expects prose text (captured as `Section.prose`) |
| `property` | Expects `- key: value` lines |
| `table` | Expects a markdown table |
| `+` | One or more |
| `*` | Zero or more |
| `?` | Optional |
| named ref | `tool+` resolves to the `tool ::= ...` production |

### Type annotations

```
@key : string                    — non-empty string
@key : string, required          — error if missing
@key : string, default("value")  — use default if missing
@key : bool                      — true or false
@key : bool, default(true)       — defaults to true
@key : integer                   — parses as i64
@key : number                    — parses as f64
@key : path                      — starts with / or ./
@key : enum(a, b, c)             — one of listed values
@key : list(string)              — comma-separated values
@key : cron                      — "every Nm/Nh" or 5-field cron expression
@key : text                      — any text including empty
@key : label                     — property name only, no value validation
```

Inline comments: `@key : string  # comment here` — the `#` and everything after is stripped.

### Scoped properties

```` ```types:production ```` scopes properties to a specific BNF production. Different section types get different property sets:

```
```types:container
@image : string, required
```

```types:server
@command : string, required
```
```

Without a tag, ```` ```types ```` is global — applies to all sections that `expects_properties`.

## Crate details

### structmd-errors

**The Diagnostic trait** — implement on your error enum:

```rust
pub trait Diagnostic {
    fn render(&self, f: &mut ErrorFormatter);
}
```

**ErrorFormatter** — collects fields, derives fix text from error code:

```rust
f.code("missing_property");  // error code from the schema enum
f.line(3);                    // line number
f.field("section", "Container");
f.field("key", "image");
f.fix("custom fix override"); // optional, auto-derived if not set
```

**Render functions:**
- `render(source, &error)` — dense markdown table (default)
- `render_verbose(source, &error)` — section-per-error format
- `render_vec(source, &errors)` — table from Vec
- `render_all(source, &[&dyn Diagnostic])` — table from trait objects

**Auto-derived fix text** based on error code:
- `missing_property` with key "image" → `add \`- image: <value>\``
- `invalid_value` with key "color", expected "red, green" → `change \`color\` to red, green`
- `io_error` → `check that the file exists and is readable`

The `Error` struct is a flat convenience type that implements `Diagnostic`. Used by structmd-lint internally.

### structmd (parser + schema)

**Parser** (`parse.rs`): single-pass, line-by-line, zero dependencies. Produces:

```rust
Document { nodes: Vec<H1Node> }
H1Node { heading, prose, properties, sections }
Section { heading, prose, properties, table, children }
Property { key, value, bold, line }
Table { header_line, columns, rows }
```

Key behaviors:
- Fenced code blocks with an info string (`` ```tag ``) become properties (key = tag, value = block content)
- Untagged code blocks are skipped
- Prose = text between heading and first structural element (property, table, code block, child heading)
- Properties: recognizes `- **key:** value` (bold), `- key: value` (plain), `- key:` (label)
- H3 headings nest under the current H2
- Multiple H1 headings create separate `H1Node` entries
- H1 nodes can have their own properties (for schemas like orchestrator with `# Settings` having `- llama_server: ...`)

**Schema loader** (`schema.rs`): reads a structmd file, extracts BNF/types/table blocks, resolves productions into a tree.

```rust
Schema { name, nodes: Vec<H1Schema>, global_properties, global_table }
H1Schema { production_name, text, quantity, expects_properties, properties, children }
SectionSchema { production_name, level, name_pattern, quantity, expects_prose, expects_properties, expects_table, properties, table, children }
```

The BNF resolver:
- Starts from the `document` production (or first production if no `document`)
- Named references inline the referenced production's tokens
- Quantifiers on references propagate to the created node
- H3 tokens nest under the last H2 in scope
- Scoped `types:production` blocks attach to sections with matching `production_name`
- Circular references are detected and reported

### structmd-lint

**Validator** (`validate.rs`): walks Document against Schema recursively.

Checks: H1 presence/text, section count (quantity), name patterns, prose presence, required properties, value types (string, bool, enum, path, integer, number, list, cron), table columns by position, table row counts, per-cell type validation.

Errors use `structmd_errors::Error` with codes from the error schema enum.

**CLI**: `structmd-lint --schema <schema.md> <file.md>`

Exit 0 + summary on success. Exit 1 + markdown table of errors on failure.

### structmd-codegen

Reads a schema, emits a `.rs` file containing:
- Rust structs (one per BNF production, `PascalCase` + `Config` suffix)
- A `parse()` function that walks the Document AST into typed structs
- Default values from schema modifiers
- `Vec<String>` for list types, `Option<T>` for optional fields

**BNF → Rust mapping:**

| BNF | Rust |
|-----|------|
| `H2("Container") property+` | struct (exact heading = fixed field name) |
| `H2(IDENTIFIER) property+` | `Vec<T>` where T has `name: String` from heading |
| `H3(IDENTIFIER) property+` | `Vec<T>` nested under parent struct |
| `?` quantifier | `Option<T>` |
| `+` quantifier | `Vec<T>`, error if empty |
| `*` quantifier | `Vec<T>`, empty ok |
| `prose` | `prose: Option<String>` |

Flat schemas (single H1, one section type) flatten the H1 into the root struct to avoid an unnecessary wrapper.

## Testing

**118 tests total:**
- 56 in structmd (parser unit tests + schema loader tests)
- 46 in structmd-lint (validator unit tests + 16 compliance fixtures)
- 16 in structmd-codegen (struct generation, field types, parse function)

**Compliance suite** (`tests/fixtures/`):
- `valid/` — 10 fixture pairs (`.md` + `.schema.md`). Must lint with 0 errors.
- `invalid/` — 6 fixture triplets (`.md` + `.schema.md` + `.errors.md`). Must produce errors matching the expected codes/sections/keys.

The `.errors.md` files specify expected error properties. The test matcher checks each table row in the actual output for the expected fields.

**Run all tests:** `cargo test`

**Run compliance only:** `cargo test -p structmd-lint --test compliance`

## Design decisions

**Productions are composition. No import system.** Each schema is self-contained. If two schemas need the same sub-structure, they define the same production. No parse-time dependencies, no resolution order, no circular reference problem. The compliance test suite catches drift.

**Prose is always captured by the parser.** The schema's `prose` token controls whether it's *required*, but the parser always collects it. Consumers decide what to do with it.

**Scoped properties win over globals.** If a section has `properties: Some(map)` from a tagged types block, those are its only properties. No merging with the global set.

**Error output defaults to dense markdown tables.** One row per error, auto-selected columns. Verbose section-per-error format available via `render_verbose()`. Callers pick based on context.

**Fix text is auto-derived from error code.** The code → fix mapping lives in `ErrorFormatter::derive_fix()`. Override with `f.fix("...")` when the auto text isn't right.

**The `Diagnostic` trait separates error data from error rendering.** Error enums carry the facts (line, section, key, got). The formatter handles presentation. The enum travels up the `?` chain intact. Rendering happens once at the top.

## How weyd uses structmd

Weyd includes structmd as a git submodule at `deps/structmd/`. Services depend on `structmd` and `structmd-errors` via path deps into the submodule.

Five services use structmd for config loading: sozu, ungyo, agyo, model-orchestrator, kannagi. Sozu also uses `structmd_errors::Diagnostic` for structured error reporting.

Config files: `config/*.conf.md` and `config/tools.md`
Schemas: `schemas/*.schema.conf.md`
Error schema: `schemas/errors.schema.conf.md`

## Known gaps and future work

- **BNF alternation (`|`)** — not implemented. Needed for heterogeneous collections (e.g., MCP content blocks). The syntax is designed but not built.
- **Prose text capture is bare** — the parser collects prose as concatenated lines. Blank lines within prose are lost. Multi-paragraph prose becomes one run-on string.
- **No cross-field validation** — e.g., "if `type` is `array`, `items` must be present." The linter validates fields independently.
- **codegen doesn't handle H1-level properties fully** — fixed for orchestrator schema but edge cases may remain.
- **error-handling.md confmd references** — audit for any remaining confmd mentions in the error handling doc.
- **cron validation is basic** — checks format (5 fields or `every Nm/Nh`) but doesn't validate field ranges.
- **Derive macro for Diagnostic** — `#[derive(Diagnostic)]` would eliminate the hand-written match arms. Not built yet.
- **No parsers in other languages** — compliance fixtures are language-agnostic and ready for a Python/TypeScript/Go implementation.
- **crates.io publication** — `structmd` name is available. Add `version` alongside `path` deps when ready.
