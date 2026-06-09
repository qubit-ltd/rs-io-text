// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Cursor, ErrorKind};

use qubit_codec_text::{CharsetDecodePolicy, CharsetDecoder};
use qubit_io_text::{BufferedReader, CodingErrorPolicy, TextRead, Utf8Codec};

#[test]
fn test_buffered_reader_decodes_utf8_across_single_byte_refills() -> std::io::Result<()> {
    let bytes = "Aé🙂".as_bytes().to_vec();
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader =
        BufferedReader::with_capacity(Cursor::new(bytes), decoder, CodingErrorPolicy::Strict, 1);

    let mut output = String::new();
    let count = reader.read_to_string(&mut output)?;

    assert_eq!(3, count);
    assert_eq!("Aé🙂", output);
    Ok(())
}

#[test]
fn test_buffered_reader_replaces_incomplete_eof_tail() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::replace('\u{FFFD}'));
    let mut reader = BufferedReader::with_capacity(
        Cursor::new(vec![0xE2, 0x82]),
        decoder,
        CodingErrorPolicy::Replace,
        1,
    );

    let mut output = String::new();
    let count = reader.read_to_string(&mut output)?;

    assert_eq!(1, count);
    assert_eq!("\u{FFFD}", output);
    Ok(())
}

#[test]
fn test_buffered_reader_reports_strict_incomplete_eof_tail() {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(
        Cursor::new(vec![0xE2, 0x82]),
        decoder,
        CodingErrorPolicy::Strict,
        1,
    );

    let error = reader
        .read_char()
        .expect_err("strict incomplete EOF should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}
