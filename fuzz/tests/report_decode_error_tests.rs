// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::io::Cursor;

use qubit_codec::ByteOrder;
use qubit_codec::TranscodeDecodeError;
use qubit_codec::TranscodeFailure;
use qubit_codec_text::Charset;
use qubit_codec_text::CharsetCodec;
use qubit_codec_text::CharsetDecodeError;
use qubit_codec_text::CharsetDecodeErrorKind;
use qubit_codec_text::CharsetDecodePolicy;
use qubit_codec_text::Utf8Codec;
use qubit_codec_text::Utf16ByteCodec;
use qubit_codec_text::Utf32ByteCodec;
use qubit_io_text::CharsetTextReader;
use qubit_io_text::TextReader;
use qubit_io_text_fuzz::report_decode_error::is_report_input_error;

fn domain_error(charset: Charset, kind: CharsetDecodeErrorKind) -> io::Error {
    let source = CharsetDecodeError::new(charset, kind, 0);
    let error = TranscodeDecodeError::domain_main(source, 0);
    io::Error::new(io::ErrorKind::InvalidData, error)
}

fn framework_error(failure: TranscodeFailure) -> io::Error {
    let source = TranscodeDecodeError::<CharsetDecodeError>::Failure(failure);
    io::Error::new(io::ErrorKind::InvalidData, source)
}

#[test]
fn test_report_input_error_rejects_internal_failures() {
    for failure in [
        TranscodeFailure::LifecyclePoisoned,
        TranscodeFailure::InvalidInputIndex { index: 1, input_len: 0 },
        TranscodeFailure::InsufficientOutput {
            output_index: 0,
            required: 1,
            available: 0,
        },
    ] {
        assert!(!is_report_input_error(&framework_error(failure), Charset::UTF_8));
    }
}

#[test]
fn test_report_input_error_accepts_encoding_errors() {
    for kind in [
        CharsetDecodeErrorKind::malformed(0xff),
        CharsetDecodeErrorKind::IncompleteSequence {
            required: 3,
            available: 2,
        },
        CharsetDecodeErrorKind::InvalidCodePoint { value: 0x11_0000 },
    ] {
        let error = domain_error(Charset::UTF_8, kind);
        assert!(is_report_input_error(&error, Charset::UTF_8));
    }
}

#[test]
fn test_report_input_error_accepts_framework_incomplete_input() {
    let error = framework_error(TranscodeFailure::IncompleteInput {
        input_index: 0,
        required: 3,
        available: 2,
    });
    assert!(is_report_input_error(&error, Charset::UTF_8));
}

#[test]
fn test_report_input_error_rejects_unknown_source_and_wrong_charset() {
    let unknown = io::Error::new(io::ErrorKind::InvalidData, "unexpected decoder error");
    assert!(!is_report_input_error(&unknown, Charset::UTF_8));
    let error = domain_error(Charset::UTF_16LE, CharsetDecodeErrorKind::malformed(0xdc00));
    assert!(!is_report_input_error(&error, Charset::UTF_8));
}

#[test]
fn test_report_input_error_rejects_other_io_kinds_and_unwrapped_domain() {
    let source = CharsetDecodeError::new(Charset::UTF_8, CharsetDecodeErrorKind::malformed(0xff), 0);
    let wrapped = TranscodeDecodeError::domain_main(source, 0);
    let other_kind = io::Error::new(io::ErrorKind::Other, wrapped);
    assert!(!is_report_input_error(&other_kind, Charset::UTF_8));
    let unwrapped = io::Error::new(io::ErrorKind::InvalidData, source);
    assert!(!is_report_input_error(&unwrapped, Charset::UTF_8));
}

fn check_reader_errors<C: CharsetCodec<Unit = u8> + Copy>(codec: C, inputs: &[&[u8]]) {
    for input in inputs {
        for capacity in 1..=8 {
            let mut reader = CharsetTextReader::new_with_buffer_capacity(
                Cursor::new(input.to_vec()),
                codec,
                CharsetDecodePolicy::report(),
                capacity,
            );
            let error = reader.read_to_string(&mut String::new()).expect_err("invalid encoding");
            assert!(is_report_input_error(&error, codec.charset()), "{error:?}");
        }
    }
}

#[test]
fn test_report_input_error_accepts_real_reader_errors() {
    check_reader_errors(Utf8Codec, &[&[0xff], &[0xe4, 0xb8]]);
    check_reader_errors(
        Utf16ByteCodec::new(ByteOrder::LittleEndian),
        &[&[0x00, 0xdc], &[0x00, 0xd8], &[0x41]],
    );
    check_reader_errors(
        Utf16ByteCodec::new(ByteOrder::BigEndian),
        &[&[0xdc, 0x00], &[0xd8, 0x00], &[0x41]],
    );
    check_reader_errors(
        Utf32ByteCodec::new(ByteOrder::LittleEndian),
        &[&[0x00, 0x00, 0x11, 0x00], &[0x41, 0x00, 0x00]],
    );
    check_reader_errors(
        Utf32ByteCodec::new(ByteOrder::BigEndian),
        &[&[0x00, 0x11, 0x00, 0x00], &[0x00, 0x00, 0x41]],
    );
}
