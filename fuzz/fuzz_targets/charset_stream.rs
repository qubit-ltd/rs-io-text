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
use qubit_codec_text::Utf8Codec;
use qubit_io_text::{
    CharsetTextReader,
    CodingErrorPolicy,
    TextRead,
};

/// Bounds allocations when the target is invoked outside CI.
const MAX_FUZZ_INPUT_LEN: usize = 4096;

fuzz_target!(|data: &[u8]| {
    let data = &data[..data.len().min(MAX_FUZZ_INPUT_LEN)];
    fuzz_strict_utf8(data);
    fuzz_replacing_utf8(data);
});

/// Verifies strict decoding agrees with the standard UTF-8 validator for all
/// internal refill capacities.
fn fuzz_strict_utf8(data: &[u8]) {
    for capacity in 1..=8 {
        let mut reader = CharsetTextReader::new_with_buffer_capacity(
            Cursor::new(data.to_vec()),
            Utf8Codec,
            CodingErrorPolicy::Strict,
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
            CodingErrorPolicy::Replace,
            capacity,
        );
        let mut output = String::new();
        reader
            .read_to_string(&mut output)
            .expect("replacement decoding must complete for in-memory input");
    }
}
