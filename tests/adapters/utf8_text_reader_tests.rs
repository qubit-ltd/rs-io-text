// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io;
use std::io::Cursor;
use std::io::ErrorKind;
use std::io::Read;

use qubit_io::Input;
use qubit_io_text::LineEnding;
use qubit_io_text::LineEndingSet;
use qubit_io_text::TextLineRead;
use qubit_io_text::TextRead;
use qubit_io_text::Utf8TextReader;

struct InputOnlyReader {
    bytes: Vec<u8>,
    position: usize,
}

impl InputOnlyReader {
    fn new(text: &str) -> Self {
        Self {
            bytes: text.as_bytes().to_vec(),
            position: 0,
        }
    }
}

impl Input for InputOnlyReader {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, output: &mut [u8], index: usize, count: usize) -> io::Result<usize> {
        let available = self.bytes.len() - self.position;
        let read = available.min(count);
        let input_end = self.position + read;
        let output_end = index + read;
        output[index..output_end].copy_from_slice(&self.bytes[self.position..input_end]);
        self.position = input_end;
        Ok(read)
    }
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("read failed"))
    }
}

struct InterruptedOnceReader {
    data: Cursor<Vec<u8>>,
    interrupted: bool,
}

impl InterruptedOnceReader {
    fn new(data: impl Into<Vec<u8>>) -> Self {
        Self {
            data: Cursor::new(data.into()),
            interrupted: false,
        }
    }
}

impl Read for InterruptedOnceReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(io::Error::from(ErrorKind::Interrupted));
        }
        Read::read(&mut self.data, buf)
    }
}

#[test]
fn test_new_accepts_qubit_input_without_std_read() -> std::io::Result<()> {
    let input = InputOnlyReader::new("input中文");
    let mut reader = Utf8TextReader::new(input);
    let mut output = String::new();

    assert_eq!(7, reader.read_to_string(&mut output)?);
    assert_eq!("input中文", output);
    Ok(())
}

#[test]
fn test_read_char_and_line_from_utf8_reader() -> std::io::Result<()> {
    let input = Cursor::new("a中\nβeta".as_bytes().to_vec());
    let mut reader = Utf8TextReader::new(input);
    let mut line = String::new();

    assert_eq!(Some('a'), reader.read_char()?);
    assert!(reader.read_line(&mut line)?);
    assert_eq!("中\n", line);

    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("βeta", line);
    assert!(!reader.read_line(&mut line)?);
    Ok(())
}

#[test]
fn test_utf8_reader_configures_line_endings() -> std::io::Result<()> {
    let mut reader = Utf8TextReader::new(Cursor::new(b"first\rsecond\nthird".to_vec()))
        .with_line_endings(LineEndingSet::only(LineEnding::Cr));
    assert_eq!(LineEndingSet::CR, reader.line_endings());

    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("first\r", line);
    Ok(())
}

#[test]
fn test_utf8_reader_read_line_limited_preserves_existing_output() -> std::io::Result<()> {
    let mut reader = Utf8TextReader::new(Cursor::new("a中\nnext".as_bytes().to_vec()));
    let mut output = String::from("prefix-");

    let error = reader
        .read_line_limited(&mut output, 4)
        .expect_err("oversized line should fail");
    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("prefix-", output);
    Ok(())
}

#[test]
fn test_new_accessors_expose_wrapped_input() {
    let input = Cursor::new("abc".as_bytes().to_vec());
    let reader = Utf8TextReader::with_capacity(input, 1);

    assert_eq!(3, reader.input().get_ref().len());
    assert_eq!(0, reader.input().position());
}

#[test]
fn test_utf8_text_reader_into_parts_preserves_unreturned_characters() -> std::io::Result<()> {
    let mut reader = Utf8TextReader::new(Cursor::new(b"abc".to_vec()));

    assert_eq!(Some('a'), reader.read_char()?);

    let parts = reader.into_parts();
    assert_eq!(3, parts.input.position());
    assert!(parts.unread_bytes.readable().is_empty());
    assert_eq!(['b', 'c'], parts.pending_chars.as_slice());
    Ok(())
}

#[test]
fn test_read_char_covers_utf8_widths_and_eof() -> std::io::Result<()> {
    let input = Cursor::new("aé中🙂".as_bytes().to_vec());
    let mut reader = Utf8TextReader::new(input);

    assert_eq!(Some('a'), reader.read_char()?);
    assert_eq!(Some('é'), reader.read_char()?);
    assert_eq!(Some('中'), reader.read_char()?);
    assert_eq!(Some('🙂'), reader.read_char()?);
    assert_eq!(None, reader.read_char()?);
    Ok(())
}

#[test]
fn test_read_chars_reads_utf8_scalars() -> std::io::Result<()> {
    let input = Cursor::new("a中🙂".as_bytes().to_vec());
    let mut reader = Utf8TextReader::new(input);
    let mut chars = Vec::new();

    assert_eq!(2, reader.read_chars(&mut chars, 2)?);
    assert_eq!(vec!['a', '中'], chars);
    assert_eq!(1, reader.read_chars(&mut chars, 4)?);
    assert_eq!(vec!['a', '中', '🙂'], chars);
    Ok(())
}

#[test]
fn test_read_chars_propagates_utf8_errors() {
    let input = Cursor::new(vec![0xFF]);
    let mut reader = Utf8TextReader::new(input);
    let mut chars = Vec::new();

    let error = reader
        .read_chars(&mut chars, 1)
        .expect_err("invalid UTF-8 must be rejected while reading chars");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_to_string_appends_valid_utf8() -> std::io::Result<()> {
    let input = Cursor::new("中🙂".as_bytes().to_vec());
    let mut reader = Utf8TextReader::new(input);
    let mut output = String::from("prefix:");

    assert_eq!(2, reader.read_to_string(&mut output)?);
    assert_eq!("prefix:中🙂", output);
    Ok(())
}

#[test]
fn test_read_to_string_reports_invalid_utf8() {
    let input = Cursor::new(vec![0xFF]);
    let mut reader = Utf8TextReader::new(input);
    let mut output = String::new();

    let error = reader
        .read_to_string(&mut output)
        .expect_err("invalid UTF-8 must be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_line_reports_invalid_utf8() {
    let input = Cursor::new(vec![0xFF]);
    let mut reader = Utf8TextReader::new(input);
    let mut line = String::new();

    let error = reader
        .read_line(&mut line)
        .expect_err("invalid UTF-8 line must be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_char_retries_interrupted_first_byte() -> std::io::Result<()> {
    let input = InterruptedOnceReader::new("é".as_bytes().to_vec());
    let mut reader = Utf8TextReader::new(input);

    assert_eq!(Some('é'), reader.read_char()?);
    assert_eq!(None, reader.read_char()?);
    Ok(())
}

#[test]
fn test_read_char_propagates_io_errors() {
    let mut reader = Utf8TextReader::new(FailingReader);

    let error = reader.read_char().expect_err("reader I/O errors must be propagated");
    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_read_char_reports_invalid_utf8() {
    let input = Cursor::new(vec![0xE4, 0xFF, 0xAD]);
    let mut reader = Utf8TextReader::new(input);

    let error = reader.read_char().expect_err("invalid UTF-8 scalar must be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_char_reports_invalid_leading_byte() {
    let input = Cursor::new(vec![0xFF]);
    let mut reader = Utf8TextReader::new(input);

    let error = reader
        .read_char()
        .expect_err("invalid UTF-8 leading byte must be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_char_reports_truncated_utf8_sequence() {
    let input = Cursor::new(vec![0xE4, 0xB8]);
    let mut reader = Utf8TextReader::new(input);

    let error = reader.read_char().expect_err("truncated UTF-8 scalar must be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_char_reports_truncated_two_byte_sequence() {
    let input = Cursor::new(vec![0xC2]);
    let mut reader = Utf8TextReader::new(input);

    let error = reader
        .read_char()
        .expect_err("truncated two-byte UTF-8 scalar must be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_char_reports_truncated_four_byte_sequence() {
    let input = Cursor::new(vec![0xF0, 0x9F]);
    let mut reader = Utf8TextReader::new(input);

    let error = reader
        .read_char()
        .expect_err("truncated four-byte UTF-8 scalar must be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_utf8_reader_limited_read_rolls_back_appended_text() {
    let input = Cursor::new("A中".as_bytes().to_vec());
    let mut reader = Utf8TextReader::new(input);
    let mut output = String::from("prefix:");

    let error = reader
        .read_to_string_limited(&mut output, 3)
        .expect_err("decoded text beyond the append limit must fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("prefix:", output);
}
