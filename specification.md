# structmd Specification

Version: 0 (pre-release)
Copyright Susan Roylance and Stephen Roylance. MIT licensed.

---

structmd is a structured document format. structmd documents are not markdown documents that happen to have structure — they are structured documents that happen to be readable as markdown. A markdown renderer will display them sensibly, but the renderer is not the authority on what they mean. The schema is.

The format is defined by this specification and by the compliance test suite in `tests/fixtures/`. Where this document and the compliance suite disagree, the compliance suite is normative. The spec describes the suite; if they conflict, the spec has a bug.

---

## 1. Lexical Structure

A structmd document is a sequence of UTF-8 lines. The parser is line-oriented and single-pass. Each line is classified by its prefix before any structural nesting is applied.

### 1.1 Line types

| Line type | Recognition rule |
|-----------|-----------------|
| H1 | Starts with exactly `# ` (hash + space) |
| H2 | Starts with exactly `## ` (two hashes + space) |
| H3 | Starts with exactly `### ` (three hashes + space) |
| Property (plain) | Starts with `- ` and contains `: ` or ends with `:` |
| Property (bold) | Starts with `- **` and contains `:**` |
| Code fence open | Starts with ` ``` ` followed by an info string (non-empty tag) |
| Code fence close | Is exactly ` ``` ` (backticks only, no tag) |
| Code fence open (untagged) | Starts with ` ``` ` with no following text |
| Table row | Starts with `|` |
| Table separator | Starts with `|` and contains only `-`, `|`, and spaces |
| Blank | Empty or whitespace only |
| Prose | Anything else |

Heading text is everything after the leading `# `, `## `, or `### ` prefix, trimmed of trailing whitespace.

### 1.2 Property syntax

Plain property:
```
- key: value
- key:
```

Bold property (reserved, currently equivalent to plain):
```
- **key:** value
- **key:**
```

The key is everything between the prefix and the first `: `. The value is everything after `: `, trimmed. A property with no value (ends with `:`) has an empty string value. Both plain and bold forms are recognized. The `bold` flag is preserved in the AST and reserved for future semantic use. Implementors MUST NOT assign semantics to the bold flag — treat bold and plain properties as equivalent until a future version of this specification says otherwise. A version marker will accompany any change that gives the bold flag meaning.

### 1.3 Code fences

A tagged code fence opens with `` ```tag `` and closes with `` ``` `` on a line by itself. The tag is the info string — everything after the opening backticks, trimmed. The content is all lines between open and close, joined with newlines, not including the fence lines themselves. Untagged code fences (`` ``` `` with no tag) are skipped entirely — their content is discarded.

### 1.4 Tables

A table begins with the first `|`-prefixed line after a heading or other structural element. It continues until a non-table line is encountered. The first row is the header. A separator row (containing only `-`, `|`, and spaces) is recognized and skipped; it does not become a data row. Each row's cells are the strings between `|` characters, trimmed.

---

## 2. Document Model

The document AST is a tree:

```
Document
  H1Node+
    heading: string
    prose: string
    properties: Property[]
    sections: Section[]

Section  (H2 or H3)
  heading: string
  level: 2 | 3
  prose: string
  properties: Property[]
  table: Table | null
  children: Section[]   (H3 only, nested under H2)

Property
  key: string
  value: string
  bold: bool
  line: integer

Table
  columns: string[]
  rows: string[][]
  header_line: integer
```

### 2.1 Building the tree

Processing is a single forward pass:

1. An H1 line opens a new `H1Node`. Any prior H1Node is closed.
2. An H2 line opens a new `Section` at level 2 under the current H1Node. Any prior H2 section is closed.
3. An H3 line opens a new `Section` at level 3 nested under the current H2 section. Any prior H3 section is closed.
4. Properties, tables, and tagged code fences are attached to the innermost open section (H3 if open, else H2, else H1).
5. A tagged code fence is stored as a `Property` with `key = tag` and `value = block content`. This is intentional: tagged fences and list-item properties occupy the same namespace in the section's property list. A schema addresses both with `@key`. On collision — a fence tag and a list-item key with the same name in the same section — the fence wins; the list-item property is discarded. Schema authors should avoid names that are valid fence tags if they also expect list-item properties of the same name.
6. Prose lines are captured into the current section's `prose` field. A prose run begins after the heading and ends at the first property, table row, code fence, or child heading. Blank lines within prose are preserved as paragraph breaks (`\n\n` in the captured string). Leading and trailing blank lines are stripped. Prose ends only at a structural element or end of document — not at a blank line.
7. Multiple H1 headings produce multiple `H1Node` entries in the document. This is valid.
8. H1 nodes may have their own properties (appearing before any H2).

### 2.2 Prose capture

Prose is everything between a heading and the first structural element (property, table, code fence, or child heading). Blank lines within prose are preserved — they become `\n` in the captured string, producing paragraph breaks. Leading and trailing blank lines are stripped. The prose field contains the full text a human would read between the heading and the first structural element, with internal paragraph structure intact.

---

## 3. Schema Format

A schema is itself a structmd document. The parser reads it as a document and extracts specific tagged code blocks. Everything else (prose, headings without code blocks) is documentation and is ignored.

### 3.1 Block types

| Tag | Meaning |
|-----|---------|
| `` ```grammar `` | Structural grammar — required, at least one |
| `` ```types `` | Global property type constraints |
| `` ```types:production `` | Property type constraints scoped to a named production |
| `` ```table `` | Global table column constraints |
| `` ```table:production `` | Table column constraints scoped to a named production |

A schema with no `grammar` block is invalid.

### 3.2 Scoped vs global constraints

A `types:production` block defines property constraints for sections whose `production_name` matches. A global `types` block applies to all sections that expect properties but have no scoped type block. Scoped properties do not merge with global properties — if a section has a scoped type block, those are its only constraints.

---

## 4. Grammar Notation

The structmd grammar notation describes the structural shape of a document: which headings appear, in what order, with what content.

### 4.1 Productions

A grammar block contains one or more productions:

```
production-name ::= token token token ...
```

Production names are lowercase identifiers. The production named `document` is the entry point. If no `document` production exists, the first production is the entry point.

### 4.2 Terminals

| Token | Meaning |
|-------|---------|
| `H1` | An H1 heading, any text |
| `H1("text")` | An H1 heading with exact text (case-sensitive) |
| `H2` | An H2 heading, any text |
| `H2("text")` | An H2 heading with exact text |
| `H2(IDENTIFIER)` | An H2 heading whose text matches `[a-zA-Z0-9_-]+` |
| `H2(SNAKE_CASE)` | An H2 heading whose text matches `[a-z][a-z0-9_]*` |
| `H3` | An H3 heading, any text |
| `H3("text")` | An H3 heading with exact text |
| `H3(IDENTIFIER)` | An H3 heading matching `[a-zA-Z0-9_-]+` |
| `H3(SNAKE_CASE)` | An H3 heading matching `[a-z][a-z0-9_]*` |
| `prose` | Expects prose text under this heading |
| `property` | Expects one or more `- key: value` lines |
| `table` | Expects a markdown table |

`IDENT` is a synonym for `IDENTIFIER`.

### 4.3 Quantifiers

Quantifiers follow a token or named reference:

| Quantifier | Meaning |
|------------|---------|
| (none) | Exactly one |
| `+` | One or more |
| `*` | Zero or more |
| `?` | Optional (zero or one) |

### 4.4 Named references

A token that is not a terminal keyword is a named reference — it resolves to the production with that name. The quantifier on the reference propagates to the instantiated production. Named references may not be circular; circular references are an error.

### 4.5 Resolution rules

The resolver walks the entry production and inlines referenced productions recursively:

- H1 tokens produce `H1Node` entries.
- H2 tokens produce `Section` entries at level 2 under the current H1.
- H3 tokens produce `Section` entries at level 3 under the current H2.
- `prose`, `property`, `table` tokens set flags on the enclosing section (`expects_prose`, `expects_properties`, `expects_table`).
- A reference with a quantifier of `+` or `*` produces a `Vec` of the referenced structure.
- A reference with `?` produces an optional structure.

---

## 5. Type System

Property type constraints appear in `types` and `types:production` blocks. Each line defines one property:

```
@key : type
@key : type, modifier
@key : type  # inline comment
```

The `@` prefix is literal. The `#` character begins an inline comment; everything from `#` to end of line is stripped before parsing.

### 5.1 Types

| Type | Meaning |
|------|---------|
| `string` | Non-empty string |
| `text` | Any string, including empty |
| `bool` | `true` or `false` (case-insensitive) |
| `integer` | Signed 64-bit integer |
| `number` | 64-bit floating point |
| `label` | Property name only; value is not validated (see below) |
| `enum(a, b, c)` | One of the listed values (case-sensitive) |
| `list(T)` | Comma-separated values, each validated as type T. Values may not contain commas. |

The `label` type is for properties whose key is significant but whose value is unstructured or schema-defined at a higher level — for example, a property whose key names a parameter and whose value is a freeform description. The linter validates that the property is present but does not validate its value.

### 5.2 Modifiers

| Modifier | Meaning |
|----------|---------|
| `required` | Error if property is absent |
| `default("value")` | Use this value if property is absent |

A property with neither modifier is optional with no default. `required` and `default` are mutually exclusive.

### 5.3 Table column constraints

`table` and `table:production` blocks use the same syntax as `types` blocks. Each line defines one column by name. Columns are matched by position in the table header, not by name search.

---

## 6. Validation

The validator walks the Document AST against the Schema tree recursively.

### 6.1 Structural checks

- **H1 presence**: If the schema requires an H1 with exact text, the document must have an H1 with that text.
- **Section count**: Sections must satisfy the quantifier on their production (`+` = at least one, `*` = zero or more, `?` = zero or one, none = exactly one).
- **Name pattern**: If the schema specifies `IDENTIFIER` or `SNAKE_CASE`, the section heading must match.
- **Prose**: If the schema specifies `prose`, the section must have non-empty prose text.
- **Table**: If the schema specifies `table`, the section must have a table.

### 6.2 Property checks

For each property defined in the applicable types block:

- If `required` and absent: `missing_property` error.
- If present: validate value against type. Type mismatch: `invalid_value` error.

Unknown properties (present in document, not in schema) are ignored.

### 6.3 Table checks

- **Column count**: The table must have at least as many columns as the schema defines.
- **Column names**: Each schema column is matched by position. If the header cell at that position does not match the expected name: `column_mismatch` error.
- **Row count**: If the schema defines any columns, the table must have at least one data row.
- **Cell values**: Each cell in a schema-defined column is validated against the column's type.

### 6.4 Error codes

| Code | Meaning |
|------|---------|
| `missing_property` | Required property absent |
| `missing_section` | Required section absent |
| `missing_prose` | Section requires prose but has none |
| `missing_table` | Section requires table but has none |
| `invalid_value` | Property or cell value fails type check |
| `invalid_name` | Section heading does not match required pattern |
| `column_mismatch` | Table column name does not match schema |
| `row_count` | Table has no data rows |
| `section_count` | Wrong number of sections |
| `io_error` | File could not be read |

---

## 7. Linter Output

A conformant linter MUST write its output to stdout. Stderr is reserved for fatal errors that prevent validation from running (e.g., file not found, schema parse failure).

On success (zero errors), a conformant linter MUST exit with status 0 and produce no output.

On failure, a conformant linter MUST exit with a non-zero status and output errors as a structmd document conforming to the errors schema. The minimum required output is a markdown table with one row per error, containing at minimum the columns `code`, `file`, and `line`. Additional columns (`section`, `key`, `got`, `fix`) SHOULD be included when the relevant data is available.

The output document is itself valid structmd and may be parsed, validated, or piped to another tool. Agents consuming linter output SHOULD parse it as a structmd document rather than processing it as plain text.

A linter that produces output only to stderr, or that produces unstructured plain text, does not conform to this specification.

---

## 8. Compliance Suite

The `tests/fixtures/` directory contains language-agnostic compliance fixtures.

`valid/` — each fixture is a pair of files:
- `name.md` — a structmd document
- `name.schema.md` — its schema

A conformant implementation must validate each valid fixture with zero errors.

`invalid/` — each fixture is a triple:
- `name.md` — a structmd document
- `name.schema.md` — its schema
- `name.errors.md` — expected errors

A conformant implementation must produce at least the errors listed in the `.errors.md` file. The `.errors.md` file is itself a structmd document: each H2 section describes one expected error with properties `code`, `section`, `key`, and `got`. An implementation passes if every expected error appears in its output (matched on whichever fields are present). Extra errors beyond those expected are allowed.

---

## 9. Provenance

structmd began as a solution to a practical problem — structured config files that agents could read and write reliably — before it had a specification or a compliance suite. The implementation came first, built iteratively with AI assistance to solve real problems in a real system. The architecture emerged from using it. The compliance suite was extracted from behavior that already worked. The specification was written last, from the compliance suite, as a description of what the system had become.

This is backwards from how format specifications are usually written. The normal order is: specify, implement, test. Here the order was: use, observe, specify. That inversion was only possible because the implementation cost was low enough to iterate freely — to let the design reveal itself through use rather than planning it in advance.

The reference implementation is an artifact of that process, not a ground-up construction from the spec. Where the spec and the implementation disagree, the compliance suite is the arbiter.
