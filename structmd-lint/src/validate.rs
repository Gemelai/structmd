use structmd::errors::Error as LintError;
use structmd::parse::{Document, H1Node, Section};
use structmd::schema::{H1Schema, Quantity, Schema, SectionSchema, ValueType};

fn type_label(vt: &ValueType) -> &'static str {
    match vt {
        ValueType::String => "string",
        ValueType::Bool => "bool",
        ValueType::Enum(_) => "enum",
        ValueType::Path => "path",
        ValueType::Integer => "integer",
        ValueType::Number => "number",
        ValueType::Text => "text",
        ValueType::Label => "label",
        ValueType::List(_) => "list",
        ValueType::Cron => "cron",
    }
}

pub fn validate(doc: &Document, schema: &Schema, file: &str) -> Vec<LintError> {
    let mut errors = Vec::new();

    let mut doc_idx = 0;
    for h1_schema in &schema.nodes {
        if let Some(ref expected_text) = h1_schema.text {
            let found = doc.nodes[doc_idx..].iter().enumerate().find(|(_, n)| {
                n.heading
                    .as_ref()
                    .map_or(false, |h| h.text == *expected_text)
            });
            match found {
                Some((offset, node)) => {
                    doc_idx += offset + 1;
                    validate_h1_children(node, h1_schema, schema, file, &mut errors);
                }
                None => match h1_schema.quantity {
                    Quantity::Optional | Quantity::ZeroOrMore => {}
                    _ => {
                        errors.push(
                            LintError::new(file,1, "", "missing_section")
                                .with_expected(&format!("# {}", expected_text))
                                .with_fix(&format!("add `# {}` section", expected_text)),
                        );
                    }
                },
            }
        } else {
            if let Some(node) = doc.nodes.get(doc_idx) {
                doc_idx += 1;
                validate_h1_children(node, h1_schema, schema, file, &mut errors);
            }
        }
    }

    errors.sort_by_key(|e| e.line);
    errors
}

fn validate_h1_children(
    doc_node: &H1Node,
    h1_schema: &H1Schema,
    schema: &Schema,
    file: &str,
    errors: &mut Vec<LintError>,
) {
    validate_sections(&doc_node.sections, &h1_schema.children, schema, file, errors);
}

fn validate_sections(
    doc_sections: &[Section],
    schema_sections: &[SectionSchema],
    schema: &Schema,
    file: &str,
    errors: &mut Vec<LintError>,
) {
    for sec_schema in schema_sections {
        let matching: Vec<&Section> = doc_sections
            .iter()
            .filter(|s| {
                s.heading.level == sec_schema.level
                    && sec_schema.name_pattern.matches(&s.heading.text)
            })
            .collect();

        let count = matching.len();

        match sec_schema.quantity {
            Quantity::One if count != 1 => {
                let name = sec_schema
                    .production_name
                    .as_deref()
                    .unwrap_or("section");
                errors.push(
                    LintError::new(file,1, name, "section_count")
                        .with_got(&count.to_string())
                        .with_expected("1")
                        .with_fix(&format!("need exactly 1 H{} `{}` section", sec_schema.level, name)),
                );
            }
            Quantity::OneOrMore if count == 0 => {
                let name = sec_schema
                    .production_name
                    .as_deref()
                    .unwrap_or("section");
                errors.push(
                    LintError::new(file,1, name, "section_count")
                        .with_got("0")
                        .with_expected("at least 1")
                        .with_fix(&format!(
                            "add at least 1 H{} section matching `{}`",
                            sec_schema.level,
                            sec_schema.name_pattern.label()
                        )),
                );
            }
            _ => {}
        }

        for sec in &matching {
            validate_section(sec, sec_schema, schema, file, errors);
        }
    }
}

fn validate_section(
    sec: &Section,
    sec_schema: &SectionSchema,
    schema: &Schema,
    file: &str,
    errors: &mut Vec<LintError>,
) {
    let name = &sec.heading.text;
    let lineno = sec.heading.line;

    if name.is_empty() {
        errors.push(
            LintError::new(file,lineno, "", "invalid_name")
                .with_got("(empty)")
                .with_fix(&format!("add a name after `{}`", "#".repeat(sec.heading.level as usize))),
        );
    }

    // Prose
    if sec_schema.expects_prose && sec.prose.is_none() {
        errors.push(
            LintError::new(file,lineno, name, "missing_prose")
                .with_fix(&format!("add text after `## {}`", name)),
        );
    }

    // Properties
    if sec_schema.expects_properties {
        let props = sec_schema
            .properties
            .as_ref()
            .unwrap_or(&schema.global_properties);

        for (key, prop_schema) in props {
            let found = sec.properties.iter().find(|p| p.key == *key);
            match found {
                None if prop_schema.required => {
                    errors.push(
                        LintError::new(file,lineno, name, "missing_property")
                            .with_key(key)
                            .with_expected(type_label(&prop_schema.value_type))
                            .with_fix(&format!("add `- {}: <{}>`", key, type_label(&prop_schema.value_type))),
                    );
                }
                Some(prop) => {
                    validate_value(
                        &prop.value,
                        &prop_schema.value_type,
                        prop.line,
                        name,
                        key,
                        file,
                        errors,
                    );
                }
                _ => {}
            }
        }
    }

    // Table
    if sec_schema.expects_table {
        let table_schema = sec_schema.table.as_ref().or(schema.global_table.as_ref());

        match (&sec.table, table_schema) {
            (None, Some(tschema)) => {
                let cols: Vec<&str> = tschema.columns.iter().map(|c| c.name.as_str()).collect();
                errors.push(
                    LintError::new(file,lineno, name, "missing_table")
                        .with_expected(&cols.join(", "))
                        .with_fix(&format!("add table with columns: {}", cols.join(", "))),
                );
            }
            (Some(table), Some(tschema)) => {
                for (i, col_schema) in tschema.columns.iter().enumerate() {
                    match table.columns.get(i) {
                        Some(got)
                            if got.to_lowercase().as_str()
                                != col_schema.name.to_lowercase().as_str() =>
                        {
                            errors.push(
                                LintError::new(file,table.header_line, name, "column_mismatch")
                                    .with_key(&format!("column {}", i + 1))
                                    .with_got(got)
                                    .with_expected(&col_schema.name)
                                    .with_fix(&format!("rename column {} to `{}`", i + 1, col_schema.name)),
                            );
                        }
                        None => {
                            errors.push(
                                LintError::new(file,table.header_line, name, "column_mismatch")
                                    .with_key(&format!("column {}", i + 1))
                                    .with_expected(&col_schema.name)
                                    .with_fix(&format!("add column `{}`", col_schema.name)),
                            );
                        }
                        _ => {}
                    }
                }

                if table.rows.is_empty() {
                    errors.push(
                        LintError::new(file,table.header_line, name, "row_count")
                            .with_got("0")
                            .with_expected("at least 1")
                            .with_fix("add at least one data row to the table"),
                    );
                }

                let col_count = table.columns.len();
                for row in &table.rows {
                    if row.cells.len() != col_count {
                        errors.push(
                            LintError::new(file,row.line, name, "column_mismatch")
                                .with_got(&row.cells.len().to_string())
                                .with_expected(&col_count.to_string())
                                .with_fix(&format!("row should have {} columns", col_count)),
                        );
                    }

                    for (i, col_schema) in tschema.columns.iter().enumerate() {
                        if let Some(cell) = row.cells.get(i) {
                            validate_value(
                                cell,
                                &col_schema.column_type,
                                row.line,
                                name,
                                &col_schema.name,
                                file,
                                errors,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Recurse into children
    if !sec_schema.children.is_empty() {
        validate_sections(&sec.children, &sec_schema.children, schema, file, errors);
    }
}

fn validate_value(
    value: &str,
    vtype: &ValueType,
    line: usize,
    section: &str,
    field: &str,
    file: &str,
    errors: &mut Vec<LintError>,
) {
    match vtype {
        ValueType::Label | ValueType::Text => {}
        ValueType::String => {
            if value.is_empty() {
                errors.push(
                    LintError::new(file,line, section, "invalid_value")
                        .with_key(field)
                        .with_got("(empty)")
                        .with_expected("non-empty string")
                        .with_fix(&format!("set `- {}: <value>`", field)),
                );
            }
        }
        ValueType::Bool => {
            if value != "true" && value != "false" {
                errors.push(
                    LintError::new(file,line, section, "invalid_value")
                        .with_key(field)
                        .with_got(value)
                        .with_expected("true or false")
                        .with_fix(&format!("change `{}` to `true` or `false`", field)),
                );
            }
        }
        ValueType::Enum(vals) => {
            if !vals.iter().any(|v| v == value) {
                errors.push(
                    LintError::new(file,line, section, "invalid_value")
                        .with_key(field)
                        .with_got(value)
                        .with_expected(&vals.join(", "))
                        .with_fix(&format!("change `{}` to one of: {}", field, vals.join(", "))),
                );
            }
        }
        ValueType::Path => {
            if !value.starts_with('/') && !value.starts_with("./") && !value.starts_with("../") {
                errors.push(
                    LintError::new(file,line, section, "invalid_value")
                        .with_key(field)
                        .with_got(value)
                        .with_expected("path starting with / or ./")
                        .with_fix(&format!("change `{}` to a path (e.g. `/path/to/thing`)", field)),
                );
            }
        }
        ValueType::Integer => {
            if value.parse::<i64>().is_err() {
                errors.push(
                    LintError::new(file,line, section, "invalid_value")
                        .with_key(field)
                        .with_got(value)
                        .with_expected("integer")
                        .with_fix(&format!("change `{}` to an integer", field)),
                );
            }
        }
        ValueType::Number => {
            if value.parse::<f64>().is_err() {
                errors.push(
                    LintError::new(file,line, section, "invalid_value")
                        .with_key(field)
                        .with_got(value)
                        .with_expected("number")
                        .with_fix(&format!("change `{}` to a number", field)),
                );
            }
        }
        ValueType::List(inner) => {
            for item in value.split(',').map(|s| s.trim()) {
                if !item.is_empty() {
                    validate_value(item, inner, line, section, field, file, errors);
                }
            }
        }
        ValueType::Cron => {
            validate_cron(value, line, section, field, file, errors);
        }
    }
}

fn validate_cron(
    value: &str,
    line: usize,
    section: &str,
    field: &str,
    file: &str,
    errors: &mut Vec<LintError>,
) {
    let value = value.trim();

    if let Some(rest) = value.strip_prefix("every ") {
        let rest = rest.trim();
        if let Some(num) = rest.strip_suffix('m').or(rest.strip_suffix('h')) {
            if num.parse::<u32>().is_ok() {
                return;
            }
        }
        errors.push(
            LintError::new(file,line, section, "invalid_value")
                .with_key(field)
                .with_got(value)
                .with_expected("interval: every Nm or every Nh")
                .with_fix(&format!("change `{}` to `every 5m`, `every 2h`, etc.", field)),
        );
        return;
    }

    let fields: Vec<&str> = value.split_whitespace().collect();
    if fields.len() != 5 {
        errors.push(
            LintError::new(file,line, section, "invalid_value")
                .with_key(field)
                .with_got(value)
                .with_expected("5-field cron or interval (every Nm/Nh)")
                .with_fix(&format!(
                    "change `{}` to `every 5m`, `every 2h`, or `0 5 * * *`",
                    field
                )),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lint(schema_text: &str, doc_text: &str) -> Vec<LintError> {
        let schema = structmd::schema::load_schema(schema_text, "test").unwrap();
        let doc = structmd::parse::parse(doc_text);
        validate(&doc, &schema, "test.md")
    }

    fn has_code(errors: &[LintError], code: &str) -> bool {
        errors.iter().any(|e| e.code == code)
    }

    fn has_key(errors: &[LintError], key: &str) -> bool {
        errors.iter().any(|e| e.key == key)
    }

    fn all_have_fix(errors: &[LintError]) -> bool {
        errors.iter().all(|e| !e.fix.is_empty())
    }

    const TOOLS_SCHEMA: &str = "\
```grammar
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

    #[test]
    fn valid_tools_file() {
        let errors = lint(TOOLS_SCHEMA, "\
# Tools

## my_tool

A description.

- **server:** fetch
- **parameters:**

| name | type   | required | desc  |
|------|--------|----------|-------|
| url  | string | yes      | A URL |
");
        assert!(errors.is_empty(), "unexpected errors: {:#?}", errors);
    }

    #[test]
    fn missing_h1() {
        let errors = lint(TOOLS_SCHEMA, "## my_tool\n- server: x\n");
        assert!(has_code(&errors, "missing_section") || has_code(&errors, "wrong_heading"));
        assert!(all_have_fix(&errors));
    }

    #[test]
    fn wrong_h1_text() {
        let errors = lint(TOOLS_SCHEMA, "# Wrong\n## my_tool\nDesc\n- server: x\n");
        assert!(!errors.is_empty());
        assert!(all_have_fix(&errors));
    }

    #[test]
    fn snake_case_enforced() {
        let errors = lint(TOOLS_SCHEMA, "\
# Tools

## Bad Name

Description.

- server: fetch
");
        assert!(has_code(&errors, "section_count"));
        assert!(all_have_fix(&errors));
    }

    #[test]
    fn prose_required() {
        let errors = lint(TOOLS_SCHEMA, "\
# Tools

## my_tool
- server: fetch
");
        assert!(has_code(&errors, "missing_prose"));
        assert!(all_have_fix(&errors));
    }

    #[test]
    fn table_required_when_expects_table() {
        let errors = lint(TOOLS_SCHEMA, "\
# Tools

## my_tool

Description.

- server: fetch
- parameters:
");
        assert!(has_code(&errors, "missing_table"));
        assert!(all_have_fix(&errors));
    }

    #[test]
    fn table_enum_validation() {
        let errors = lint(TOOLS_SCHEMA, "\
# Tools

## my_tool

Description.

- server: fetch
- parameters:

| name | type   | required | desc  |
|------|--------|----------|-------|
| x    | widget | maybe    | stuff |
");
        let invalid: Vec<_> = errors.iter().filter(|e| e.code == "invalid_value").collect();
        assert!(invalid.iter().any(|e| e.got == "widget"));
        assert!(invalid.iter().any(|e| e.got == "maybe"));
        assert!(all_have_fix(&errors));
    }

    const SOZU_SCHEMA: &str = "\
```grammar
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

    #[test]
    fn valid_sozu_file() {
        let errors = lint(SOZU_SCHEMA, "\
# Agyo

## Container
- image: localhost/test:latest
- network: outbound
- mounts: /tmp/a, /tmp/b

## Servers

### fetch
- command: /usr/local/bin/fetch

# Tasks

## heartbeat

A test task.

- schedule: every 5m
- run: echo hi
- log: true
");
        assert!(errors.is_empty(), "unexpected errors: {:#?}", errors);
    }

    #[test]
    fn sozu_without_tasks_is_ok() {
        let errors = lint(SOZU_SCHEMA, "\
# Agyo

## Container
- image: localhost/test:latest

## Servers

### fetch
- command: /usr/local/bin/fetch
");
        assert!(errors.is_empty(), "unexpected errors: {:#?}", errors);
    }

    #[test]
    fn sozu_missing_required_image() {
        let errors = lint(SOZU_SCHEMA, "\
# Agyo

## Container
- network: outbound

## Servers

### fetch
- command: /usr/local/bin/fetch
");
        assert!(has_code(&errors, "missing_property"));
        assert!(has_key(&errors, "image"));
        assert!(all_have_fix(&errors));
    }

    #[test]
    fn sozu_missing_server_command() {
        let errors = lint(SOZU_SCHEMA, "\
# Agyo

## Container
- image: localhost/test:latest

## Servers

### fetch
- secrets: key1
");
        assert!(has_code(&errors, "missing_property"));
        assert!(has_key(&errors, "command"));
        assert!(all_have_fix(&errors));
    }

    #[test]
    fn sozu_scoped_properties_dont_leak() {
        let errors = lint(SOZU_SCHEMA, "\
# Agyo

## Container
- image: localhost/test:latest

## Servers

### fetch
- command: /usr/local/bin/fetch
");
        assert!(errors.is_empty(), "scoped properties leaked: {:#?}", errors);
    }

    #[test]
    fn bool_validation() {
        let errors = lint(SOZU_SCHEMA, "\
# Agyo
## Container
- image: test
## Servers
### s1
- command: /bin/s1
# Tasks
## t1
- schedule: every 5m
- run: echo hi
- log: maybe
");
        let bool_err = errors.iter().find(|e| e.key == "log").unwrap();
        assert_eq!(bool_err.code, "invalid_value");
        assert_eq!(bool_err.got, "maybe");
        assert!(bool_err.expected.contains("true"));
        assert!(!bool_err.fix.is_empty());
    }

    #[test]
    fn cron_interval_valid() {
        let errors = lint(SOZU_SCHEMA, "\
# Agyo
## Container
- image: test
## Servers
### s1
- command: /bin/s1
# Tasks
## t1
- schedule: every 10m
- run: echo hi
");
        let cron_errors: Vec<_> = errors.iter().filter(|e| e.key == "schedule").collect();
        assert!(cron_errors.is_empty());
    }

    #[test]
    fn cron_expression_valid() {
        let errors = lint(SOZU_SCHEMA, "\
# Agyo
## Container
- image: test
## Servers
### s1
- command: /bin/s1
# Tasks
## t1
- schedule: 0 5 * * *
- run: echo hi
");
        let cron_errors: Vec<_> = errors.iter().filter(|e| e.key == "schedule").collect();
        assert!(cron_errors.is_empty());
    }

    #[test]
    fn cron_invalid_interval() {
        let errors = lint(SOZU_SCHEMA, "\
# Agyo
## Container
- image: test
## Servers
### s1
- command: /bin/s1
# Tasks
## t1
- schedule: every tuesday
- run: echo hi
");
        let err = errors.iter().find(|e| e.key == "schedule").unwrap();
        assert_eq!(err.code, "invalid_value");
        assert_eq!(err.got, "every tuesday");
        assert!(!err.fix.is_empty());
    }

    #[test]
    fn cron_invalid_expression() {
        let errors = lint(SOZU_SCHEMA, "\
# Agyo
## Container
- image: test
## Servers
### s1
- command: /bin/s1
# Tasks
## t1
- schedule: at 5pm
- run: echo hi
");
        let err = errors.iter().find(|e| e.key == "schedule").unwrap();
        assert_eq!(err.code, "invalid_value");
        assert!(!err.fix.is_empty());
    }

    #[test]
    fn list_comma_separated() {
        let errors = lint(SOZU_SCHEMA, "\
# Agyo
## Container
- image: test
- mounts: /tmp/a, /tmp/b
## Servers
### s1
- command: /bin/s1
");
        let mount_errors: Vec<_> = errors.iter().filter(|e| e.key == "mounts").collect();
        assert!(mount_errors.is_empty());
    }

    #[test]
    fn one_or_more_section_required() {
        let schema_text = "\
```grammar
document ::= H1(\"T\") item+
item     ::= H2(IDENT) property+
```

```types
@key : string
```
";
        let errors = lint(schema_text, "# T\n");
        assert!(has_code(&errors, "section_count"));
        assert!(all_have_fix(&errors));
    }

    #[test]
    fn every_error_has_a_fix() {
        // Intentionally broken file — every error should have a fix
        let errors = lint(SOZU_SCHEMA, "\
# Agyo
## Container
- network: outbound
## Servers
### s1
- secrets: key
# Tasks
## t1
- schedule: nope
- log: maybe
");
        assert!(!errors.is_empty());
        for err in &errors {
            assert!(
                !err.fix.is_empty(),
                "error at line {} ({}) has no fix: code={}, key={}",
                err.line, err.section, err.code, err.key
            );
        }
    }
}
