# Error Handling with structmd

How to make your Rust program report errors as structured markdown — readable by humans, parseable by agents.

## The short version

1. Define an enum with one variant per error kind
2. Put `?` at your failure sites — the compiler tells you what to handle
3. Implement `Diagnostic` on your enum — one match arm per variant
4. Call `structmd::errors::render()` in main

Your program's error output becomes structured markdown that an agent can read, locate, and fix.

## Step by step

### 1. Define your error enum

Think about what can go wrong in your program. Each kind of failure is a variant. The fields are the facts about what happened — not what to do about it.

```rust
use std::path::PathBuf;

enum MyError {
    // A file operation failed
    Io { path: PathBuf, source: std::io::Error },

    // A required config property was missing
    MissingProperty { line: usize, section: String, key: String },

    // A config value was the wrong type or out of range
    InvalidValue { line: usize, section: String, key: String, got: String },
}
```

Don't overthink it. Start with 2-3 variants. Add more when the compiler tells you a new failure doesn't fit.

### 2. Use `?` at failure sites

The `?` operator sends errors up the call stack. Your function just needs to return `Result<T, MyError>`:

```rust
fn load_config(path: &std::path::Path) -> Result<Config, MyError> {
    // If this fails, ? converts io::Error into MyError::Io
    let text = std::fs::read_to_string(path)
        .map_err(|e| MyError::Io { path: path.into(), source: e })?;

    let doc = structmd::parse::parse(&text);

    // If the property is missing, ? sends MyError::MissingProperty up
    let name = doc.nodes[0].sections[0]
        .properties.iter()
        .find(|p| p.key == "name")
        .map(|p| p.value.clone())
        .ok_or(MyError::MissingProperty {
            line: doc.nodes[0].sections[0].heading.line,
            section: "item".into(),
            key: "name".into(),
        })?;

    Ok(Config { name })
}
```

The compiler forces you to handle every `Result`. If you use `?`, you must provide a variant. If no variant fits, add one to the enum.

### 3. Implement `Diagnostic`

This tells confmd how to render your error as conf.md. One match arm per variant. Write the code, write the fields:

```rust
impl structmd_errors::Diagnostic for MyError {
    fn render(&self, f: &mut structmd::ErrorFormatter) {
        match self {
            MyError::Io { path, source } => {
                f.code("io_error");
                f.field("file", &path.display().to_string());
                f.field("got", &source.to_string());
            }
            MyError::MissingProperty { line, section, key } => {
                f.code("missing_property");
                f.line(*line);
                f.field("section", section);
                f.field("key", key);
            }
            MyError::InvalidValue { line, section, key, got } => {
                f.code("invalid_value");
                f.line(*line);
                f.field("section", section);
                f.field("key", key);
                f.field("got", got);
            }
        }
    }
}
```

The method names map to conf.md properties:
- `f.code("io_error")` → `- code: io_error`
- `f.line(3)` → `- line: 3`
- `f.field("key", "image")` → `- key: image`

The formatter adds the fix text automatically based on the code. You don't write fix strings.

### 4. Render in main

```rust
fn main() {
    if let Err(e) = run() {
        eprint!("{}", structmd::errors::render("my-program", &e));
        std::process::exit(1);
    }
}

fn run() -> Result<(), MyError> {
    let config = load_config(std::path::Path::new("config.conf.md"))?;
    // ... do stuff with config ...
    Ok(())
}
```

That's it. When the program fails, the output looks like:

```markdown
# my-program

1 error(s)

## error-1

- file: config.conf.md
- line: 3
- section: item
- code: missing_property
- key: name
- fix: add `- name: <value>`
```

An agent reading that knows: read line 3 of config.conf.md, add `- name: <value>` to the `item` section. A human reading it sees the same thing, formatted as readable markdown.

## On large error enums

A large error enum is a measurement, not a problem. It tells you something true about your program that was always true — you just couldn't see it before.

With `anyhow` you have 100 failure modes too. They're dissolved into strings scattered across the codebase. The enum didn't create the complexity, it surfaced it. And once it's surfaced you can reason about it: 40 variants all related to config loading might be a signal that config loading should be its own subsystem with its own error type. The enum is a forcing function for that conversation. `anyhow` lets you avoid the conversation indefinitely.

Some programs are genuinely complex and a large enum reflects that honestly. A multi-process orchestrator has many things that can go wrong, and pretending otherwise with loose error strings doesn't reduce the complexity — it just hides it until 2am.

## Adding new error kinds

When you hit a new failure case and no existing variant fits:

1. Add a variant to your enum with the relevant fields
2. Add a match arm to your `render` impl
3. The compiler will tell you if you missed either step

```rust
// New variant
enum MyError {
    // ... existing variants ...

    // A scheduled task has an invalid cron expression
    InvalidSchedule { line: usize, section: String, got: String },
}

// New match arm
MyError::InvalidSchedule { line, section, got } => {
    f.code("invalid_value");
    f.line(*line);
    f.field("section", section);
    f.field("key", "schedule");
    f.field("got", got);
}
```

## Multiple errors

If your program can detect multiple errors before stopping (like a linter), collect them:

```rust
fn validate(doc: &Document) -> Result<(), Vec<MyError>> {
    let mut errors = Vec::new();

    // check things, push errors...
    if image.is_none() {
        errors.push(MyError::MissingProperty { ... });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
```

Render them all:

```rust
fn main() {
    if let Err(errors) = run() {
        eprint!("{}", structmd::errors::render_all("my-program", &errors));
        std::process::exit(1);
    }
}
```

## Error codes

The error codes come from the structured markdown error schema (`schemas/errors.schema.conf.md`). Use the standard codes when they fit:

| Code | Meaning |
|------|---------|
| `io_error` | File system or network operation failed |
| `missing_property` | Required property not present |
| `missing_section` | Required section heading not present |
| `missing_prose` | Required description text not present |
| `missing_table` | Required table not present |
| `invalid_value` | Value doesn't match expected type or range |
| `invalid_name` | Heading name doesn't match expected pattern |
| `column_mismatch` | Table column name or count is wrong |
| `row_count` | Table has too few rows |
| `section_count` | Wrong number of sections |

If none of these fit, use a descriptive snake_case code. The schema can be extended.

## Fix text

The formatter derives fix text from the error code and fields. You don't write it:

| Code | Fix (auto-generated) |
|------|---------------------|
| `missing_property` with key `image` | `add \`- image: <value>\`` |
| `invalid_value` with key `log`, expected `bool` | `change \`log\` to \`true\` or \`false\`` |
| `io_error` | `check that the file exists and is readable` |

If the auto-generated fix isn't right for your case, override it:

```rust
f.code("invalid_value");
f.field("key", "schedule");
f.field("got", &got);
f.fix("use `every 5m`, `every 2h`, or a 5-field cron expression");
```
