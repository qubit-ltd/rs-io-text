// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io;

use qubit_codec_text::{
    CharsetCodec,
    CharsetDecoder,
};
use qubit_io::{
    Buffer,
    Input,
};

use crate::{
    BufferedReader,
    CodingErrorPolicy,
    TextLineRead,
    TextRead,
};

/// Text reader that decodes a byte stream with a charset codec.
///
/// This adapter is a charset-specific wrapper around [`BufferedReader`]. It
/// constructs the appropriate [`CharsetDecoder`] from the supplied codec and
/// malformed-input policy.
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
///
/// use qubit_codec_text::Utf8Codec;
/// use qubit_io_text::{CharsetTextReader, CodingErrorPolicy, TextRead};
///
/// let mut reader = CharsetTextReader::new(
///     Cursor::new("中文".as_bytes().to_vec()),
///     Utf8Codec,
///     CodingErrorPolicy::Strict,
/// );
/// let mut text = String::new();
/// reader.read_to_string(&mut text)?;
/// assert_eq!("中文", text);
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct CharsetTextReader<I, C>
where
    I: Input<Item = u8>,
    C: CharsetCodec<Unit = u8>,
{
    reader: BufferedReader<I, CharsetDecoder<C>>,
}

impl<I, C> CharsetTextReader<I, C>
where
    I: Input<Item = u8>,
    C: CharsetCodec<Unit = u8>,
{
    /// Creates a charset text reader with the default buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `input`: Byte reader to decode lazily.
    /// - `codec`: Byte-oriented charset codec used by the input.
    /// - `policy`: Malformed input handling policy.
    ///
    /// # Returns
    ///
    /// Returns a streaming text reader. Construction does not read from
    /// `input`; I/O and decode errors are reported by read methods.
    #[must_use]
    #[inline]
    pub fn new(input: I, codec: C, policy: CodingErrorPolicy) -> Self {
        let decoder =
            CharsetDecoder::with_policy(codec, policy.decode_policy());
        Self {
            reader: BufferedReader::new(input, decoder, policy),
        }
    }

    /// Creates a charset text reader with a requested byte buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `input`: Byte reader to decode lazily.
    /// - `codec`: Byte-oriented charset codec used by the input.
    /// - `policy`: Malformed input handling policy.
    /// - `buffer_capacity`: Requested internal byte buffer capacity.
    ///
    /// # Returns
    ///
    /// Returns a streaming text reader. The generic buffered text layer raises
    /// too-small capacities enough to retain built-in charset tails.
    #[must_use]
    #[inline]
    pub fn new_with_buffer_capacity(
        input: I,
        codec: C,
        policy: CodingErrorPolicy,
        buffer_capacity: usize,
    ) -> Self {
        let decoder =
            CharsetDecoder::with_policy(codec, policy.decode_policy());
        Self {
            reader: BufferedReader::with_capacity(
                input,
                decoder,
                policy,
                buffer_capacity,
            ),
        }
    }

    /// Returns a shared reference to the wrapped byte reader.
    ///
    /// # Returns
    ///
    /// Returns the wrapped byte reader. It may already be positioned beyond
    /// bytes retained in this reader's internal buffer.
    #[must_use]
    #[inline(always)]
    pub const fn input(&self) -> &I {
        self.reader.inner()
    }

    /// Consumes this reader and returns every component that may contain
    /// unread logical input.
    ///
    /// The returned characters must be processed before continuing the text
    /// stream. The returned byte buffer must be consumed before reading from
    /// the wrapped input because the wrapped input can be physically ahead of
    /// that buffer. This method does not finish the decoder.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input, unread encoded bytes, decoder, and decoded
    /// characters not yet returned by this reader, in that order.
    #[must_use = "all returned reader state must be handled"]
    #[inline]
    pub fn into_parts(self) -> (I, Buffer<u8>, CharsetDecoder<C>, Vec<char>) {
        self.reader.into_parts()
    }

    /// Appends decoded text while enforcing a UTF-8 byte limit.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination string to append to.
    /// - `max_append_len`: Maximum UTF-8 byte length appended by this call.
    ///
    /// # Returns
    ///
    /// Returns the number of Unicode scalar values appended to `output`.
    ///
    /// # Errors
    ///
    /// Returns input or decoding errors. Returns [`io::ErrorKind::InvalidData`]
    /// and restores `output` to its original length when the decoded text
    /// exceeds `max_append_len`. Previously consumed characters remain
    /// consumed by the reader.
    pub fn read_to_string_limited(
        &mut self,
        output: &mut String,
        max_append_len: usize,
    ) -> io::Result<usize> {
        self.reader.read_to_string_limited(output, max_append_len)
    }
}

impl<I, C> TextRead for CharsetTextReader<I, C>
where
    I: Input<Item = u8>,
    C: CharsetCodec<Unit = u8>,
{
    type Error = io::Error;

    #[inline]
    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        self.reader.read_char()
    }

    #[inline]
    fn read_chars(
        &mut self,
        output: &mut Vec<char>,
        max: usize,
    ) -> Result<usize, Self::Error> {
        self.reader.read_chars(output, max)
    }

    #[inline]
    fn read_to_string(
        &mut self,
        output: &mut String,
    ) -> Result<usize, Self::Error> {
        self.reader.read_to_string(output)
    }
}

impl<I, C> TextLineRead for CharsetTextReader<I, C>
where
    I: Input<Item = u8>,
    C: CharsetCodec<Unit = u8>,
{
    #[inline]
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        self.reader.read_line(output)
    }
}
