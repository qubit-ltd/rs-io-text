// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::env;
use std::fs;
use std::path::PathBuf;

/// Extracts and generates the executable core examples from both user guides.
fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let project = manifest.join("../../..");
    let mut generated = String::from("mod user_guide_examples {\n");
    for (file, heading, name) in [
        ("user_guide.md", "## Core Workflow", "english"),
        ("user_guide.zh_CN.md", "## 核心流程", "chinese"),
    ] {
        let path = project.join("doc").join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        let markdown = fs::read_to_string(&path).expect("read user guide");
        let code = extract_core(&markdown, heading);
        generated.push_str(&format!("pub fn {name}() -> std::io::Result<()> {{\n"));
        for line in code.lines() {
            generated.push_str(line.strip_prefix("# ").unwrap_or(line));
            generated.push('\n');
        }
        generated.push_str("}\n");
    }
    generated.push_str("}\n");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    fs::write(output.join("user_guide_examples.rs"), generated).expect("write generated guide examples");
}

/// Finds the unique Rust code block in a uniquely headed Markdown section.
fn extract_core<'a>(markdown: &'a str, heading: &str) -> &'a str {
    let mut matches = markdown.match_indices(heading);
    let (position, _) = matches.next().expect("required core workflow heading");
    assert!(matches.next().is_none(), "core workflow heading must be unique");
    let tail = &markdown[position + heading.len()..];
    let section = tail.split("\n## ").next().expect("core workflow section");
    let mut blocks = section.split("```rust\n");
    blocks.next();
    let block = blocks.next().expect("core workflow Rust example");
    assert!(blocks.next().is_none(), "core workflow must have one Rust example");
    block.split_once("\n```").expect("closed Rust code fence").0
}
