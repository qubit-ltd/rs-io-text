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

/// Extracts and generates the executable synchronous and asynchronous examples from four documents.
fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let project = manifest.join("../../..");
    let mut generated = String::from("mod user_guide_examples {\n");
    for (file, heading, name) in [
        ("README.md", "## Quick Start: Encode a UTF-8 Text Message", "readme_english"),
        ("README.zh_CN.md", "## 快速开始：编码一条 UTF-8 文本消息", "readme_chinese"),
        ("doc/user_guide.md", "## Core Workflow", "guide_english"),
        ("doc/user_guide.zh_CN.md", "## 核心流程", "guide_chinese"),
        ("doc/user_guide.md", "### Recovering unread text", "reader_recovery_english"),
        ("doc/user_guide.zh_CN.md", "### 恢复未读文本", "reader_recovery_chinese"),
    ] {
        let path = project.join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        let markdown = fs::read_to_string(&path).expect("read documented example");
        let code = extract_core(&markdown, heading);
        generated.push_str(&format!("pub mod {name} {{\n"));
        generated.push_str(code);
        generated.push_str("\npub fn run() -> std::io::Result<()> { main() }\n}\n");
    }
    for (file, heading, name, function) in [
        ("README.md", "## Runtime-Neutral Async", "readme_async_english", "write_message"),
        ("README.zh_CN.md", "## 运行时无关的异步 API", "readme_async_chinese", "write_message"),
        ("doc/user_guide.md", "## Async Workflow", "guide_async_english", "write_document"),
        ("doc/user_guide.zh_CN.md", "## 异步流程", "guide_async_chinese", "write_document"),
    ] {
        let path = project.join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        let markdown = fs::read_to_string(&path).expect("read documented async example");
        let code = extract_core(&markdown, heading);
        generated.push_str(&format!("pub mod {name} {{\n"));
        generated.push_str(code);
        generated.push_str(&format!(
            "\npub(crate) async fn run(output: crate::RecordingAsyncOutput) -> std::io::Result<crate::RecordingAsyncOutput> {{ {function}(output).await }}\n}}\n"
        ));
    }
    for (file, heading, name) in [
        ("doc/user_guide.md", "## Resuming an interrupted line read", "line_resume_english"),
        ("doc/user_guide.zh_CN.md", "## 恢复中断的行读取", "line_resume_chinese"),
    ] {
        let path = project.join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        let markdown = fs::read_to_string(&path).expect("read line recovery example");
        let code = extract_core(&markdown, heading);
        generated.push_str(&format!("pub mod {name} {{\n"));
        generated.push_str(code);
        generated.push_str("\n}\n");
    }
    for (file, heading, name) in [
        ("doc/user_guide.md", "## Complete In-Memory Conversion", "memory_english"),
        ("doc/user_guide.zh_CN.md", "## 完整内存转换", "memory_chinese"),
        ("doc/user_guide.md", "## Send Futures and Threaded Tasks", "send_english"),
        ("doc/user_guide.zh_CN.md", "## Send future 与跨线程任务", "send_chinese"),
    ] {
        let path = project.join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        let markdown = fs::read_to_string(&path).expect("read documented example");
        let code = extract_core(&markdown, heading);
        generated.push_str(&format!("pub mod {name} {{\n"));
        generated.push_str(code);
        generated.push_str("\n}\n");
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
