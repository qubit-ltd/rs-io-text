// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io;

use qubit_codec_text::Utf8Codec;
use qubit_io::Output;

use crate::{
    CharsetTextWriter,
    CodingErrorPolicy,
    LineEnding,
    TextWrite,
};

/// Streaming UTF-8 text writer over a Qubit byte output.
///
/// This is the strict UTF-8 convenience form of [`CharsetTextWriter`]. It
/// shares the generic charset buffering and encoder state machine instead of
/// exposing a separate `std::io::Write`-based core API.
#[derive(Debug)]
pub struct Utf8TextWriter<O>
where
    O: Output<Item = u8>,
{
    writer: CharsetTextWriter<O, Utf8Codec>,
}

impl<O> Utf8TextWriter<O>
where
    O: Output<Item = u8>,
{
    /// Creates a strict UTF-8 text writer with the default buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `output`: Qubit byte output that receives encoded bytes.
    ///
    /// # Returns
    ///
    /// Returns a text writer using LF line endings.
    #[inline]
    #[must_use]
    pub fn new(output: O) -> Self {
        Self {
            writer: CharsetTextWriter::new(
                output,
                Utf8Codec,
                CodingErrorPolicy::Strict,
            ),
        }
    }

    /// Creates a strict UTF-8 writer with a requested byte buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `output`: Qubit byte output that receives encoded bytes.
    /// - `buffer_capacity`: Requested internal byte buffer capacity.
    ///
    /// # Returns
    ///
    /// Returns a text writer using LF line endings.
    #[inline]
    #[must_use]
    pub fn with_capacity(output: O, buffer_capacity: usize) -> Self {
        Self {
            writer: CharsetTextWriter::new_with_buffer_capacity(
                output,
                Utf8Codec,
                CodingErrorPolicy::Strict,
                buffer_capacity,
            ),
        }
    }

    /// Sets the line ending used for subsequent lines.
    ///
    /// # Parameters
    ///
    /// - `line_ending`: Line ending to append from [`TextWrite::write_line`].
    ///
    /// # Returns
    ///
    /// Returns this writer with the configured line ending.
    #[inline]
    #[must_use]
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.writer = self.writer.with_line_ending(line_ending);
        self
    }

    /// Returns a shared reference to the wrapped byte output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output. Encoded bytes may still be buffered.
    #[inline(always)]
    #[must_use]
    pub const fn output(&self) -> &O {
        self.writer.output()
    }

    /// Returns a mutable reference to the wrapped byte output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output. Direct mutation can invalidate the logical
    /// ordering represented by pending encoded bytes.
    #[inline(always)]
    pub fn output_mut(&mut self) -> &mut O {
        self.writer.output_mut()
    }

    /// Finalizes encoded output and flushes pending bytes.
    ///
    /// # Errors
    ///
    /// Returns an encoding finalization or underlying output error.
    #[inline]
    pub fn finish(&mut self) -> io::Result<()> {
        self.writer.finish()
    }

    /// Finalizes output and returns the wrapped byte output.
    ///
    /// # Returns
    ///
    /// Returns the underlying output after pending bytes reach it.
    ///
    /// # Errors
    ///
    /// Returns an encoding finalization or underlying output error.
    #[inline]
    pub fn into_output(self) -> io::Result<O> {
        self.writer.into_output()
    }
}

impl<O> TextWrite for Utf8TextWriter<O>
where
    O: Output<Item = u8>,
{
    type Error = io::Error;

    #[inline]
    fn line_ending(&self) -> LineEnding {
        self.writer.line_ending()
    }

    #[inline]
    fn write_char(&mut self, ch: char) -> Result<(), Self::Error> {
        self.writer.write_char(ch)
    }

    #[inline]
    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        self.writer.write_chars(chars)
    }

    #[inline]
    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        self.writer.write_str(text)
    }

    #[inline]
    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.writer.write_line(line)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.writer.flush()
    }
}
