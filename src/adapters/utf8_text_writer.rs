// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io;

use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::Utf8Codec;
use qubit_io::Buffer;
use qubit_io::Output;

use crate::CharsetTextWriter;
use crate::LineEnding;
use crate::TextWrite;

/// Streaming UTF-8 text writer over a Qubit byte output.
///
/// This is the strict UTF-8 convenience form of [`CharsetTextWriter`]. It
/// shares the generic charset buffering and encoder state machine instead of
/// exposing a separate `std::io::Write`-based core API.
///
/// # Examples
///
/// ```
/// use qubit_io_text::{TextWrite, Utf8TextWriter};
///
/// let mut bytes = Vec::new();
/// let mut writer = Utf8TextWriter::new(&mut bytes);
/// writer.write_str("hello")?;
/// writer.finish()?;
/// let (output, pending) = writer.into_parts();
/// assert!(pending.readable().is_empty());
/// assert_eq!(b"hello", output.as_slice());
/// # Ok::<(), std::io::Error>(())
/// ```
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
            writer: CharsetTextWriter::new(output, Utf8Codec, CharsetEncodePolicy::report()),
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
                CharsetEncodePolicy::report(),
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

    /// Finalizes encoded output and flushes pending bytes.
    ///
    /// # Errors
    ///
    /// Returns an encoding finalization or underlying output error.
    #[inline]
    pub fn finish(&mut self) -> io::Result<()> {
        self.writer.finish()
    }

    /// Returns the wrapped byte output and every encoded byte still pending.
    ///
    /// This method does not call [`Self::finish`] or flush the wrapped output.
    /// Call [`Self::finish`] first for normal completion; otherwise the
    /// returned buffer contains encoded bytes that have not reached the
    /// returned output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output and pending encoded bytes in logical write
    /// order.
    #[must_use = "the returned output and pending buffer must be handled"]
    #[inline(always)]
    pub fn into_parts(self) -> (O, Buffer<u8>) {
        self.writer.into_parts()
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
        self.writer.write_utf8_bytes(text)
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
