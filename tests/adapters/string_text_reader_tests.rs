// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::convert::Infallible;

use qubit_io_text::LineEndingSet;
use qubit_io_text::StringTextReader;
use qubit_io_text::TextLineRead;
use qubit_io_text::TextRead;

#[test]
fn test_from_string_reads_owned_text() -> Result<(), std::convert::Infallible> {
    let mut reader = StringTextReader::new("alpha\nβeta".to_owned());
    let mut line = String::new();

    assert_eq!(0, reader.position());
    assert!(reader.read_line(&mut line)?);
    assert_eq!("alpha\n", line);
    assert_eq!(6, reader.position());
    assert_eq!(Some('β'), reader.read_char()?);
    Ok(())
}

#[test]
fn test_read_line_accepts_common_endings_by_default() -> Result<(), Infallible> {
    let mut reader = StringTextReader::new("a\rb\r\nc\nend".to_owned());
    let mut line = String::new();

    assert_eq!(LineEndingSet::ALL, reader.line_endings());
    assert!(reader.read_line(&mut line)?);
    assert_eq!("a\r", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("b\r\n", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("c\n", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("end", line);
    Ok(())
}

#[test]
fn test_read_line_pending_character_is_returned_by_other_read_methods() -> Result<(), Infallible> {
    let mut reader = StringTextReader::new("first\rsecond".to_owned());
    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    assert_eq!(Some('s'), reader.read_char()?);

    let mut reader = StringTextReader::new("first\rsecond".to_owned());
    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    let mut chars = Vec::new();
    assert_eq!(1, reader.read_chars(&mut chars, 1)?);
    assert_eq!(vec!['s'], chars);

    let mut reader = StringTextReader::new("first\rsecond".to_owned());
    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    let mut output = String::new();
    assert_eq!(6, reader.read_to_string(&mut output)?);
    assert_eq!("second", output);
    Ok(())
}

#[test]
fn test_read_line_crlf_only_handles_lone_cr_and_eof() -> Result<(), Infallible> {
    let mut reader = StringTextReader::new("first\rsecond\r\nlast".to_owned()).with_line_endings(LineEndingSet::CRLF);
    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("first\rsecond\r\n", line);

    let mut reader = StringTextReader::new("tail\r".to_owned()).with_line_endings(LineEndingSet::CRLF);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("tail\r", line);
    Ok(())
}

#[test]
fn test_read_chars_reads_owned_text() -> Result<(), std::convert::Infallible> {
    let mut reader = StringTextReader::new("ab中".to_owned());
    let mut chars = Vec::new();

    assert_eq!(2, reader.read_chars(&mut chars, 2)?);
    assert_eq!(vec!['a', 'b'], chars);
    assert_eq!(1, reader.read_chars(&mut chars, 8)?);
    assert_eq!(vec!['a', 'b', '中'], chars);
    Ok(())
}

#[test]
fn test_read_chars_with_zero_max_does_not_advance() -> Result<(), std::convert::Infallible> {
    let mut reader = StringTextReader::new("ab".to_owned());
    let mut chars = vec!['x'];

    assert_eq!(0, reader.read_chars(&mut chars, 0)?);
    assert_eq!(vec!['x'], chars);
    assert_eq!(0, reader.position());
    Ok(())
}

#[test]
fn test_read_to_string_appends_remaining_owned_text() -> Result<(), std::convert::Infallible> {
    let mut reader = StringTextReader::new("ab中".to_owned());
    let mut output = String::from("prefix:");

    assert_eq!(3, reader.read_to_string(&mut output)?);
    assert_eq!("prefix:ab中", output);
    assert_eq!(5, reader.position());
    assert_eq!(0, reader.read_to_string(&mut output)?);
    Ok(())
}

#[test]
fn test_read_line_returns_false_at_eof() -> Result<(), std::convert::Infallible> {
    let mut reader = StringTextReader::new(String::new());
    let mut line = String::from("seed");

    assert!(!reader.read_line(&mut line)?);
    assert_eq!("seed", line);
    Ok(())
}

#[test]
fn test_into_inner_returns_original_text() {
    let reader = StringTextReader::new("payload".to_owned());

    assert_eq!("payload", reader.into_inner());
}

#[test]
fn test_string_text_reader_is_infallible() {
    fn assert_infallible<T>(_: &T)
    where
        T: TextRead<Error = std::convert::Infallible>,
    {
    }

    let reader = StringTextReader::new("payload".to_owned());

    assert_infallible(&reader);
}
