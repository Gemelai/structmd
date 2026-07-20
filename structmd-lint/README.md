# structmd-lint

Schema-driven linter for [structmd](https://crates.io/crates/structmd) — a structured document format whose syntax is borrowed from markdown.

Validates a structmd document against a schema and prints errors as a structmd table: readable by a human, parseable by an agent or another process.

## Install

```sh
cargo install structmd-lint
```

## Usage

```sh
structmd-lint --schema config.schema.md config.md
```

Exit codes:

- `0` — the document validates against the schema
- `1` — validation errors (printed to stdout as a structmd error table)
- `2` — usage error (unreadable file, invalid schema)

## Example

Given a schema:

    # Server Config Schema

    ```grammar
    document ::= H1("Servers") server+
    server   ::= H2(IDENTIFIER) property+
    ```

    ```types
    @command : string, required
    @timeout : integer, default("60")
    ```

and a document that omits `command`, the linter prints an error table with the code, section, line, and a fix suggestion (`add ` + the missing property line), which a model can apply and retry without human intervention.

See the [structmd repository](https://github.com/Gemelai/structmd) for the format specification and the compliance test suite.

## License

MIT. Copyright Susan Roylance and Stephen Roylance.
