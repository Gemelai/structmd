//! Schema types for structmd validation and codegen.
//!
//! A schema is itself a structmd file containing:
//!   - ` ```grammar ` block: structural grammar (document shape)
//!   - ` ```types ` or ` ```types:production ` block: property type constraints
//!   - ` ```table ` or ` ```table:production ` block: table column constraints
//!
//! Load a schema with [`load_schema`], then use it with a validator or codegen.

use std::collections::BTreeMap;

// ── Public types ──

/// A fully parsed and resolved schema, ready for validation or codegen.
#[derive(Debug)]
pub struct Schema {
    /// Schema name (passed to [`load_schema`], used in error output).
    pub name: String,
    /// Expected H1-level nodes, in grammar order.
    pub nodes: Vec<H1Schema>,
    /// Properties declared in a bare ` ```types ` block (apply to any section without scoped types).
    pub global_properties: BTreeMap<String, PropertySchema>,
    /// Table declared in a bare ` ```table ` block.
    pub global_table: Option<TableSchema>,
}

/// Schema for an H1-level node.
#[derive(Debug)]
pub struct H1Schema {
    /// Grammar production that expanded into this node, if any.
    pub production_name: Option<String>,
    /// Expected heading text. `None` means any text is accepted.
    pub text: Option<String>,
    /// How many times this node may appear.
    pub quantity: Quantity,
    /// Whether this node expects top-level properties.
    pub expects_properties: bool,
    /// Scoped property constraints for this node's production.
    pub properties: Option<BTreeMap<String, PropertySchema>>,
    /// Expected H2 children.
    pub children: Vec<SectionSchema>,
}

/// Schema for an H2 or H3 section.
#[derive(Debug)]
pub struct SectionSchema {
    /// Grammar production that expanded into this section, if any.
    pub production_name: Option<String>,
    /// Heading level: 2 or 3.
    pub level: u8,
    /// Pattern the section heading must match.
    pub name_pattern: NamePattern,
    /// How many times this section may appear.
    pub quantity: Quantity,
    /// Whether this section requires prose.
    pub expects_prose: bool,
    /// Whether this section requires properties.
    pub expects_properties: bool,
    /// Whether this section requires a table.
    pub expects_table: bool,
    /// Scoped property constraints for this section's production.
    pub properties: Option<BTreeMap<String, PropertySchema>>,
    /// Scoped table constraint for this section's production.
    pub table: Option<TableSchema>,
    /// Expected H3 children (only meaningful when `level == 2`).
    pub children: Vec<SectionSchema>,
}

/// Type and modifier constraints for a single property.
#[derive(Debug, Clone)]
pub struct PropertySchema {
    /// Expected value type.
    pub value_type: ValueType,
    /// Whether this property must be present.
    pub required: bool,
    /// Default value to use when the property is absent.
    pub default: Option<String>,
}

/// The type system for structmd property values.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    /// Arbitrary single-line text with no format constraint.
    String,
    /// `true` or `false` (case-insensitive).
    Bool,
    /// One of a fixed set of string values.
    Enum(Vec<String>),
    /// Whole number.
    Integer,
    /// Floating-point or integer number.
    Number,
    /// Multi-line text (typically from a fenced code block property).
    Text,
    /// A value-less property used as a label or flag (`- key:`).
    Label,
    /// A comma-separated list of another type.
    List(Box<ValueType>),
}

/// Table column definitions.
#[derive(Debug, Clone)]
pub struct TableSchema {
    /// Expected columns, in declared order.
    pub columns: Vec<ColumnSchema>,
}

/// Type constraint for one table column.
#[derive(Debug, Clone)]
pub struct ColumnSchema {
    /// Expected column header name.
    pub name: String,
    /// Expected value type for cells in this column.
    pub column_type: ValueType,
}

/// Pattern that a section heading must match.
#[derive(Debug)]
pub enum NamePattern {
    /// Heading must equal this exact string.
    Exact(String),
    /// Heading must be `snake_case` (lowercase letters, digits, underscores).
    SnakeCase,
    /// Heading must be `kebab-case` (lowercase letters, digits, hyphens).
    KebabCase,
    /// Heading must be an identifier (letters/digits/underscores/hyphens, starts with a letter).
    Ident,
    /// Any heading is accepted.
    Any,
}

/// How many times a node or section may appear.
#[derive(Debug, Clone, Copy)]
pub enum Quantity {
    /// Exactly once.
    One,
    /// One or more times.
    OneOrMore,
    /// Zero or more times.
    ZeroOrMore,
    /// Zero or one time.
    Optional,
}

/// Whether a structural element (prose, table, etc.) is required or optional.
#[derive(Debug, Default, PartialEq)]
pub enum Presence {
    Required,
    #[default]
    Optional,
}

impl NamePattern {
    /// Returns `true` if `name` satisfies this pattern.
    pub fn matches(&self, name: &str) -> bool {
        match self {
            NamePattern::Exact(expected) => name == expected,
            NamePattern::SnakeCase => {
                !name.is_empty()
                    && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                    && name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            }
            NamePattern::KebabCase => {
                !name.is_empty()
                    && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                    && name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            }
            NamePattern::Ident => {
                !name.is_empty()
                    && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                    && name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            }
            NamePattern::Any => true,
        }
    }

    /// Human-readable label for this pattern, used in error messages.
    pub fn label(&self) -> &str {
        match self {
            NamePattern::Exact(s) => s,
            NamePattern::SnakeCase => "snake_case",
            NamePattern::KebabCase => "kebab-case",
            NamePattern::Ident => "identifier",
            NamePattern::Any => "any",
        }
    }

    /// Returns `true` if section names are free-form (i.e. this is not an [`Exact`](Self::Exact) match).
    ///
    /// Used by codegen to decide whether to generate a named field or a `HashMap`.
    pub fn is_dynamic(&self) -> bool {
        !matches!(self, NamePattern::Exact(_))
    }
}

// ── Schema loader ──

/// Parse and resolve a schema from structmd text.
///
/// `schema_name` is used in error messages and stored in [`Schema::name`].
///
/// # Errors
///
/// Returns an error string if the schema has no ` ```grammar ` block,
/// an unknown type annotation, an unknown name pattern, or a circular production reference.
pub fn load_schema(text: &str, schema_name: &str) -> Result<Schema, String> {
    let mut grammar_blocks = Vec::new();
    let mut global_types_blocks = Vec::new();
    let mut global_table_blocks = Vec::new();
    let mut scoped_types: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut scoped_tables: BTreeMap<String, Vec<String>> = BTreeMap::new();

    // Extract fenced code blocks
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if let Some(info) = trimmed.strip_prefix("```") {
            let info = info.trim();
            if info.is_empty() {
                i += 1;
                while i < lines.len() && !lines[i].trim().starts_with("```") {
                    i += 1;
                }
                i += 1;
                continue;
            }
            let tag = info.to_string();
            i += 1;
            let mut content = String::new();
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(lines[i]);
                i += 1;
            }
            i += 1; // skip closing fence

            if tag == "grammar" {
                grammar_blocks.push(content);
            } else if let Some(prod) = tag.strip_prefix("types:") {
                scoped_types
                    .entry(prod.trim().to_string())
                    .or_default()
                    .push(content);
            } else if tag == "types" {
                global_types_blocks.push(content);
            } else if let Some(prod) = tag.strip_prefix("table:") {
                scoped_tables
                    .entry(prod.trim().to_string())
                    .or_default()
                    .push(content);
            } else if tag == "table" {
                global_table_blocks.push(content);
            }
        } else {
            i += 1;
        }
    }

    if grammar_blocks.is_empty() {
        return Err("schema has no ```grammar block".into());
    }

    // Parse grammar productions
    let mut productions: Vec<(String, Vec<String>)> = Vec::new();
    for block in &grammar_blocks {
        for line in block.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some((name, body)) = line.split_once("::=") {
                let name = name.trim().to_string();
                let tokens = tokenize_grammar(body.trim());
                productions.push((name, tokens));
            }
        }
    }

    // Parse scoped property maps
    let mut scoped_props: BTreeMap<String, BTreeMap<String, PropertySchema>> = BTreeMap::new();
    for (prod_name, blocks) in &scoped_types {
        let props = parse_types_blocks(blocks)?;
        scoped_props.insert(prod_name.clone(), props);
    }

    // Parse scoped table maps
    let mut scoped_tbls: BTreeMap<String, TableSchema> = BTreeMap::new();
    for (prod_name, blocks) in &scoped_tables {
        if let Some(tbl) = parse_table_blocks(blocks)? {
            scoped_tbls.insert(prod_name.clone(), tbl);
        }
    }

    // Parse global properties and table
    let global_properties = parse_types_blocks(&global_types_blocks)?;
    let global_table = parse_table_blocks(&global_table_blocks)?;

    // Resolve grammar into tree
    let production_map: BTreeMap<String, Vec<String>> = productions.iter().cloned().collect();
    let root_tokens = production_map
        .get("document")
        .or_else(|| productions.first().map(|(_, t)| t))
        .cloned()
        .unwrap_or_default();

    let mut h1_nodes = Vec::new();
    let mut root_sections = Vec::new();
    resolve_tokens(
        &root_tokens,
        &production_map,
        &scoped_props,
        &scoped_tbls,
        &mut h1_nodes,
        &mut root_sections,
        None,
        &mut Vec::new(),
    )?;

    // If there are root sections but no H1 nodes, wrap them
    if !root_sections.is_empty() && h1_nodes.is_empty() {
        h1_nodes.push(H1Schema {
            production_name: None,
            text: None,
            quantity: Quantity::One,
            expects_properties: false,
            properties: None,
            children: root_sections,
        });
    }

    Ok(Schema {
        name: schema_name.to_string(),
        nodes: h1_nodes,
        global_properties,
        global_table,
    })
}

// ── Grammar resolution ──

#[allow(clippy::too_many_arguments)]
fn resolve_tokens(
    tokens: &[String],
    productions: &BTreeMap<String, Vec<String>>,
    scoped_props: &BTreeMap<String, BTreeMap<String, PropertySchema>>,
    scoped_tbls: &BTreeMap<String, TableSchema>,
    h1_nodes: &mut Vec<H1Schema>,
    sections: &mut Vec<SectionSchema>,
    current_production: Option<&str>,
    visited: &mut Vec<String>,
) -> Result<(), String> {
    for token in tokens {
        let s = token.as_str();

        // H1("text") or H1
        if s.starts_with("H1") {
            let (base, quantity) = split_quantifier(s);
            let text = extract_quoted_arg(base);
            h1_nodes.push(H1Schema {
                production_name: current_production.map(|s| s.to_string()),
                text,
                quantity,
                expects_properties: false,
                properties: current_production.and_then(|p| scoped_props.get(p).cloned()),
                children: Vec::new(),
            });
            continue;
        }

        // H2/H3
        if s.starts_with("H2") || s.starts_with("H3") {
            let level = if s.starts_with("H2") { 2 } else { 3 };
            let (base, quantity) = split_quantifier(s);
            let pattern = extract_paren_arg(base).unwrap_or("ANY");
            let name_pattern = parse_name_pattern(pattern)?;

            let sec = SectionSchema {
                production_name: current_production.map(|s| s.to_string()),
                level,
                name_pattern,
                quantity,
                expects_prose: false,
                expects_properties: false,
                expects_table: false,
                properties: current_production.and_then(|p| scoped_props.get(p).cloned()),
                table: current_production.and_then(|p| scoped_tbls.get(p).cloned()),
                children: Vec::new(),
            };

            // H3 nests under last H2, H2 nests under last H1
            if level == 3 {
                // Find the last H2 section to nest under
                let parent = sections.last_mut().or_else(|| {
                    h1_nodes
                        .last_mut()
                        .and_then(|h1| h1.children.last_mut())
                });
                if let Some(parent) = parent {
                    parent.children.push(sec);
                } else {
                    sections.push(sec);
                }
            } else if !h1_nodes.is_empty() {
                h1_nodes.last_mut().unwrap().children.push(sec);
            } else {
                sections.push(sec);
            }
            continue;
        }

        let (base, quantity) = split_quantifier(s);

        // Built-in tokens modify the last section in scope, or H1 if no section
        match base {
            "prose" | "property" | "table" => {
                let target = find_last_section_mut(h1_nodes, sections);
                if let Some(sec) = target {
                    match base {
                        "prose" => sec.expects_prose = true,
                        "property" => sec.expects_properties = true,
                        "table" => sec.expects_table = true,
                        _ => unreachable!(),
                    }
                } else if base == "property" {
                    // No section — apply to last H1 node directly
                    if let Some(h1) = h1_nodes.last_mut() {
                        h1.expects_properties = true;
                    }
                }
                continue;
            }
            _ => {}
        }

        // Named production reference
        if let Some(ref_tokens) = productions.get(base) {
            if visited.contains(&base.to_string()) {
                return Err(format!("circular reference in grammar: {}", base));
            }
            visited.push(base.to_string());
            let h1_count_before = h1_nodes.len();

            resolve_tokens(
                ref_tokens,
                productions,
                scoped_props,
                scoped_tbls,
                h1_nodes,
                sections,
                Some(base),
                visited,
            )?;

            // Apply quantifier from reference to the outermost thing it created
            match quantity {
                Quantity::One => {}
                q => {
                    // Check if this reference created a new H1 node
                    let created_h1 = h1_nodes.len() > h1_count_before;
                    if created_h1 {
                        if let Some(h1) = h1_nodes.last_mut() {
                            h1.quantity = q;
                        }
                    } else if let Some(sec) = find_last_section_mut(h1_nodes, sections) {
                        sec.quantity = q;
                    }
                }
            }

            visited.pop();
        }
    }

    Ok(())
}

fn find_last_section_mut<'a>(
    h1_nodes: &'a mut [H1Schema],
    sections: &'a mut [SectionSchema],
) -> Option<&'a mut SectionSchema> {
    // Prefer the deepest section in the last H1 node
    if let Some(h1) = h1_nodes.last_mut() {
        if let Some(sec) = h1.children.last_mut() {
            if !sec.children.is_empty() {
                return sec.children.last_mut();
            }
            return Some(sec);
        }
    }
    // Fallback to root sections
    if let Some(sec) = sections.last_mut() {
        if !sec.children.is_empty() {
            return sec.children.last_mut();
        }
        return Some(sec);
    }
    None
}

// ── Types/table block parsing ──

fn parse_types_blocks(blocks: &[String]) -> Result<BTreeMap<String, PropertySchema>, String> {
    let mut properties = BTreeMap::new();
    for block in blocks {
        for line in block.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some((key, rest)) = line.strip_prefix('@').and_then(|l| l.split_once(':')) {
                let key = key.trim().to_string();
                let rest = rest.split('#').next().unwrap_or(rest).trim();
                let (vtype, required, default) = parse_type_with_modifiers(rest)?;
                properties.insert(
                    key,
                    PropertySchema {
                        value_type: vtype,
                        required,
                        default,
                    },
                );
            }
        }
    }
    Ok(properties)
}

fn parse_table_blocks(blocks: &[String]) -> Result<Option<TableSchema>, String> {
    let mut columns = Vec::new();
    for block in blocks {
        for line in block.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some((key, rest)) = line.strip_prefix('@').and_then(|l| l.split_once(':')) {
                let name = key.trim().to_string();
                let rest = rest.split('#').next().unwrap_or(rest).trim();
                let column_type = parse_value_type(rest)?;
                columns.push(ColumnSchema { name, column_type });
            }
        }
    }
    if columns.is_empty() {
        Ok(None)
    } else {
        Ok(Some(TableSchema { columns }))
    }
}

// ── Type parsing with modifiers ──

fn parse_type_with_modifiers(s: &str) -> Result<(ValueType, bool, Option<String>), String> {
    // Split on commas, but respect parentheses (enum(a, b) shouldn't split)
    let parts = split_modifiers(s);
    if parts.is_empty() {
        return Err("empty type annotation".into());
    }

    let vtype = parse_value_type(parts[0].trim())?;
    let mut required = false;
    let mut default = None;

    for part in &parts[1..] {
        let part = part.trim();
        if part == "required" {
            required = true;
        } else if let Some(inner) = part
            .strip_prefix("default(")
            .and_then(|r| r.strip_suffix(')'))
        {
            default = Some(inner.trim().trim_matches('"').to_string());
        }
    }

    Ok((vtype, required, default))
}

fn split_modifiers(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut paren_depth = 0;

    for (i, ch) in s.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            ',' if paren_depth == 0 => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

fn parse_value_type(s: &str) -> Result<ValueType, String> {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix("enum(").and_then(|r| r.strip_suffix(')')) {
        let values: Vec<String> = inner.split(',').map(|v| v.trim().to_string()).collect();
        return Ok(ValueType::Enum(values));
    }
    if let Some(inner) = s.strip_prefix("list(").and_then(|r| r.strip_suffix(')')) {
        let inner_type = parse_value_type(inner.trim())?;
        return Ok(ValueType::List(Box::new(inner_type)));
    }
    match s {
        "string" => Ok(ValueType::String),
        "bool" => Ok(ValueType::Bool),
        "integer" => Ok(ValueType::Integer),
        "number" => Ok(ValueType::Number),
        "text" => Ok(ValueType::Text),
        "label" => Ok(ValueType::Label),
        _ => Err(format!("unknown type: {}", s)),
    }
}

// ── Grammar tokenizer and helpers ──

fn tokenize_grammar(body: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = body.chars().peekable();
    let mut current = String::new();
    let mut paren_depth = 0;

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' if paren_depth == 0 => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                chars.next();
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
                chars.next();
            }
            ')' => {
                paren_depth -= 1;
                current.push(ch);
                chars.next();
            }
            _ => {
                current.push(ch);
                chars.next();
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn extract_quoted_arg(s: &str) -> Option<String> {
    let start = s.find('"')?;
    let end = s[start + 1..].find('"')?;
    Some(s[start + 1..start + 1 + end].to_string())
}

fn extract_paren_arg(s: &str) -> Option<&str> {
    let start = s.find('(')?;
    let end = s.rfind(')')?;
    Some(&s[start + 1..end])
}

fn split_quantifier(s: &str) -> (&str, Quantity) {
    if let Some(base) = s.strip_suffix('+') {
        (base, Quantity::OneOrMore)
    } else if let Some(base) = s.strip_suffix('*') {
        (base, Quantity::ZeroOrMore)
    } else if let Some(base) = s.strip_suffix('?') {
        (base, Quantity::Optional)
    } else {
        (s, Quantity::One)
    }
}

fn parse_name_pattern(s: &str) -> Result<NamePattern, String> {
    match s {
        "SNAKE_CASE" => Ok(NamePattern::SnakeCase),
        "KEBAB_CASE" | "KEBAB-CASE" => Ok(NamePattern::KebabCase),
        "IDENT" | "IDENTIFIER" => Ok(NamePattern::Ident),
        "ANY" => Ok(NamePattern::Any),
        quoted if quoted.starts_with('"') && quoted.ends_with('"') => {
            Ok(NamePattern::Exact(quoted[1..quoted.len() - 1].to_string()))
        }
        _ => Err(format!("unknown name pattern: {}", s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(text: &str) -> Schema {
        load_schema(text, "test").unwrap()
    }

    // ── Grammar basics ──

    #[test]
    fn no_grammar_block_errors() {
        let err = load_schema("# Schema\nNo code blocks here.", "test");
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("no ```grammar block"));
    }

    #[test]
    fn simple_flat_grammar() {
        let schema = load("\
# Schema
```grammar
document ::= H1(\"Tools\") tool+
tool     ::= H2(SNAKE_CASE) prose property* table
```

```types
@server : string
```

```table
@name : string
@type : enum(string, integer)
```
");
        assert_eq!(schema.nodes.len(), 1);
        assert_eq!(schema.nodes[0].text.as_deref(), Some("Tools"));
        assert_eq!(schema.nodes[0].children.len(), 1);

        let tool = &schema.nodes[0].children[0];
        assert_eq!(tool.production_name.as_deref(), Some("tool"));
        assert!(tool.expects_prose);
        assert!(tool.expects_properties);
        assert!(tool.expects_table);
        assert!(matches!(tool.name_pattern, NamePattern::SnakeCase));
    }

    #[test]
    fn multi_h1_grammar() {
        let schema = load("\
```grammar
document ::= agyo tasks?
agyo     ::= H1(\"Agyo\") stuff
stuff    ::= H2(\"Container\") property+
tasks    ::= H1(\"Tasks\") task+
task     ::= H2(IDENT) property+
```
");
        assert_eq!(schema.nodes.len(), 2);
        assert_eq!(schema.nodes[0].text.as_deref(), Some("Agyo"));
        assert_eq!(schema.nodes[1].text.as_deref(), Some("Tasks"));

        // tasks? should make the second H1 optional
        assert!(matches!(schema.nodes[1].quantity, Quantity::Optional));
    }

    // ── Quantifiers ──

    #[test]
    fn quantifier_one_or_more() {
        let schema = load("\
```grammar
document ::= H1(\"T\") item+
item     ::= H2(IDENT) property+
```
");
        let item = &schema.nodes[0].children[0];
        assert!(matches!(item.quantity, Quantity::OneOrMore));
    }

    #[test]
    fn quantifier_zero_or_more() {
        let schema = load("\
```grammar
document ::= H1(\"T\") item*
item     ::= H2(IDENT) property*
```
");
        let item = &schema.nodes[0].children[0];
        assert!(matches!(item.quantity, Quantity::ZeroOrMore));
    }

    #[test]
    fn quantifier_optional() {
        let schema = load("\
```grammar
document ::= H1(\"T\") item?
item     ::= H2(IDENT)
```
");
        let item = &schema.nodes[0].children[0];
        assert!(matches!(item.quantity, Quantity::Optional));
    }

    // ── Scoped properties ──

    #[test]
    fn scoped_types_blocks() {
        let schema = load("\
```grammar
document  ::= H1(\"Root\") alpha beta
alpha     ::= H2(\"Alpha\") property+
beta      ::= H2(\"Beta\") property+
```

```types:alpha
@color : string
```

```types:beta
@size : integer
```
");
        let alpha = &schema.nodes[0].children[0];
        let beta = &schema.nodes[0].children[1];

        let alpha_props = alpha.properties.as_ref().unwrap();
        assert!(alpha_props.contains_key("color"));
        assert!(!alpha_props.contains_key("size"));

        let beta_props = beta.properties.as_ref().unwrap();
        assert!(beta_props.contains_key("size"));
        assert!(!beta_props.contains_key("color"));
    }

    #[test]
    fn global_types_fallback() {
        let schema = load("\
```grammar
document ::= H1(\"T\") item+
item     ::= H2(IDENT) property+
```

```types
@name : string
```
");
        // No scoped types — item should have properties = None, falls back to global
        let item = &schema.nodes[0].children[0];
        assert!(item.properties.is_none());
        assert!(schema.global_properties.contains_key("name"));
    }

    // ── Type parsing ──

    #[test]
    fn type_string() {
        assert_eq!(parse_value_type("string").unwrap(), ValueType::String);
    }

    #[test]
    fn type_bool() {
        assert_eq!(parse_value_type("bool").unwrap(), ValueType::Bool);
    }

    #[test]
    fn type_enum() {
        let vt = parse_value_type("enum(a, b, c)").unwrap();
        assert_eq!(
            vt,
            ValueType::Enum(vec!["a".into(), "b".into(), "c".into()])
        );
    }

    #[test]
    fn type_list() {
        let vt = parse_value_type("list(string)").unwrap();
        assert_eq!(vt, ValueType::List(Box::new(ValueType::String)));
    }

    #[test]
    fn type_unknown_errors() {
        assert!(parse_value_type("widget").is_err());
    }

    // ── Modifiers ──

    #[test]
    fn modifier_required() {
        let (vt, req, def) = parse_type_with_modifiers("string, required").unwrap();
        assert_eq!(vt, ValueType::String);
        assert!(req);
        assert!(def.is_none());
    }

    #[test]
    fn modifier_default() {
        let (vt, req, def) = parse_type_with_modifiers("string, default(\"hello\")").unwrap();
        assert_eq!(vt, ValueType::String);
        assert!(!req);
        assert_eq!(def.as_deref(), Some("hello"));
    }

    #[test]
    fn modifier_default_bool() {
        let (vt, _, def) = parse_type_with_modifiers("bool, default(true)").unwrap();
        assert_eq!(vt, ValueType::Bool);
        assert_eq!(def.as_deref(), Some("true"));
    }

    #[test]
    fn enum_with_required_does_not_split_enum_values() {
        let (vt, req, _) =
            parse_type_with_modifiers("enum(a, b, c), required").unwrap();
        assert_eq!(
            vt,
            ValueType::Enum(vec!["a".into(), "b".into(), "c".into()])
        );
        assert!(req);
    }

    // ── Inline comments ──

    #[test]
    fn inline_comment_stripped() {
        let schema = load("\
```grammar
document ::= H1(\"T\") item+
item     ::= H2(IDENT) property+
```

```types
@image : string  # podman image name
```
");
        let props = &schema.global_properties;
        assert_eq!(props["image"].value_type, ValueType::String);
    }

    // ── H3 nesting in grammar ──

    #[test]
    fn h3_nests_under_h2_in_schema() {
        let schema = load("\
```grammar
document ::= H1(\"Root\") parent
parent   ::= H2(\"Parent\") child+
child    ::= H3(IDENT) property+
```

```types:child
@cmd : string
```
");
        let parent = &schema.nodes[0].children[0];
        assert_eq!(parent.children.len(), 1);

        let child = &parent.children[0];
        assert_eq!(child.production_name.as_deref(), Some("child"));
        assert!(child.expects_properties);
        assert_eq!(child.level, 3);

        let child_props = child.properties.as_ref().unwrap();
        assert!(child_props.contains_key("cmd"));
    }

    // ── Name patterns ──

    #[test]
    fn snake_case_pattern() {
        let p = NamePattern::SnakeCase;
        assert!(p.matches("hello_world"));
        assert!(p.matches("a"));
        assert!(!p.matches("Hello"));
        assert!(!p.matches("hello-world"));
        assert!(!p.matches(""));
    }

    #[test]
    fn kebab_case_pattern() {
        let p = NamePattern::KebabCase;
        assert!(p.matches("hello-world"));
        assert!(!p.matches("hello_world"));
        assert!(!p.matches("Hello"));
    }

    #[test]
    fn ident_pattern() {
        let p = NamePattern::Ident;
        assert!(p.matches("hello"));
        assert!(p.matches("Hello"));
        assert!(p.matches("hello_world"));
        assert!(p.matches("hello-world"));
        assert!(!p.matches("123"));
        assert!(!p.matches(""));
    }

    #[test]
    fn exact_pattern() {
        let p = NamePattern::Exact("Container".into());
        assert!(p.matches("Container"));
        assert!(!p.matches("container"));
        assert!(!p.matches("Other"));
    }

    // ── Circular reference detection ──

    #[test]
    fn circular_reference_detected() {
        let err = load_schema("\
```grammar
document ::= a
a        ::= b
b        ::= a
```
", "test");
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("circular"));
    }

    // ── Grammar tokenizer ──

    #[test]
    fn tokenizer_respects_parens() {
        let tokens = tokenize_grammar("H1(\"Hello World\") item+");
        assert_eq!(tokens, vec!["H1(\"Hello World\")", "item+"]);
    }

    #[test]
    fn tokenizer_multiple_spaces() {
        let tokens = tokenize_grammar("a   b   c");
        assert_eq!(tokens, vec!["a", "b", "c"]);
    }

    // ── Full sozu-like schema ──

    #[test]
    fn sozu_schema_structure() {
        let schema = load("\
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
@schedule : string, required
@run      : string, required
@log      : bool, default(true)
```
");
        // Two H1 nodes
        assert_eq!(schema.nodes.len(), 2);
        assert_eq!(schema.nodes[0].text.as_deref(), Some("Agyo"));
        assert_eq!(schema.nodes[1].text.as_deref(), Some("Tasks"));
        assert!(matches!(schema.nodes[1].quantity, Quantity::Optional));

        // Agyo has Container and Servers
        assert_eq!(schema.nodes[0].children.len(), 2);
        let container = &schema.nodes[0].children[0];
        assert_eq!(container.production_name.as_deref(), Some("container"));
        let c_props = container.properties.as_ref().unwrap();
        assert!(c_props["image"].required);
        assert_eq!(c_props["network"].default.as_deref(), Some("outbound"));
        assert_eq!(c_props["mounts"].value_type, ValueType::List(Box::new(ValueType::String)));

        // Servers has server children
        let servers_sec = &schema.nodes[0].children[1];
        assert_eq!(servers_sec.children.len(), 1);
        let server = &servers_sec.children[0];
        assert_eq!(server.production_name.as_deref(), Some("server"));
        let s_props = server.properties.as_ref().unwrap();
        assert!(s_props["command"].required);

        // Tasks
        let task = &schema.nodes[1].children[0];
        assert!(task.expects_prose);
        assert!(task.expects_properties);
        let t_props = task.properties.as_ref().unwrap();
        assert_eq!(t_props["schedule"].value_type, ValueType::String);
        assert_eq!(t_props["log"].default.as_deref(), Some("true"));

        // No global properties
        assert!(schema.global_properties.is_empty());
    }
}
