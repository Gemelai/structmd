//! structmd-workflow — minimal workflow engine.
//!
//! Reads a workflow document, validates it against the schema,
//! and runs each step's command in dependency order.
//!
//! Usage: structmd-workflow <workflow.md>
//!
//! Exit codes:
//!   0 — all steps succeeded
//!   1 — validation error or step failure
//!   2 — usage error

include!(concat!(env!("OUT_DIR"), "/workflow_config.rs"));

use std::collections::HashSet;
use std::process::{Command, exit};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: structmd-workflow <workflow.md>");
        exit(2);
    }

    let path = &args[1];
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", path, e);
            exit(2);
        }
    };

    // The generated loader validates against the embedded schema before walking
    let config = match parse_with_source(&text, path) {
        Ok(c) => c,
        Err(errors) => {
            print!("{}", structmd::errors::render_vec("structmd-workflow", &errors));
            exit(1);
        }
    };

    // Run steps in document order, respecting depends
    let mut completed: HashSet<String> = HashSet::new();
    let mut failed: HashSet<String> = HashSet::new();

    for step in &config.step {
        // Check all deps completed successfully
        let blocked: Vec<&String> = step.depends.iter()
            .filter(|d| !completed.contains(*d))
            .collect();
        if !blocked.is_empty() {
            let blocking: Vec<&str> = blocked.iter()
                .filter(|d| failed.contains(**d))
                .map(|d| d.as_str())
                .collect();
            if !blocking.is_empty() {
                eprintln!("skip {}: depends on failed step(s): {}", step.name, blocking.join(", "));
            } else {
                eprintln!("skip {}: depends on unknown step(s): {}", step.name,
                    blocked.iter().map(|d| d.as_str()).collect::<Vec<_>>().join(", "));
            }
            failed.insert(step.name.clone());
            continue;
        }

        println!("run  {}: {}", step.name, step.command);
        let status = Command::new("sh")
            .args(["-c", &step.command])
            .status();

        match status {
            Ok(s) if s.success() => {
                completed.insert(step.name.clone());
            }
            Ok(s) => {
                eprintln!("fail {}: exit {}", step.name, s.code().unwrap_or(-1));
                failed.insert(step.name.clone());
            }
            Err(e) => {
                eprintln!("fail {}: {}", step.name, e);
                failed.insert(step.name.clone());
            }
        }
    }

    if failed.is_empty() {
        exit(0);
    } else {
        exit(1);
    }
}
