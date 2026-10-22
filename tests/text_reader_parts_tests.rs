// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::Buffer;
use qubit_io_text::TextReaderParts;

#[test]
fn test_text_reader_parts_exposes_named_state_fields() {
    let parts = TextReaderParts {
        input: 1_u8,
        unread_bytes: Buffer::with_capacity(0),
        decoder: 2_u8,
        pending_chars: vec!['x'],
    };
    assert_eq!(1, parts.input, "the original input should be returned");
    assert_eq!(2, parts.decoder, "the decoder state should be returned");
    assert_eq!(
        ['x'],
        parts.pending_chars.as_slice(),
        "pending decoded characters should be returned",
    );
}
