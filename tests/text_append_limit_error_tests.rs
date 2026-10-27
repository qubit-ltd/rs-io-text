// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::io::Cursor;

use qubit_codec_text::CharsetDecodePolicy;
use qubit_codec_text::Utf8Codec;
use qubit_io_text::CharsetReadExt;
use qubit_io_text::TextAppendLimitError;
use qubit_io_text::TextLineReader;
use qubit_io_text::TextReader;
use qubit_io_text::Utf8TextReader;

#[test]
fn test_limited_text_exposes_utf8_byte_limit() {
    let mut reader = Utf8TextReader::new(Cursor::new("中🙂".as_bytes()));
    let mut output = String::from("seed:");
    let error = reader.read_to_string_limited(&mut output, 3).unwrap_err();
    let detail = error.get_ref().unwrap().downcast_ref::<TextAppendLimitError>().unwrap();
    assert_eq!(3, detail.max_append_len());
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert_eq!("decoded text exceeds the 3-byte append limit", error.to_string());
    assert_eq!("seed:", output);
}

#[test]
fn test_limited_line_reports_limit_then_reads_next_record() -> io::Result<()> {
    let mut reader = Utf8TextReader::new(Cursor::new(b"abcdef\nok\n"));
    let mut output = String::new();
    let error = reader.read_line_limited(&mut output, 4).unwrap_err();
    assert_eq!(
        4,
        error
            .get_ref()
            .unwrap()
            .downcast_ref::<TextAppendLimitError>()
            .unwrap()
            .max_append_len()
    );
    assert!(output.is_empty());
    assert!(reader.read_line(&mut output)?);
    assert_eq!("ok\n", output);
    Ok(())
}

#[test]
fn test_one_shot_limit_has_structured_source() {
    let mut input = Cursor::new(b"abc");
    let error = input
        .read_to_string_with_charset_limited(Utf8Codec, CharsetDecodePolicy::report(), 1)
        .unwrap_err();
    assert_eq!(
        1,
        error
            .get_ref()
            .unwrap()
            .downcast_ref::<TextAppendLimitError>()
            .unwrap()
            .max_append_len()
    );
}

#[test]
fn test_invalid_encoding_and_unrelated_invalid_data_are_not_limit_errors() {
    let mut reader = Utf8TextReader::new(Cursor::new([0xff_u8]));
    let error = reader.read_char().unwrap_err();
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert!(
        error
            .get_ref()
            .unwrap()
            .downcast_ref::<TextAppendLimitError>()
            .is_none()
    );
    let unrelated = io::Error::new(io::ErrorKind::InvalidData, "external invalid data");
    assert!(
        unrelated
            .get_ref()
            .unwrap()
            .downcast_ref::<TextAppendLimitError>()
            .is_none()
    );
}
