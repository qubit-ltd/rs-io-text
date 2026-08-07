// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use qubit_io_text::LineEnding;
use qubit_io_text::TextWrite;
use qubit_io_text::Utf8TextWriter;

/// Compiles the README quick-start example with only its documented dependency.
fn main() -> std::io::Result<()> {
    let mut bytes = Vec::new();
    let mut writer = Utf8TextWriter::new(&mut bytes)
        .with_line_ending(LineEnding::CrLf);

    writer.write_line("hello")?;
    writer.write_str("中文")?;
    writer.finish()?;

    let (output, pending) = writer.into_parts();
    assert!(pending.readable().is_empty());
    assert_eq!("hello\r\n中文".as_bytes(), output.as_slice());
    Ok(())
}
