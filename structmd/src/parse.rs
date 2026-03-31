/// conf.md parser — produces an untyped Document tree from markdown text.
///
/// The parser is permissive: it extracts structure without validating.
/// Validation is done separately against a schema.

#[derive(Debug)]
pub struct Document {
    pub nodes: Vec<H1Node>,
}

#[derive(Debug)]
pub struct H1Node {
    pub heading: Option<Heading>,
    pub prose: Option<String>,
    pub properties: Vec<Property>,
    pub sections: Vec<Section>,
}

#[derive(Debug)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub line: usize,
}

#[derive(Debug)]
pub struct Section {
    pub heading: Heading,
    pub prose: Option<String>,
    pub prose_line: Option<usize>,
    pub properties: Vec<Property>,
    pub table: Option<Table>,
    pub children: Vec<Section>,
}

#[derive(Debug)]
pub struct Property {
    pub key: String,
    pub value: String,
    pub bold: bool,
    pub line: usize,
}

#[derive(Debug)]
pub struct Table {
    pub header_line: usize,
    pub columns: Vec<String>,
    pub rows: Vec<TableRow>,
}

#[derive(Debug)]
pub struct TableRow {
    pub line: usize,
    pub cells: Vec<String>,
}

pub fn parse(text: &str) -> Document {
    let lines: Vec<&str> = text.lines().collect();
    let mut doc = Document { nodes: Vec::new() };

    let mut current_h2: Option<Section> = None;
    let mut current_h3: Option<Section> = None;
    let mut in_table = false;
    let mut in_code_fence = false;
    let mut expect_separator = false;
    let mut code_fence_key: Option<String> = None;
    let mut code_fence_buf: Vec<String> = Vec::new();
    let mut code_fence_start: usize = 0;
    let mut prose_buf: Vec<String> = Vec::new();
    let mut prose_start: Option<usize> = None;
    let mut collecting_prose = false;

    for (i, line) in lines.iter().enumerate() {
        let lineno = i + 1;
        let trimmed = line.trim();

        // Fenced code blocks — capture as multi-line properties if info string present
        if trimmed.starts_with("```") {
            if in_code_fence {
                // Closing fence — flush captured block
                in_code_fence = false;
                if let Some(key) = code_fence_key.take() {
                    let prop = Property {
                        key,
                        value: code_fence_buf.drain(..).collect::<Vec<_>>().join("\n"),
                        bold: false,
                        line: code_fence_start,
                    };
                    if let Some(sec) = current_h3.as_mut().or(current_h2.as_mut()) {
                        sec.properties.push(prop);
                    } else if let Some(node) = doc.nodes.last_mut() {
                        node.properties.push(prop);
                    }
                }
                continue;
            }
            // Opening fence
            let info = trimmed[3..].trim();
            in_code_fence = true;
            if !info.is_empty() && !info.contains(' ') {
                code_fence_key = Some(info.to_string());
                code_fence_start = lineno;
                code_fence_buf.clear();
            } else {
                code_fence_key = None;
            }
            continue;
        }
        if in_code_fence {
            if code_fence_key.is_some() {
                code_fence_buf.push(line.to_string());
            }
            continue;
        }

        // H1 heading — start a new node
        if trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
            // Flush prose into current target
            flush_prose(&prose_buf, prose_start, &mut current_h3, &mut current_h2, &mut doc);
            prose_buf.clear();
            prose_start = None;

            flush_h3(&mut current_h2, &mut current_h3);
            if let Some(sec) = current_h2.take() {
                ensure_node(&mut doc).sections.push(sec);
            }

            doc.nodes.push(H1Node {
                heading: Some(Heading {
                    level: 1,
                    text: trimmed[2..].trim().to_string(),
                    line: lineno,
                }),
                prose: None,
                properties: Vec::new(),
                sections: Vec::new(),
            });
            collecting_prose = true;
            in_table = false;
            expect_separator = false;
            continue;
        }

        // H3 heading — child of current H2
        if let Some(name) = trimmed.strip_prefix("### ") {
            flush_prose(&prose_buf, prose_start, &mut current_h3, &mut current_h2, &mut doc);
            prose_buf.clear();
            prose_start = None;

            flush_h3(&mut current_h2, &mut current_h3);

            current_h3 = Some(Section {
                heading: Heading {
                    level: 3,
                    text: name.trim().to_string(),
                    line: lineno,
                },
                prose: None,
                prose_line: None,
                properties: Vec::new(),
                table: None,
                children: Vec::new(),
            });
            collecting_prose = true;
            in_table = false;
            expect_separator = false;
            continue;
        }

        // H2 heading — new section
        if let Some(name) = trimmed.strip_prefix("## ") {
            flush_prose(&prose_buf, prose_start, &mut current_h3, &mut current_h2, &mut doc);
            prose_buf.clear();
            prose_start = None;

            flush_h3(&mut current_h2, &mut current_h3);
            if let Some(sec) = current_h2.take() {
                ensure_node(&mut doc).sections.push(sec);
            }

            current_h2 = Some(Section {
                heading: Heading {
                    level: 2,
                    text: name.trim().to_string(),
                    line: lineno,
                },
                prose: None,
                prose_line: None,
                properties: Vec::new(),
                table: None,
                children: Vec::new(),
            });
            collecting_prose = true;
            in_table = false;
            expect_separator = false;
            continue;
        }

        // Accumulate into deepest active section, or H1 node if no H2 active
        if current_h3.is_none() && current_h2.is_none() {
            // No active H2/H3
            if let Some(rest) = trimmed.strip_prefix("- ") {
                // Structure — stop collecting prose
                flush_prose(&prose_buf, prose_start, &mut current_h3, &mut current_h2, &mut doc);
                prose_buf.clear();
                prose_start = None;
                collecting_prose = false;
                if let Some(prop) = parse_property(rest, lineno) {
                    if let Some(node) = doc.nodes.last_mut() {
                        node.properties.push(prop);
                    }
                }
            } else if collecting_prose {
                if !trimmed.is_empty() {
                    if prose_start.is_none() {
                        prose_start = Some(lineno);
                    }
                    prose_buf.push(trimmed.to_string());
                } else if prose_start.is_some() {
                    // blank line within prose — preserve as paragraph break
                    prose_buf.push(String::new());
                }
            }
            continue;
        }

        let sec = current_h3.as_mut().or(current_h2.as_mut()).unwrap();

        // Property line — structure, stops prose collection
        if let Some(rest) = trimmed.strip_prefix("- ") {
            if let Some(prop) = parse_property(rest, lineno) {
                if collecting_prose {
                    flush_prose_into_sec(sec, &prose_buf, prose_start);
                    prose_buf.clear();
                    prose_start = None;
                    collecting_prose = false;
                }
                sec.properties.push(prop);
                in_table = false;
                continue;
            }
        }

        // Table rows — structure, stops prose collection
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            if collecting_prose {
                flush_prose_into_sec(sec, &prose_buf, prose_start);
                prose_buf.clear();
                prose_start = None;
                collecting_prose = false;
            }

            let cells: Vec<String> = trimmed
                .trim_matches('|')
                .split('|')
                .map(|s| s.trim().to_string())
                .collect();

            let is_separator = cells
                .iter()
                .all(|c| c.chars().all(|ch| ch == '-' || ch == ' ' || ch == ':'));

            if expect_separator {
                expect_separator = false;
                if is_separator {
                    continue;
                }
            }

            if is_separator {
                continue;
            }

            if !in_table {
                in_table = true;
                expect_separator = true;
                sec.table = Some(Table {
                    header_line: lineno,
                    columns: cells,
                    rows: Vec::new(),
                });
                continue;
            }

            if let Some(ref mut table) = sec.table {
                table.rows.push(TableRow {
                    line: lineno,
                    cells,
                });
            }
            continue;
        }

        // Prose — collect if before structure
        if !in_table && collecting_prose {
            if !trimmed.is_empty() {
                if prose_start.is_none() {
                    prose_start = Some(lineno);
                }
                prose_buf.push(trimmed.to_string());
            } else if prose_start.is_some() {
                // blank line within prose — preserve as paragraph break
                prose_buf.push(String::new());
            }
        }
    }

    // Flush remaining prose and sections
    flush_prose(&prose_buf, prose_start, &mut current_h3, &mut current_h2, &mut doc);
    flush_h3(&mut current_h2, &mut current_h3);
    if let Some(sec) = current_h2.take() {
        ensure_node(&mut doc).sections.push(sec);
    }

    doc
}

fn ensure_node(doc: &mut Document) -> &mut H1Node {
    if doc.nodes.is_empty() {
        doc.nodes.push(H1Node {
            heading: None,
            prose: None,
            properties: Vec::new(),
            sections: Vec::new(),
        });
    }
    doc.nodes.last_mut().unwrap()
}

fn flush_prose_into_sec(sec: &mut Section, buf: &[String], start: Option<usize>) {
    let trimmed = trim_prose(buf);
    if !trimmed.is_empty() {
        sec.prose = Some(trimmed);
        sec.prose_line = start;
    }
}

fn flush_prose(
    buf: &[String],
    start: Option<usize>,
    h3: &mut Option<Section>,
    h2: &mut Option<Section>,
    doc: &mut Document,
) {
    let text = trim_prose(buf);
    if text.is_empty() {
        return;
    }
    if let Some(sec) = h3.as_mut().or(h2.as_mut()) {
        sec.prose = Some(text);
        sec.prose_line = start;
    } else if let Some(node) = doc.nodes.last_mut() {
        node.prose = Some(text);
    }
}

fn trim_prose(buf: &[String]) -> String {
    // Strip leading and trailing blank lines, preserve internal ones
    let start = buf.iter().position(|s| !s.is_empty()).unwrap_or(buf.len());
    let end = buf.iter().rposition(|s| !s.is_empty()).map(|i| i + 1).unwrap_or(0);
    if start >= end {
        return String::new();
    }
    buf[start..end].join("\n")
}

fn flush_h3(h2: &mut Option<Section>, h3: &mut Option<Section>) {
    if let Some(child) = h3.take() {
        if let Some(parent) = h2.as_mut() {
            parent.children.push(child);
        }
    }
}

fn parse_property(text: &str, line: usize) -> Option<Property> {
    // Bold style: **key:** value
    if let Some(rest) = text.strip_prefix("**") {
        if let Some(colon_pos) = rest.find(":**") {
            let key = &rest[..colon_pos];
            let value = rest[colon_pos + 3..].trim();
            return Some(Property {
                key: key.to_string(),
                value: value.to_string(),
                bold: true,
                line,
            });
        }
    }

    // Plain style: key: value
    if let Some(colon_pos) = text.find(": ") {
        let key = &text[..colon_pos];
        if !key.is_empty() && !key.contains(' ') {
            let value = text[colon_pos + 2..].trim();
            return Some(Property {
                key: key.to_string(),
                value: value.to_string(),
                bold: false,
                line,
            });
        }
    }

    // Plain style: key: (no value)
    if let Some(key) = text.strip_suffix(':') {
        if !key.is_empty() && !key.contains(' ') {
            return Some(Property {
                key: key.to_string(),
                value: String::new(),
                bold: false,
                line,
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Basic structure ──

    #[test]
    fn empty_document() {
        let doc = parse("");
        assert!(doc.nodes.is_empty());
    }

    #[test]
    fn single_h1() {
        let doc = parse("# Title\n");
        assert_eq!(doc.nodes.len(), 1);
        assert_eq!(doc.nodes[0].heading.as_ref().unwrap().text, "Title");
        assert_eq!(doc.nodes[0].heading.as_ref().unwrap().line, 1);
        assert!(doc.nodes[0].sections.is_empty());
    }

    #[test]
    fn multi_h1() {
        let doc = parse("# First\n\n## sec1\n\n# Second\n\n## sec2\n");
        assert_eq!(doc.nodes.len(), 2);
        assert_eq!(doc.nodes[0].heading.as_ref().unwrap().text, "First");
        assert_eq!(doc.nodes[0].sections.len(), 1);
        assert_eq!(doc.nodes[0].sections[0].heading.text, "sec1");
        assert_eq!(doc.nodes[1].heading.as_ref().unwrap().text, "Second");
        assert_eq!(doc.nodes[1].sections.len(), 1);
        assert_eq!(doc.nodes[1].sections[0].heading.text, "sec2");
    }

    #[test]
    fn h2_before_any_h1() {
        let doc = parse("## orphan\n- key: value\n");
        assert_eq!(doc.nodes.len(), 1);
        assert!(doc.nodes[0].heading.is_none()); // implicit node
        assert_eq!(doc.nodes[0].sections.len(), 1);
        assert_eq!(doc.nodes[0].sections[0].heading.text, "orphan");
    }

    // ── H3 nesting ──

    #[test]
    fn h3_nests_under_h2() {
        let doc = parse("# Top\n\n## Parent\n\n### Child1\n- a: 1\n\n### Child2\n- b: 2\n");
        let parent = &doc.nodes[0].sections[0];
        assert_eq!(parent.heading.text, "Parent");
        assert_eq!(parent.children.len(), 2);
        assert_eq!(parent.children[0].heading.text, "Child1");
        assert_eq!(parent.children[1].heading.text, "Child2");
        assert_eq!(parent.children[0].properties[0].key, "a");
        assert_eq!(parent.children[1].properties[0].key, "b");
    }

    // ── Properties ──

    #[test]
    fn plain_property() {
        let doc = parse("# T\n## S\n- key: value\n");
        let props = &doc.nodes[0].sections[0].properties;
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].key, "key");
        assert_eq!(props[0].value, "value");
        assert!(!props[0].bold);
    }

    #[test]
    fn bold_property() {
        let doc = parse("# T\n## S\n- **server:** fetch\n");
        let props = &doc.nodes[0].sections[0].properties;
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].key, "server");
        assert_eq!(props[0].value, "fetch");
        assert!(props[0].bold);
    }

    #[test]
    fn label_property_no_value() {
        let doc = parse("# T\n## S\n- parameters:\n");
        let props = &doc.nodes[0].sections[0].properties;
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].key, "parameters");
        assert_eq!(props[0].value, "");
    }

    #[test]
    fn property_with_spaces_in_value() {
        let doc = parse("# T\n## S\n- cmd: echo hello world\n");
        let props = &doc.nodes[0].sections[0].properties;
        assert_eq!(props[0].value, "echo hello world");
    }

    #[test]
    fn list_item_with_spaces_not_property() {
        // "- some text here" should not parse as a property (key has spaces)
        let doc = parse("# T\n## S\n- some text here\n");
        assert!(doc.nodes[0].sections[0].properties.is_empty());
    }

    // ── Prose ──

    #[test]
    fn prose_captured() {
        let doc = parse("# T\n## S\n\nThis is a description.\n\n- key: val\n");
        let sec = &doc.nodes[0].sections[0];
        assert_eq!(sec.prose.as_deref(), Some("This is a description."));
        assert_eq!(sec.prose_line, Some(4));
    }

    #[test]
    fn no_prose() {
        let doc = parse("# T\n## S\n- key: val\n");
        assert!(doc.nodes[0].sections[0].prose.is_none());
    }

    #[test]
    fn multiline_prose() {
        let doc = parse("# T\n## S\n\nLine one.\nLine two.\n\n- key: val\n");
        let sec = &doc.nodes[0].sections[0];
        assert_eq!(sec.prose.as_deref(), Some("Line one.\nLine two."));
    }

    #[test]
    fn prose_stops_at_property() {
        let doc = parse("# T\n## S\nProse here.\n- key: val\nNot prose.\n");
        let sec = &doc.nodes[0].sections[0];
        assert_eq!(sec.prose.as_deref(), Some("Prose here."));
    }

    #[test]
    fn prose_on_h1() {
        let doc = parse("# Title\n\nSome prose on the H1.\n\n## Section\n");
        assert_eq!(doc.nodes[0].prose.as_deref(), Some("Some prose on the H1."));
    }

    // ── Tables ──

    #[test]
    fn basic_table() {
        let input = "# T\n## S\n| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n";
        let doc = parse(input);
        let table = doc.nodes[0].sections[0].table.as_ref().unwrap();
        assert_eq!(table.columns, vec!["a", "b"]);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells, vec!["1", "2"]);
        assert_eq!(table.rows[1].cells, vec!["3", "4"]);
    }

    #[test]
    fn table_with_alignment_separators() {
        let input = "# T\n## S\n| a | b |\n|:--|--:|\n| 1 | 2 |\n";
        let doc = parse(input);
        let table = doc.nodes[0].sections[0].table.as_ref().unwrap();
        assert_eq!(table.columns, vec!["a", "b"]);
        assert_eq!(table.rows.len(), 1);
    }

    #[test]
    fn no_table() {
        let doc = parse("# T\n## S\n- key: val\n");
        assert!(doc.nodes[0].sections[0].table.is_none());
    }

    // ── Code fence skipping ──

    #[test]
    fn headings_inside_code_fence_ignored() {
        let input = "# Real\n## Also Real\n```\n# Fake\n## Also Fake\n- fake: prop\n```\n";
        let doc = parse(input);
        assert_eq!(doc.nodes.len(), 1);
        assert_eq!(doc.nodes[0].heading.as_ref().unwrap().text, "Real");
        assert_eq!(doc.nodes[0].sections.len(), 1);
        assert_eq!(doc.nodes[0].sections[0].heading.text, "Also Real");
        assert!(doc.nodes[0].sections[0].properties.is_empty());
    }

    #[test]
    fn table_inside_code_fence_ignored() {
        let input = "# T\n## S\n```\n| a | b |\n|---|---|\n| 1 | 2 |\n```\n";
        let doc = parse(input);
        assert!(doc.nodes[0].sections[0].table.is_none());
    }

    // ── Fenced code blocks as properties ──

    #[test]
    fn tagged_code_block_becomes_property() {
        let input = "# T\n## S\n```system_prompt\nYou are Weid.\nBe brief.\n```\n";
        let doc = parse(input);
        let props = &doc.nodes[0].sections[0].properties;
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].key, "system_prompt");
        assert_eq!(props[0].value, "You are Weid.\nBe brief.");
        assert!(!props[0].bold);
    }

    #[test]
    fn untagged_code_block_not_captured() {
        let input = "# T\n## S\n```\nsome code\n```\n";
        let doc = parse(input);
        assert!(doc.nodes[0].sections[0].properties.is_empty());
    }

    #[test]
    fn code_block_preserves_blank_lines() {
        let input = "# T\n## S\n```prompt\nLine one.\n\nLine three.\n```\n";
        let doc = parse(input);
        let prop = &doc.nodes[0].sections[0].properties[0];
        assert_eq!(prop.value, "Line one.\n\nLine three.");
    }

    #[test]
    fn code_block_preserves_markdown_inside() {
        let input = "# T\n## S\n```prompt\n# Not a heading\n- not: a property\n| not | a table |\n```\n";
        let doc = parse(input);
        let prop = &doc.nodes[0].sections[0].properties[0];
        assert!(prop.value.contains("# Not a heading"));
        assert!(prop.value.contains("- not: a property"));
        assert!(prop.value.contains("| not | a table |"));
        // Should not have parsed as actual structure
        assert_eq!(doc.nodes.len(), 1);
        assert_eq!(doc.nodes[0].sections[0].properties.len(), 1);
        assert!(doc.nodes[0].sections[0].table.is_none());
    }

    #[test]
    fn multiple_code_blocks_on_same_section() {
        let input = "# T\n## S\n```greeting\nHello\n```\n```farewell\nGoodbye\n```\n";
        let doc = parse(input);
        let props = &doc.nodes[0].sections[0].properties;
        assert_eq!(props.len(), 2);
        assert_eq!(props[0].key, "greeting");
        assert_eq!(props[0].value, "Hello");
        assert_eq!(props[1].key, "farewell");
        assert_eq!(props[1].value, "Goodbye");
    }

    // ── Line numbers ──

    #[test]
    fn line_numbers_correct() {
        let input = "# Title\n\n## Section\n\n- key: val\n";
        let doc = parse(input);
        assert_eq!(doc.nodes[0].heading.as_ref().unwrap().line, 1);
        assert_eq!(doc.nodes[0].sections[0].heading.line, 3);
        assert_eq!(doc.nodes[0].sections[0].properties[0].line, 5);
    }

    // ── Full document (sozu-like) ──

    #[test]
    fn sozu_structure() {
        let input = "\
# Agyo

## Container
- image: localhost/test:latest
- network: outbound
- mounts: /tmp/a, /tmp/b

## Servers

### fetch
- command: /usr/local/bin/fetch

### brave
- command: /usr/local/bin/brave
- secrets: key1

# Tasks

## heartbeat

A test task.

- schedule: every 5m
- run: echo hi
- log: true
";
        let doc = parse(input);
        assert_eq!(doc.nodes.len(), 2);

        // Agyo
        let agyo = &doc.nodes[0];
        assert_eq!(agyo.heading.as_ref().unwrap().text, "Agyo");
        assert_eq!(agyo.sections.len(), 2);

        let container = &agyo.sections[0];
        assert_eq!(container.heading.text, "Container");
        assert_eq!(container.properties.len(), 3);

        let servers = &agyo.sections[1];
        assert_eq!(servers.heading.text, "Servers");
        assert_eq!(servers.children.len(), 2);
        assert_eq!(servers.children[0].heading.text, "fetch");
        assert_eq!(servers.children[1].heading.text, "brave");
        assert_eq!(servers.children[1].properties.len(), 2);

        // Tasks
        let tasks = &doc.nodes[1];
        assert_eq!(tasks.heading.as_ref().unwrap().text, "Tasks");
        assert_eq!(tasks.sections.len(), 1);

        let heartbeat = &tasks.sections[0];
        assert_eq!(heartbeat.heading.text, "heartbeat");
        assert_eq!(heartbeat.prose.as_deref(), Some("A test task."));
        assert_eq!(heartbeat.properties.len(), 3);
    }
}
