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
use qubit_io::Input;

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

    /// Returns a mutable reference to the wrapped byte reader.
    ///
    /// # Returns
    ///
    /// Returns the wrapped byte reader.
    /// Returns the wrapped byte reader. Mutating it directly can invalidate the
    /// logical stream position represented by buffered bytes.
    #[inline(always)]
    pub fn input_mut(&mut self) -> &mut I {
        self.reader.inner_mut()
    }

    /// Consumes this reader and returns the wrapped byte reader.
    ///
    /// # Returns
    ///
    /// Returns the underlying reader. Any encoded bytes or decoded characters
    /// already buffered by this reader are discarded.
    #[must_use]
    #[inline]
    pub fn into_input(self) -> I {
        self.reader.into_inner()
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

/// Buffered alias preserved for API compatibility with older naming patterns.
pub type BufferedCharsetTextReader<I, C> = CharsetTextReader<I, C>;
