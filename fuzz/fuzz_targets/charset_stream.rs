// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![no_main]

use std::io::Cursor;

use libfuzzer_sys::fuzz_target;
use qubit_codec::ByteOrder;
use qubit_codec_text::CharsetDecodePolicy;
use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::Utf8Codec;
use qubit_codec_text::Utf16ByteCodec;
use qubit_codec_text::Utf32ByteCodec;
use qubit_io_text::CharsetTextReader;
use qubit_io_text::CharsetTextWriter;
use qubit_io_text::TextRead;
use qubit_io_text::TextWrite;

/// Bounds allocations when the target is invoked outside CI.
const MAX_FUZZ_INPUT_LEN: usize = 4096;

fuzz_target!(|data: &[u8]| {
    let data = &data[..data.len().min(MAX_FUZZ_INPUT_LEN)];
    fuzz_strict_utf8(data);
    fuzz_replacing_utf8(data);
    fuzz_utf8_round_trip(data);
    fuzz_bounded_utf8_reads(data);
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
        assert!(pending.readable().is_empty());

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

/// Exercises append limits and line-oriented state transitions on arbitrary
/// input without allowing the target to allocate unbounded output.
fn fuzz_bounded_utf8_reads(data: &[u8]) {
    for capacity in 1..=4 {
        let mut reader = CharsetTextReader::new_with_buffer_capacity(
            Cursor::new(data.to_vec()),
            Utf8Codec,
            CharsetDecodePolicy::replace(CharsetDecodePolicy::DEFAULT_REPLACEMENT),
            capacity,
        );
        let mut output = String::new();
        let result = reader.read_to_string_limited(&mut output, 128);
        if result.is_ok() {
            assert!(output.len() <= 128);
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
            assert!(pending.readable().is_empty());

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
        assert!(pending.readable().is_empty());

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
