// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{
    self,
    Read,
};

use qubit_codec_text::{
    CharsetCodec,
    CharsetDecoder,
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
#[derive(Debug)]
pub struct CharsetTextReader<R, C>
where
    R: Read,
    C: CharsetCodec<Unit = u8>,
{
    reader: BufferedReader<R, CharsetDecoder<C>>,
}

impl<R, C> CharsetTextReader<R, C>
where
    R: Read,
    C: CharsetCodec<Unit = u8>,
{
    /// Creates a charset text reader with the default buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Byte reader to decode lazily.
    /// - `codec`: Byte-oriented charset codec used by the input.
    /// - `policy`: Malformed input handling policy.
    ///
    /// # Returns
    ///
    /// Returns a streaming text reader. Construction does not read from
    /// `inner`; I/O and decode errors are reported by read methods.
    #[must_use]
    pub fn new(inner: R, codec: C, policy: CodingErrorPolicy) -> Self {
        let decoder =
            CharsetDecoder::with_policy(codec, policy.decode_policy());
        Self {
            reader: BufferedReader::new(inner, decoder, policy),
        }
    }

    /// Creates a charset text reader with a requested byte buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Byte reader to decode lazily.
    /// - `codec`: Byte-oriented charset codec used by the input.
    /// - `policy`: Malformed input handling policy.
    /// - `capacity`: Requested internal byte buffer capacity.
    ///
    /// # Returns
    ///
    /// Returns a streaming text reader. The generic buffered text layer raises
    /// too-small capacities enough to retain built-in charset tails.
    #[must_use]
    pub fn with_capacity(
        inner: R,
        codec: C,
        policy: CodingErrorPolicy,
        capacity: usize,
    ) -> Self {
        let decoder =
            CharsetDecoder::with_policy(codec, policy.decode_policy());
        Self {
            reader: BufferedReader::with_capacity(
                inner, decoder, policy, capacity,
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
    pub const fn get_ref(&self) -> &R {
        self.reader.inner()
    }

    /// Returns a mutable reference to the wrapped byte reader.
    ///
    /// # Returns
    ///
    /// Returns the wrapped byte reader. Mutating it directly can invalidate the
    /// logical stream position represented by buffered bytes.
    pub fn get_mut(&mut self) -> &mut R {
        self.reader.inner_mut()
    }

    /// Returns a shared reference to the wrapped byte reader.
    ///
    /// # Returns
    ///
    /// Returns the wrapped byte reader.
    #[must_use]
    pub const fn inner(&self) -> &R {
        self.reader.inner()
    }

    /// Returns a mutable reference to the wrapped byte reader.
    ///
    /// # Returns
    ///
    /// Returns the wrapped byte reader.
    pub fn inner_mut(&mut self) -> &mut R {
        self.reader.inner_mut()
    }

    /// Consumes this reader and returns the wrapped byte reader.
    ///
    /// # Returns
    ///
    /// Returns the underlying reader. Any encoded bytes or decoded characters
    /// already buffered by this reader are discarded.
    #[must_use]
    pub fn into_inner(self) -> R {
        self.reader.into_inner()
    }
}

impl<R, C> TextRead for CharsetTextReader<R, C>
where
    R: Read,
    C: CharsetCodec<Unit = u8>,
{
    type Error = io::Error;

    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        self.reader.read_char()
    }

    fn read_chars(
        &mut self,
        output: &mut Vec<char>,
        max: usize,
    ) -> Result<usize, Self::Error> {
        self.reader.read_chars(output, max)
    }

    fn read_to_string(
        &mut self,
        output: &mut String,
    ) -> Result<usize, Self::Error> {
        self.reader.read_to_string(output)
    }
}

impl<R, C> TextLineRead for CharsetTextReader<R, C>
where
    R: Read,
    C: CharsetCodec<Unit = u8>,
{
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        self.reader.read_line(output)
    }
}

/// Buffered alias preserved for API compatibility with older naming patterns.
pub type BufferedCharsetTextReader<R, C> = CharsetTextReader<R, C>;
