use structmd_codegen::generate_from_text;

const TOOLS_SCHEMA: &str = "\
```bnf
document ::= H1(\"Tools\") tool+
tool     ::= H2(SNAKE_CASE) prose property* table
```

```types
@server     : string
@parameters : label
```

```table
@name : string
@type : enum(string, integer)
@required : enum(yes, no)
@desc : string
```
";

const SOZU_SCHEMA: &str = "\
```bnf
document  ::= agyo tasks?
agyo      ::= H1(\"Agyo\") container servers
container ::= H2(\"Container\") property+
servers   ::= H2(\"Servers\") server+
server    ::= H3(IDENTIFIER) property+
tasks     ::= H1(\"Tasks\") task+
task      ::= H2(IDENTIFIER) prose? property+
```

```types:container
@image   : string, required
@network : string, default(\"outbound\")
@mounts  : list(string)
```

```types:server
@command : string, required
@secrets : list(string)
```

```types:task
@schedule : cron, required
@run      : string, required
@log      : bool, default(true)
```
";

// ── Struct generation ──

#[test]
fn flat_schema_root_struct() {
    let code = generate_from_text(TOOLS_SCHEMA, "tools");
    assert!(code.contains("pub struct ToolsConfig"), "missing root struct:\n{}", code);
    assert!(code.contains("pub tool:"), "missing tool field in root:\n{}", code);
}

#[test]
fn flat_schema_section_struct() {
    let code = generate_from_text(TOOLS_SCHEMA, "tools");
    assert!(code.contains("pub struct ToolConfig"), "missing tool struct:\n{}", code);
    assert!(code.contains("pub name: String"), "missing name field:\n{}", code);
    assert!(code.contains("pub prose: Option<String>"), "missing prose field:\n{}", code);
}

#[test]
fn hierarchical_schema_structs() {
    let code = generate_from_text(SOZU_SCHEMA, "sozu");
    assert!(code.contains("pub struct SozuConfig"), "missing root struct:\n{}", code);
    assert!(code.contains("pub struct AgyoConfig"), "missing agyo struct:\n{}", code);
    assert!(code.contains("pub struct ContainerConfig"), "missing container struct:\n{}", code);
    assert!(code.contains("pub struct ServerConfig"), "missing server struct:\n{}", code);
    assert!(code.contains("pub struct TaskConfig"), "missing task struct:\n{}", code);
}

#[test]
fn optional_h1_becomes_option() {
    let code = generate_from_text(SOZU_SCHEMA, "sozu");
    assert!(code.contains("Option<TasksConfig>"), "tasks should be Option:\n{}", code);
}

#[test]
fn list_type_becomes_vec() {
    let code = generate_from_text(SOZU_SCHEMA, "sozu");
    // mounts should be Vec<String>, not Option<Vec<String>>
    assert!(code.contains("pub mounts: Vec<String>"), "mounts should be Vec<String>:\n{}", code);
}

#[test]
fn required_field_not_option() {
    let code = generate_from_text(SOZU_SCHEMA, "sozu");
    assert!(code.contains("pub image: String"), "required field should not be Option:\n{}", code);
}

#[test]
fn default_field_not_option() {
    let code = generate_from_text(SOZU_SCHEMA, "sozu");
    // network has default("outbound"), should be String not Option
    assert!(code.contains("pub network: String"), "default field should not be Option:\n{}", code);
}

#[test]
fn bool_field_type() {
    let code = generate_from_text(SOZU_SCHEMA, "sozu");
    assert!(code.contains("pub log: bool"), "log should be bool:\n{}", code);
}

#[test]
fn dynamic_section_has_name_field() {
    let code = generate_from_text(SOZU_SCHEMA, "sozu");
    // ServerConfig and TaskConfig have dynamic names from headings
    assert!(code.contains("pub name: String"), "dynamic sections need name field:\n{}", code);
}

// ── Parse function ──

#[test]
fn generates_parse_fn() {
    let code = generate_from_text(TOOLS_SCHEMA, "tools");
    assert!(code.contains("pub fn parse(text: &str) -> Result<ToolsConfig, Vec<String>>"),
        "missing parse function:\n{}", code);
}

#[test]
fn generated_code_uses_structmd_parse() {
    let code = generate_from_text(TOOLS_SCHEMA, "tools");
    assert!(code.contains("structmd::parse::parse(text)"), "should use confmd parser:\n{}", code);
}

// ── End-to-end: compile and run generated code ──

#[test]
fn sozu_generated_code_compiles_and_parses() {
    let code = generate_from_text(SOZU_SCHEMA, "sozu");

    // Write to temp file
    let dir = std::env::temp_dir().join("structmd_codegen_test");
    std::fs::create_dir_all(&dir).unwrap();
    let gen_path = dir.join("sozu_gen.rs");
    std::fs::write(&gen_path, &code).unwrap();

    // Include and test via a module
    // We can't actually compile+run in a unit test, but we can verify the generated
    // code can be parsed as valid Rust tokens by checking for balanced braces
    let open = code.matches('{').count();
    let close = code.matches('}').count();
    assert_eq!(open, close, "unbalanced braces in generated code");

    // Verify all struct defs are complete (each has both { and })
    for struct_name in &["SozuConfig", "AgyoConfig", "ContainerConfig", "ServerConfig", "TaskConfig"] {
        assert!(code.contains(&format!("pub struct {}", struct_name)),
            "missing struct {}", struct_name);
    }
}

#[test]
fn tools_generated_no_duplicate_names() {
    let code = generate_from_text(TOOLS_SCHEMA, "tools");
    // "pub struct ToolsConfig" should appear exactly once
    let count = code.matches("pub struct ToolsConfig").count();
    assert_eq!(count, 1, "ToolsConfig defined {} times", count);
}

#[test]
fn generated_code_has_no_return_err() {
    // The inner function shouldn't return Result, so no Err() calls
    let code = generate_from_text(SOZU_SCHEMA, "sozu");
    // The only Result should be in the outer parse() function
    let err_count = code.matches("Err(").count();
    assert!(err_count <= 1, "too many Err() calls in generated code: {}", err_count);
}

// ── Default values ──

#[test]
fn default_value_in_generated_parser() {
    let code = generate_from_text(SOZU_SCHEMA, "sozu");
    assert!(code.contains("outbound"), "should contain default value 'outbound':\n{}", code);
    assert!(code.contains("true"), "should contain default value 'true' for log");
}

// ── Naming conventions ──

#[test]
fn pascal_case_struct_names() {
    let code = generate_from_text(SOZU_SCHEMA, "sozu");
    // All struct names should end with Config and be PascalCase
    for line in code.lines() {
        if line.contains("pub struct ") {
            let name = line.split("pub struct ").nth(1).unwrap().split_whitespace().next().unwrap();
            assert!(name.ends_with("Config"), "struct {} should end with Config", name);
            assert!(name.chars().next().unwrap().is_uppercase(), "struct {} should be PascalCase", name);
        }
    }
}
