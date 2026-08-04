// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::convert::Infallible;

use qubit_io_text::{
    LineEnding,
    LineEndingSet,
    StrTextReader,
    TextLineRead,
    TextRead,
};

#[test]
fn test_read_char_returns_unicode_scalars() -> Result<(), Infallible> {
    let mut reader = StrTextReader::new("a中🙂");

    assert_eq!(0, reader.position());
    assert_eq!(Some('a'), reader.read_char()?);
    assert_eq!(1, reader.position());
    assert_eq!(Some('中'), reader.read_char()?);
    assert_eq!(Some('🙂'), reader.read_char()?);
    assert_eq!(None, reader.read_char()?);
    Ok(())
}

#[test]
fn test_read_line_accepts_all_common_line_endings_by_default()
-> Result<(), Infallible> {
    let mut reader = StrTextReader::new("lf\ncrlf\r\ncr\rtail");
    let mut line = String::new();

    assert!(reader.read_line(&mut line)?);
    assert_eq!("lf\n", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("crlf\r\n", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("cr\r", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("tail", line);
    Ok(())
}

#[test]
fn test_read_line_honors_configured_line_endings() -> Result<(), Infallible> {
    let mut reader = StrTextReader::new("first\rsecond\nthird")
        .with_line_endings(LineEndingSet::only(LineEnding::Cr));
    let mut line = String::new();

    assert!(reader.read_line(&mut line)?);
    assert_eq!("first\r", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("second\nthird", line);
    Ok(())
}

#[test]
fn test_read_line_pending_character_is_returned_by_other_read_methods()
-> Result<(), Infallible> {
    let mut reader = StrTextReader::new("first\rsecond")
        .with_line_endings(LineEndingSet::ALL);
    assert_eq!(LineEndingSet::ALL, reader.line_endings());
    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("first\r", line);
    assert_eq!(Some('s'), reader.read_char()?);

    let mut chars = Vec::new();
    let mut reader = StrTextReader::new("first\rsecond");
    assert!(reader.read_line(&mut line)?);
    assert_eq!(1, reader.read_chars(&mut chars, 1)?);
    assert_eq!(vec!['s'], chars);

    let mut reader = StrTextReader::new("first\rsecond");
    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    let mut output = String::new();
    assert_eq!(
        1 + "econd".chars().count(),
        reader.read_to_string(&mut output)?
    );
    assert_eq!("second", output);
    Ok(())
}

#[test]
fn test_read_line_crlf_only_handles_lone_cr_and_eof() -> Result<(), Infallible>
{
    let mut reader = StrTextReader::new("first\rsecond\r\nlast")
        .with_line_endings(LineEndingSet::CRLF);
    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("first\rsecond\r\n", line);

    let mut reader =
        StrTextReader::new("tail\r").with_line_endings(LineEndingSet::CRLF);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("tail\r", line);

    let mut reader = StrTextReader::new("tail\r");
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("tail\r", line);
    Ok(())
}

#[test]
fn test_read_chars_reads_at_most_requested_count() -> Result<(), Infallible> {
    let mut reader = StrTextReader::new("ab中🙂");
    let mut chars = Vec::new();

    assert_eq!(0, reader.read_chars(&mut chars, 0)?);
    assert_eq!(3, reader.read_chars(&mut chars, 3)?);
    assert_eq!(vec!['a', 'b', '中'], chars);
    assert_eq!(1, reader.read_chars(&mut chars, 8)?);
    assert_eq!(vec!['a', 'b', '中', '🙂'], chars);
    assert_eq!(0, reader.read_chars(&mut chars, 8)?);
    Ok(())
}

#[test]
fn test_read_line_appends_line_with_terminator() -> Result<(), Infallible> {
    let mut reader = StrTextReader::new("first\r\nsecond\nlast");
    let mut line = String::from("prefix:");

    assert!(reader.read_line(&mut line)?);
    assert_eq!("prefix:first\r\n", line);

    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("second\n", line);

    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("last", line);

    line.clear();
    assert!(!reader.read_line(&mut line)?);
    assert!(line.is_empty());
    Ok(())
}

#[test]
fn test_read_to_string_appends_remaining_text() -> Result<(), Infallible> {
    let mut reader = StrTextReader::new("a中🙂");
    assert_eq!(Some('a'), reader.read_char()?);

    let mut output = String::from("prefix:");
    assert_eq!(2, reader.read_to_string(&mut output)?);
    assert_eq!("prefix:中🙂", output);
    assert_eq!(0, reader.read_to_string(&mut output)?);
    Ok(())
}
