// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![no_main]

use std::io::Cursor;
use std::io::ErrorKind;

use libfuzzer_sys::fuzz_target;
use qubit_codec::ByteOrder;
use qubit_codec_text::CharsetDecodePolicy;
use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::Utf8Codec;
use qubit_codec_text::Utf16ByteCodec;
use qubit_codec_text::Utf32ByteCodec;
use qubit_io_text::CharsetTextReader;
use qubit_io_text::CharsetTextWriter;
use qubit_io_text::TextReader;
use qubit_io_text::TextWriter;

/// Bounds allocations when the target is invoked outside CI.
const MAX_FUZZ_INPUT_LEN: usize = 4096;

fuzz_target!(|data: &[u8]| {
    let data = &data[..data.len().min(MAX_FUZZ_INPUT_LEN)];
    fuzz_strict_utf8(data);
    fuzz_replacing_utf8(data);
    fuzz_utf8_round_trip(data);
    fuzz_bounded_utf8_reads(data);
    fuzz_mixed_utf8_reads(data);
    fuzz_multibyte_round_trips(data);
});

/// Verifies strict decoding agrees with the standard UTF-8 validator for all
/// internal refill capacities.
fn fuzz_strict_utf8(data: &[u8]) {
    for capacity in 1..=8 {
        let mut reader = CharsetTextReader::new_with_buffer_capacity(
            Cursor::new(data.to_vec()),
            Utf8Codec,
            CharsetDecodePolicy::report(),
            capacity,
        );
        let mut output = String::new();
        let result = reader.read_to_string(&mut output);

        match std::str::from_utf8(data) {
            Ok(expected) => {
                result.expect("valid UTF-8 must decode in strict mode");
                assert_eq!(expected, output);
            }
            Err(_) => assert!(result.is_err()),
        }
    }
}

/// Verifies replacement decoding completes for arbitrary byte streams and all
/// internal refill capacities.
fn fuzz_replacing_utf8(data: &[u8]) {
    for capacity in 1..=8 {
        let mut reader = CharsetTextReader::new_with_buffer_capacity(
            Cursor::new(data.to_vec()),
            Utf8Codec,
            CharsetDecodePolicy::replace(CharsetDecodePolicy::DEFAULT_REPLACEMENT),
            capacity,
        );
        let mut output = String::new();
        reader
            .read_to_string(&mut output)
            .expect("replacement decoding must complete for in-memory input");
        assert_eq!(String::from_utf8_lossy(data), output);
    }
}

/// Verifies valid UTF-8 survives streaming encoding and decoding at every
/// small internal buffer capacity.
fn fuzz_utf8_round_trip(data: &[u8]) {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    for capacity in 1..=8 {
        let mut writer = CharsetTextWriter::new_with_buffer_capacity(
            Cursor::new(Vec::new()),
            Utf8Codec,
            CharsetEncodePolicy::report(),
            capacity,
        );
        writer.write_str(text).expect("valid UTF-8 must encode in strict mode");
        writer.finish().expect("in-memory UTF-8 writer must finish");
        let (output, pending) = writer.into_parts();
        assert!(matches!(
            pending,
            qubit_io::BufferedOutputRecovery::Pending(buffer) if buffer.readable().is_empty()
        ));

        let mut reader = CharsetTextReader::new_with_buffer_capacity(
            Cursor::new(output.into_inner()),
            Utf8Codec,
            CharsetDecodePolicy::report(),
            capacity,
        );
        let mut decoded = String::new();
        reader
            .read_to_string(&mut decoded)
            .expect("encoded valid UTF-8 must decode in strict mode");
        assert_eq!(text, decoded);
    }
}

/// Checks bounded bulk reads against the standard UTF-8 replacement decoder.
fn fuzz_bounded_utf8_reads(data: &[u8]) {
    let expected = String::from_utf8_lossy(data);
    let variable_limit = usize::from(data.first().copied().unwrap_or(0)) % 129;
    for capacity in 1..=4 {
        for limit in [0, variable_limit, 128, expected.len().saturating_sub(1), expected.len()] {
            let mut reader = CharsetTextReader::new_with_buffer_capacity(
                Cursor::new(data.to_vec()),
                Utf8Codec,
                CharsetDecodePolicy::replace(CharsetDecodePolicy::DEFAULT_REPLACEMENT),
                capacity,
            );
            let mut output = String::from("seed:中");
            let result = reader.read_to_string_limited(&mut output, limit);
            if expected.len() <= limit {
                assert_eq!(expected.chars().count(), result.expect("bounded text must fit"));
                assert_eq!(format!("seed:中{expected}"), output);
            } else {
                assert_eq!(
                    ErrorKind::InvalidData,
                    result.expect_err("text must exceed limit").kind()
                );
                assert_eq!("seed:中", output);
            }
        }
    }
}

/// Returns the byte length of the next line under the default ALL policy.
/// This model uses string boundaries rather than the production state machine.
fn expected_line_len(text: &str) -> usize {
    let Some(index) = text.find(['\r', '\n']) else {
        return text.len();
    };
    let ending = if text[index..].starts_with("\r\n") { 2 } else { 1 };
    index + ending
}

/// Checks mixed line and character operations against a logical text cursor.
fn fuzz_mixed_utf8_reads(data: &[u8]) {
    let expected = String::from_utf8_lossy(data);
    let variable_limit = usize::from(data.first().copied().unwrap_or(0)) % 17;
    for capacity in 1..=4 {
        for limit in [0, variable_limit, 128, expected.len()] {
            let mut reader = CharsetTextReader::new_with_buffer_capacity(
                Cursor::new(data.to_vec()),
                Utf8Codec,
                CharsetDecodePolicy::replace(CharsetDecodePolicy::DEFAULT_REPLACEMENT),
                capacity,
            );
            let mut position = 0;
            let operations = [2_u8, 0, 1].into_iter().chain(data.iter().copied());
            for selector in operations.take(expected.chars().count().saturating_add(2)) {
                let suffix = &expected[position..];
                match selector % 3 {
                    0 => {
                        let next = suffix.chars().next();
                        assert_eq!(next, reader.read_char().expect("replacement read succeeds"));
                        if let Some(ch) = next {
                            position += ch.len_utf8();
                        }
                    }
                    1 => {
                        let max = usize::from(selector) % 4 + 1;
                        let wanted: Vec<char> = suffix.chars().take(max).collect();
                        let mut output = vec!['中'];
                        let count = reader
                            .read_chars(&mut output, max)
                            .expect("replacement bulk read succeeds");
                        assert_eq!(wanted.len(), count);
                        assert_eq!('中', output[0]);
                        assert_eq!(wanted.as_slice(), &output[1..]);
                        position += wanted.iter().map(|ch| ch.len_utf8()).sum::<usize>();
                    }
                    _ => {
                        let length = expected_line_len(suffix);
                        let line = &suffix[..length];
                        let mut output = String::from("seed:中");
                        let result = reader.read_line_limited(&mut output, limit);
                        if length <= limit {
                            assert_eq!(!line.is_empty(), result.expect("line must fit"));
                            assert_eq!(format!("seed:中{line}"), output);
                        } else {
                            assert_eq!(
                                ErrorKind::InvalidData,
                                result.expect_err("line must exceed limit").kind()
                            );
                            assert_eq!("seed:中", output);
                        }
                        // Oversized records are discarded through their ending.
                        position += length;
                    }
                }
            }
            let mut tail = String::new();
            let count = reader
                .read_to_string(&mut tail)
                .expect("replacement tail read succeeds");
            assert_eq!(&expected[position..], tail);
            assert_eq!(expected[position..].chars().count(), count);
        }
    }
}

/// Verifies that the fixed-width codecs preserve valid Unicode text through
/// small streaming buffers.
fn fuzz_multibyte_round_trips(data: &[u8]) {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    for capacity in 1..=4 {
        for codec in [
            Utf16ByteCodec::new(ByteOrder::LittleEndian),
            Utf16ByteCodec::new(ByteOrder::BigEndian),
        ] {
            let mut writer = CharsetTextWriter::new_with_buffer_capacity(
                Cursor::new(Vec::new()),
                codec,
                CharsetEncodePolicy::report(),
                capacity,
            );
            writer.write_str(text).expect("UTF-16 encoding must succeed");
            writer.finish().expect("UTF-16 writer must finish");
            let (output, pending) = writer.into_parts();
            assert!(matches!(
                pending,
                qubit_io::BufferedOutputRecovery::Pending(buffer) if buffer.readable().is_empty()
            ));

            let mut reader = CharsetTextReader::new_with_buffer_capacity(
                Cursor::new(output.into_inner()),
                codec,
                CharsetDecodePolicy::report(),
                capacity,
            );
            let mut decoded = String::new();
            reader.read_to_string(&mut decoded).expect("UTF-16 decode");
            assert_eq!(text, decoded);
        }

        let codec = Utf32ByteCodec::new(ByteOrder::LittleEndian);
        let mut writer = CharsetTextWriter::new_with_buffer_capacity(
            Cursor::new(Vec::new()),
            codec,
            CharsetEncodePolicy::report(),
            capacity,
        );
        writer.write_str(text).expect("UTF-32 encoding must succeed");
        writer.finish().expect("UTF-32 writer must finish");
        let (output, pending) = writer.into_parts();
        assert!(matches!(
            pending,
            qubit_io::BufferedOutputRecovery::Pending(buffer) if buffer.readable().is_empty()
        ));

        let mut reader = CharsetTextReader::new_with_buffer_capacity(
            Cursor::new(output.into_inner()),
            codec,
            CharsetDecodePolicy::report(),
            capacity,
        );
        let mut decoded = String::new();
        reader.read_to_string(&mut decoded).expect("UTF-32 decode");
        assert_eq!(text, decoded);
    }
}
