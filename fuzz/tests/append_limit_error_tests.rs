// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::io::Cursor;

use qubit_codec::TranscodeDecodeError;
use qubit_codec::TranscodeFailure;
use qubit_codec_text::Charset;
use qubit_codec_text::CharsetDecodeError;
use qubit_codec_text::CharsetDecodeErrorKind;
use qubit_io_text::Utf8TextReader;
use qubit_io_text_fuzz::append_limit_error::is_append_limit_error;

#[test]
fn test_append_limit_error_accepts_real_reader_error_and_exact_limit() {
    let mut reader = Utf8TextReader::new(Cursor::new(b"ab"));
    let mut output = String::new();
    let error = reader
        .read_to_string_limited(&mut output, 1)
        .expect_err("two bytes exceed a one-byte append limit");

    assert!(is_append_limit_error(&error, 1));
    assert!(!is_append_limit_error(&error, 0));
    assert!(!is_append_limit_error(&error, 2));
}

#[test]
fn test_append_limit_error_rejects_unrelated_errors() {
    let ordinary = io::Error::new(io::ErrorKind::InvalidData, "other");
    assert!(!is_append_limit_error(&ordinary, 1));

    let framework = TranscodeDecodeError::<CharsetDecodeError>::Failure(TranscodeFailure::LifecyclePoisoned);
    let framework = io::Error::new(io::ErrorKind::InvalidData, framework);
    assert!(!is_append_limit_error(&framework, 1));

    let malformed = CharsetDecodeError::new(Charset::UTF_8, CharsetDecodeErrorKind::malformed(0xff), 0);
    let malformed = TranscodeDecodeError::domain_main(malformed, 0);
    let malformed = io::Error::new(io::ErrorKind::InvalidData, malformed);
    assert!(!is_append_limit_error(&malformed, 1));

    let other_kind = io::Error::new(io::ErrorKind::Other, "other");
    assert!(!is_append_limit_error(&other_kind, 1));
}
