// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{self, Cursor, ErrorKind, Read};

use qubit_codec_text::{CharsetDecodePolicy, Utf8Codec};
use qubit_io::Input;
use qubit_io_text::{CharsetReadExt, CharsetTextReader, TextLineRead, TextRead};

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("read failed"))
    }
}

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

    unsafe fn read_unchecked(
        &mut self,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        let available = self.bytes.len() - self.position;
        let read = available.min(count);
        let input_end = self.position + read;
        let output_end = index + read;
        output[index..output_end].copy_from_slice(&self.bytes[self.position..input_end]);
        self.position = input_end;
        Ok(read)
    }
}

#[test]
fn test_new_decodes_utf8_text() -> std::io::Result<()> {
    let bytes = "中文\nsecond".as_bytes().to_vec();
    let mut reader =
        CharsetTextReader::new(Cursor::new(bytes), Utf8Codec, CharsetDecodePolicy::report());
    let mut line = String::new();

    assert!(reader.read_line(&mut line)?);
    assert_eq!("中文\n", line);

    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("second", line);
    Ok(())
}

#[test]
fn test_new_accepts_qubit_input_without_std_read() -> std::io::Result<()> {
    let input = InputOnlyReader::new("input中文");
    let mut reader = CharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut output = String::new();

    assert_eq!(7, reader.read_to_string(&mut output)?);
    assert_eq!("input中文", output);
    Ok(())
}

#[test]
fn test_read_char_preserves_access_to_wrapped_input() -> std::io::Result<()> {
    let mut reader = CharsetTextReader::new(
        Cursor::new("中文".as_bytes().to_vec()),
        Utf8Codec,
        CharsetDecodePolicy::report(),
    );

    assert_eq!(Some('中'), reader.read_char()?);

    assert_eq!(6, reader.input().position());
    Ok(())
}

#[test]
fn test_accessors_expose_wrapped_reader() {
    let input = Cursor::new("abc".as_bytes().to_vec());
    let reader = CharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());

    assert_eq!(0, reader.input().position());
    assert_eq!(0, reader.input().position());
}

#[test]
fn test_charset_text_reader_into_parts_preserves_unreturned_characters() -> std::io::Result<()> {
    let mut reader = CharsetTextReader::new(
        Cursor::new(b"abc".to_vec()),
        Utf8Codec,
        CharsetDecodePolicy::report(),
    );

    assert_eq!(Some('a'), reader.read_char()?);

    let (input, unread, _decoder, pending_chars) = reader.into_parts();
    assert_eq!(3, input.position());
    assert!(unread.readable().is_empty());
    assert_eq!(['b', 'c'], pending_chars.as_slice());
    Ok(())
}

#[test]
fn test_read_chars_after_decoding() -> std::io::Result<()> {
    let mut reader = CharsetTextReader::new(
        Cursor::new("中文".as_bytes().to_vec()),
        Utf8Codec,
        CharsetDecodePolicy::report(),
    );
    let mut chars = Vec::new();

    assert_eq!(2, reader.read_chars(&mut chars, 8)?);
    assert_eq!(vec!['中', '文'], chars);
    Ok(())
}

#[test]
fn test_new_propagates_reader_errors() {
    let mut reader =
        CharsetTextReader::new(FailingReader, Utf8Codec, CharsetDecodePolicy::report());
    let error = reader
        .read_char()
        .expect_err("reader errors must be propagated");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_new_rejects_invalid_bytes_in_strict_mode() {
    let mut reader = CharsetTextReader::new(
        Cursor::new(vec![0xFF]),
        Utf8Codec,
        CharsetDecodePolicy::report(),
    );
    let error = reader
        .read_char()
        .expect_err("strict mode must reject invalid text");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_new_replaces_invalid_bytes_in_replace_mode() -> std::io::Result<()> {
    let mut reader = CharsetTextReader::new(
        Cursor::new(vec![0xFF]),
        Utf8Codec,
        CharsetDecodePolicy::replace(CharsetDecodePolicy::DEFAULT_REPLACEMENT),
    );
    let mut output = String::new();

    assert_eq!(1, reader.read_to_string(&mut output)?);
    assert_eq!("\u{FFFD}", output);
    Ok(())
}

#[test]
fn test_new_reports_incomplete_bytes_in_strict_mode() {
    let mut reader = CharsetTextReader::new(
        Cursor::new(vec![0xE4, 0xB8]),
        Utf8Codec,
        CharsetDecodePolicy::report(),
    );
    let error = reader
        .read_char()
        .expect_err("strict mode must reject incomplete text");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_new_replaces_incomplete_bytes_in_replace_mode() -> std::io::Result<()> {
    let mut reader = CharsetTextReader::new(
        Cursor::new(vec![0xE4, 0xB8]),
        Utf8Codec,
        CharsetDecodePolicy::replace('!'),
    );
    let mut output = String::new();

    assert_eq!(1, reader.read_to_string(&mut output)?);
    assert_eq!("!", output);
    Ok(())
}

#[test]
fn test_new_ignores_incomplete_bytes_in_ignore_mode() -> std::io::Result<()> {
    let mut reader = CharsetTextReader::new(
        Cursor::new(vec![0xE4, 0xB8]),
        Utf8Codec,
        CharsetDecodePolicy::ignore(),
    );
    let mut output = String::new();

    assert_eq!(0, reader.read_to_string(&mut output)?);
    assert!(output.is_empty());
    Ok(())
}

#[test]
fn test_with_capacity_preserves_utf8_tail_across_refills() -> std::io::Result<()> {
    let input = Cursor::new("中🙂".as_bytes().to_vec());
    let mut reader = CharsetTextReader::new_with_buffer_capacity(
        input,
        Utf8Codec,
        CharsetDecodePolicy::report(),
        1,
    );

    assert_eq!(Some('中'), reader.read_char()?);
    assert_eq!(Some('🙂'), reader.read_char()?);
    assert_eq!(None, reader.read_char()?);
    Ok(())
}

#[test]
fn test_all_small_capacities_preserve_utf8_boundaries() -> std::io::Result<()> {
    let expected = "A中🙂B";
    for capacity in 0..=expected.len() {
        let input = Cursor::new(expected.as_bytes().to_vec());
        let mut reader = CharsetTextReader::new_with_buffer_capacity(
            input,
            Utf8Codec,
            CharsetDecodePolicy::report(),
            capacity,
        );
        let mut output = String::new();

        reader.read_to_string(&mut output)?;

        assert_eq!(expected, output, "capacity {capacity}");
    }
    Ok(())
}

#[test]
fn test_charset_read_ext_creates_stream_reader() -> std::io::Result<()> {
    let input = Cursor::new("ext中文".as_bytes().to_vec());
    let mut reader = input.charset_text_reader(Utf8Codec, CharsetDecodePolicy::report());
    let mut output = String::new();

    assert_eq!(5, reader.read_to_string(&mut output)?);
    assert_eq!("ext中文", output);
    Ok(())
}

#[test]
fn test_charset_read_ext_accepts_qubit_input_without_std_read() -> std::io::Result<()> {
    let input = InputOnlyReader::new("ext输入");
    let mut reader = input.charset_text_reader(Utf8Codec, CharsetDecodePolicy::report());
    let mut output = String::new();

    assert_eq!(5, reader.read_to_string(&mut output)?);
    assert_eq!("ext输入", output);
    Ok(())
}

#[test]
fn test_charset_read_ext_creates_buffered_stream_reader() -> std::io::Result<()> {
    let input = Cursor::new("Aé🙂".as_bytes().to_vec());
    let mut reader =
        input.buffered_charset_text_reader(Utf8Codec, CharsetDecodePolicy::report(), 1);
    let mut output = String::new();

    assert_eq!(3, reader.read_to_string(&mut output)?);
    assert_eq!("Aé🙂", output);
    Ok(())
}

#[test]
fn test_charset_read_ext_reads_one_shot_from_qubit_input() -> std::io::Result<()> {
    let mut input = InputOnlyReader::new("one-shot输入");

    let output = input.read_to_string_with_charset(Utf8Codec, CharsetDecodePolicy::report())?;

    assert_eq!("one-shot输入", output);
    Ok(())
}

#[test]
fn test_charset_read_ext_reads_one_shot_text() -> std::io::Result<()> {
    let mut input = Cursor::new("one-shot".as_bytes().to_vec());

    let output = input.read_to_string_with_charset(Utf8Codec, CharsetDecodePolicy::report())?;

    assert_eq!("one-shot", output);
    Ok(())
}

#[test]
fn test_charset_reader_limited_read_rolls_back_appended_text() {
    let input = Cursor::new("A中".as_bytes().to_vec());
    let mut reader = CharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut output = String::from("prefix:");

    let error = reader
        .read_to_string_limited(&mut output, 3)
        .expect_err("decoded text beyond the append limit must fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("prefix:", output);
}

#[test]
fn test_charset_read_ext_reads_one_shot_limited_text() -> std::io::Result<()> {
    let mut input = Cursor::new("中".as_bytes().to_vec());

    let output =
        input.read_to_string_with_charset_limited(Utf8Codec, CharsetDecodePolicy::report(), 3)?;

    assert_eq!("中", output);
    Ok(())
}
