// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_io_text::TextReaderParts;

#[test]
fn text_reader_parts_has_named_state_fields() {
    let parts = TextReaderParts {
        input: 1_u8,
        unread_bytes: qubit_io::Buffer::with_capacity(0),
        decoder: 2_u8,
        pending_chars: vec!['x'],
    };
    assert_eq!(1, parts.input);
    assert_eq!(2, parts.decoder);
    assert_eq!(['x'], parts.pending_chars.as_slice());
}
