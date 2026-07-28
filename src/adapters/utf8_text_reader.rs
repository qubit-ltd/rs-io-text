// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io;

use qubit_codec_text::Utf8Codec;
use qubit_io::Input;

use crate::{CharsetTextReader, CodingErrorPolicy, TextLineRead, TextRead};

/// Streaming UTF-8 text reader over a Qubit byte input.
///
/// This is the strict UTF-8 convenience form of [`CharsetTextReader`]. It
/// shares the same buffering and decoder state machine instead of introducing
/// a separate `std::io::Read`-based stream boundary.
#[derive(Debug)]
pub struct Utf8TextReader<I>
where
    I: Input<Item = u8>,
{
    reader: CharsetTextReader<I, Utf8Codec>,
}

impl<I> Utf8TextReader<I>
where
    I: Input<Item = u8>,
{
    /// Creates a strict UTF-8 text reader with the default buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `input`: Qubit byte input to decode lazily.
    ///
    /// # Returns
    ///
    /// Returns a streaming reader. Construction does not read from `input`.
    #[inline]
    #[must_use]
    pub fn new(input: I) -> Self {
        Self {
            reader: CharsetTextReader::new(input, Utf8Codec, CodingErrorPolicy::Strict),
        }
    }

    /// Creates a strict UTF-8 reader with a requested byte buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `input`: Qubit byte input to decode lazily.
    /// - `buffer_capacity`: Requested internal byte buffer capacity.
    ///
    /// # Returns
    ///
    /// Returns a streaming reader. The generic buffered layer raises a
    /// too-small capacity enough to retain an incomplete UTF-8 scalar.
    #[inline]
    #[must_use]
    pub fn with_capacity(input: I, buffer_capacity: usize) -> Self {
        Self {
            reader: CharsetTextReader::new_with_buffer_capacity(
                input,
                Utf8Codec,
                CodingErrorPolicy::Strict,
                buffer_capacity,
            ),
        }
    }

    /// Returns a shared reference to the wrapped byte input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input. It may already be positioned beyond bytes
    /// retained in the reader's internal buffer.
    #[inline(always)]
    #[must_use]
    pub const fn input(&self) -> &I {
        self.reader.input()
    }

    /// Returns a mutable reference to the wrapped byte input.
    ///
    /// Mutating the input directly can invalidate the logical position
    /// represented by buffered bytes.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input.
    #[inline(always)]
    pub fn input_mut(&mut self) -> &mut I {
        self.reader.input_mut()
    }
}

impl<I> TextRead for Utf8TextReader<I>
where
    I: Input<Item = u8>,
{
    type Error = io::Error;

    #[inline]
    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        self.reader.read_char()
    }

    #[inline]
    fn read_chars(&mut self, output: &mut Vec<char>, max: usize) -> Result<usize, Self::Error> {
        self.reader.read_chars(output, max)
    }

    #[inline]
    fn read_to_string(&mut self, output: &mut String) -> Result<usize, Self::Error> {
        self.reader.read_to_string(output)
    }
}

impl<I> TextLineRead for Utf8TextReader<I>
where
    I: Input<Item = u8>,
{
    #[inline]
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        self.reader.read_line(output)
    }
}
