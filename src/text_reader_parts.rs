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
/// use qubit_io_text::TextReader;
///
/// use qubit_codec_text::{CharsetDecodePolicy, CharsetDecoder, Utf8Codec};
/// use qubit_io_text::{BufferedTextReader, TextReaderParts};
///
/// let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
/// let mut reader = BufferedTextReader::with_capacity(
///     Cursor::new("abc🙂tail".as_bytes().to_vec()), decoder, 4,
/// );
/// assert_eq!(Some('a'), reader.read_char()?);
/// let parts: TextReaderParts<_, _> = reader.into_parts();
/// assert_eq!(['b', 'c'], parts.pending_chars.as_slice());
/// assert_eq!([0xf0], parts.unread_bytes.readable());
/// let mut bytes = parts.unread_bytes.readable().to_vec();
/// use std::io::Read;
/// let mut input = parts.input;
/// input.read_to_end(&mut bytes)?;
/// let mut restored: String = parts.pending_chars.into_iter().collect();
/// restored.push_str(std::str::from_utf8(&bytes)?);
/// assert_eq!("bc🙂tail", restored);
/// # Ok::<(), Box<dyn std::error::Error>>(())
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
