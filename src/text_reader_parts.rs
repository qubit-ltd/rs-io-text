// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Named components returned when a buffered text reader is consumed.

use qubit_io::Buffer;

/// The components of a buffered text reader that may contain unread state.
///
/// The input can be physically ahead of `unread_bytes`; consume the byte
/// buffer before reading from `input`. Process `pending_chars` before reading
/// more decoded text. Decomposing a reader does not finalize its decoder.
/// The returned decoder preserves its existing state and may already have
/// been finalized at EOF; decomposition does not reset or reopen it.
///
/// # Type Parameters
///
/// - `I`: The byte input returned by the reader.
/// - `D`: The decoder, including any codec state retained across input
///   boundaries.
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
///
/// use qubit_codec_text::{CharsetDecodePolicy, CharsetDecoder, Utf8Codec};
/// use qubit_io_text::{BufferedTextReader, TextReaderParts};
///
/// let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
/// let reader = BufferedTextReader::new(Cursor::new(b"hello".to_vec()), decoder);
/// let parts: TextReaderParts<_, _> = reader.into_parts();
/// assert!(parts.unread_bytes.is_empty());
/// assert!(parts.pending_chars.is_empty());
/// ```
#[must_use = "all returned reader state must be handled"]
#[derive(Debug)]
pub struct TextReaderParts<I, D> {
    /// The wrapped byte input.
    pub input: I,
    /// Encoded bytes already read from `input` but not consumed by the
    /// decoder.
    pub unread_bytes: Buffer<u8>,
    /// The decoder retaining codec state across input boundaries.
    pub decoder: D,
    /// Decoded characters already available to the caller.
    pub pending_chars: Vec<char>,
}
