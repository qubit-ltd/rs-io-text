// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io_text::{TextLineRead, TextRead, TextWrite};

#[derive(Debug, Eq, PartialEq)]
struct ReadError;

struct FailingTextReader;

impl TextRead for FailingTextReader {
    type Error = ReadError;

    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        Err(ReadError)
    }
}

struct EmptyTextReader;

impl TextRead for EmptyTextReader {
    type Error = ReadError;

    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        Ok(None)
    }
}

struct SequenceTextReader {
    chars: std::vec::IntoIter<char>,
}

impl SequenceTextReader {
    fn new(text: &str) -> Self {
        Self {
            chars: text.chars().collect::<Vec<_>>().into_iter(),
        }
    }
}

impl TextRead for SequenceTextReader {
    type Error = ReadError;

    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        Ok(self.chars.next())
    }
}

impl TextLineRead for SequenceTextReader {}

impl TextLineRead for FailingTextReader {}

#[derive(Debug, Eq, PartialEq)]
struct WriteError;

struct FailingTextWriter;

impl TextWrite for FailingTextWriter {
    type Error = WriteError;

    fn write_str(&mut self, _text: &str) -> Result<(), Self::Error> {
        Err(WriteError)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct FailOnSecondWrite {
    calls: usize,
}

#[derive(Default)]
struct MinimalTextWriter {
    output: String,
}

impl TextWrite for MinimalTextWriter {
    type Error = WriteError;

    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        self.output.push_str(text);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl TextWrite for FailOnSecondWrite {
    type Error = WriteError;

    fn write_str(&mut self, _text: &str) -> Result<(), Self::Error> {
        self.calls += 1;
        if self.calls == 2 {
            return Err(WriteError);
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn test_default_read_methods_append_requested_text() {
    let mut reader = SequenceTextReader::new("ab中");
    let mut chars = vec![':'];
    let mut text = String::from("prefix:");

    assert_eq!(Ok(2), reader.read_chars(&mut chars, 2));
    assert_eq!(&[':', 'a', 'b'], chars.as_slice());
    assert_eq!(Ok(1), reader.read_to_string(&mut text));
    assert_eq!("prefix:中", text);
}

#[test]
fn test_default_read_methods_cover_zero_limit_and_errors() {
    let mut reader = SequenceTextReader::new("A");
    let mut chars = Vec::new();
    assert_eq!(Ok(0), reader.read_chars(&mut chars, 0));
    assert_eq!(Ok(Some('A')), reader.read_char());

    let mut reader = FailingTextReader;
    assert_eq!(Err(ReadError), reader.read_to_string(&mut String::new()));
    assert_eq!(Err(ReadError), reader.read_line(&mut String::new()));
}

#[test]
fn test_default_read_line_preserves_terminator_and_eof() {
    let mut reader = SequenceTextReader::new("first\nlast");
    let mut output = String::new();

    assert_eq!(Ok(true), reader.read_line(&mut output));
    assert_eq!("first\n", output);
    output.clear();
    assert_eq!(Ok(true), reader.read_line(&mut output));
    assert_eq!("last", output);
    output.clear();
    assert_eq!(Ok(false), reader.read_line(&mut output));
}

#[test]
fn test_read_chars_propagates_read_errors() {
    let mut reader = FailingTextReader;
    let mut chars = Vec::new();

    assert_eq!(Err(ReadError), reader.read_chars(&mut chars, 1));
    assert!(chars.is_empty());
}

#[test]
fn test_read_chars_stops_at_eof_without_appending() {
    let mut reader = EmptyTextReader;
    let mut chars = Vec::new();

    assert_eq!(Ok(0), reader.read_chars(&mut chars, 1));
    assert!(chars.is_empty());
}

#[test]
fn test_write_char_and_chars_propagate_write_errors() {
    let mut writer = FailingTextWriter;

    assert_eq!(Err(WriteError), writer.write_char('x'));
    assert_eq!(Err(WriteError), writer.write_chars(&['x']));
    assert_eq!(Err(WriteError), writer.write_line("line"));
}

#[test]
fn test_write_line_propagates_line_ending_errors() {
    let mut writer = FailOnSecondWrite { calls: 0 };

    assert_eq!(Err(WriteError), writer.write_line("line"));
    assert_eq!(2, writer.calls);
}

#[test]
fn test_default_write_methods_write_complete_text() {
    let mut writer = MinimalTextWriter::default();

    assert_eq!(Ok(()), writer.write_char('中'));
    assert_eq!(Ok(()), writer.write_chars(&['A', '🙂']));
    assert_eq!(Ok(()), writer.write_chars(&[]));
    assert_eq!(Ok(()), writer.write_line("line"));
    assert_eq!(Ok(()), writer.flush());

    assert_eq!("中A🙂line\n", writer.output);
}
