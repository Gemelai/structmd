//! Schema-driven validator for structmd documents.
//!
//! Takes a parsed [`Document`] and a loaded [`Schema`] and returns a list of
//! [`Error`]s. The document is valid if the returned list is empty.

use crate::errors::Error;
use crate::parse::{Document, H1Node, Section};
use crate::schema::{H1Schema, Quantity, Schema, SectionSchema, ValueType};

fn type_label(vt: &ValueType) -> &'static str {
    match vt {
        ValueType::String => "string",
        ValueType::Bool => "bool",
        ValueType::Enum(_) => "enum",
        ValueType::Integer => "integer",
        ValueType::Number => "number",
        ValueType::Text => "text",
        ValueType::Label => "label",
        ValueType::List(_) => "list",
    }
}

/// Validate `doc` against `schema` and return any errors.
///
/// `file` is used as the file name in error output.
/// Returns an empty vec if the document is valid.
pub fn validate(doc: &Document, schema: &Schema, file: &str) -> Vec<Error> {
    let mut errors = Vec::new();

    let mut doc_idx = 0;
    for h1_schema in &schema.nodes {
        if let Some(ref expected_text) = h1_schema.text {
            let found = doc.nodes[doc_idx..].iter().enumerate().find(|(_, n)| {
                n.heading
                    .as_ref()
                    .is_some_and(|h| h.text == *expected_text)
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
                            Error::new(file, 1, "", "missing_section")
                                .with_expected(&format!("# {}", expected_text))
                                .with_fix(&format!("add `# {}` section", expected_text)),
                        );
                    }
                },
            }
        } else if let Some(node) = doc.nodes.get(doc_idx) {
            doc_idx += 1;
            validate_h1_children(node, h1_schema, schema, file, &mut errors);
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
    errors: &mut Vec<Error>,
) {
    validate_sections(&doc_node.sections, &h1_schema.children, schema, file, errors);
}

fn validate_sections(
    doc_sections: &[Section],
    schema_sections: &[SectionSchema],
    schema: &Schema,
    file: &str,
    errors: &mut Vec<Error>,
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
                let name = sec_schema.production_name.as_deref().unwrap_or("section");
                errors.push(
                    Error::new(file, 1, name, "section_count")
                        .with_got(&count.to_string())
                        .with_expected("1")
                        .with_fix(&format!(
                            "need exactly 1 H{} `{}` section",
                            sec_schema.level, name
                        )),
                );
            }
            Quantity::OneOrMore if count == 0 => {
                let name = sec_schema.production_name.as_deref().unwrap_or("section");
                errors.push(
                    Error::new(file, 1, name, "section_count")
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
    errors: &mut Vec<Error>,
) {
    let name = &sec.heading.text;
    let lineno = sec.heading.line;

    if name.is_empty() {
        errors.push(
            Error::new(file, lineno, "", "invalid_name")
                .with_got("(empty)")
                .with_fix(&format!(
                    "add a name after `{}`",
                    "#".repeat(sec.heading.level as usize)
                )),
        );
    }

    if sec_schema.expects_prose && sec.prose.is_none() {
        errors.push(
            Error::new(file, lineno, name, "missing_prose")
                .with_fix(&format!("add text after `## {}`", name)),
        );
    }

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
                        Error::new(file, lineno, name, "missing_property")
                            .with_key(key)
                            .with_expected(type_label(&prop_schema.value_type))
                            .with_fix(&format!(
                                "add `- {}: <{}>`",
                                key,
                                type_label(&prop_schema.value_type)
                            )),
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

    if sec_schema.expects_table {
        let table_schema = sec_schema.table.as_ref().or(schema.global_table.as_ref());

        match (&sec.table, table_schema) {
            (None, Some(tschema)) => {
                let cols: Vec<&str> = tschema.columns.iter().map(|c| c.name.as_str()).collect();
                errors.push(
                    Error::new(file, lineno, name, "missing_table")
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
                                Error::new(file, table.header_line, name, "column_mismatch")
                                    .with_key(&format!("column {}", i + 1))
                                    .with_got(got)
                                    .with_expected(&col_schema.name)
                                    .with_fix(&format!(
                                        "rename column {} to `{}`",
                                        i + 1,
                                        col_schema.name
                                    )),
                            );
                        }
                        None => {
                            errors.push(
                                Error::new(file, table.header_line, name, "column_mismatch")
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
                        Error::new(file, table.header_line, name, "row_count")
                            .with_got("0")
                            .with_expected("at least 1")
                            .with_fix("add at least one data row to the table"),
                    );
                }

                let col_count = table.columns.len();
                for row in &table.rows {
                    if row.cells.len() != col_count {
                        errors.push(
                            Error::new(file, row.line, name, "column_mismatch")
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
    errors: &mut Vec<Error>,
) {
    match vtype {
        ValueType::Label | ValueType::Text => {}
        ValueType::String => {
            if value.is_empty() {
                errors.push(
                    Error::new(file, line, section, "invalid_value")
                        .with_key(field)
                        .with_got("(empty)")
                        .with_expected("non-empty string")
                        .with_fix(&format!("set `- {}: <value>`", field)),
                );
            }
        }
        ValueType::Bool => {
            let lower = value.to_lowercase();
            if lower != "true" && lower != "false" {
                errors.push(
                    Error::new(file, line, section, "invalid_value")
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
                    Error::new(file, line, section, "invalid_value")
                        .with_key(field)
                        .with_got(value)
                        .with_expected(&vals.join(", "))
                        .with_fix(&format!(
                            "change `{}` to one of: {}",
                            field,
                            vals.join(", ")
                        )),
                );
            }
        }
        ValueType::Integer => {
            if value.parse::<i64>().is_err() {
                errors.push(
                    Error::new(file, line, section, "invalid_value")
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
                    Error::new(file, line, section, "invalid_value")
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
    }
}
