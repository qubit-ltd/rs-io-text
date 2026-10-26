// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::ops::Deref;
use std::ops::DerefMut;

use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::Utf8Codec;
use qubit_io::AsyncByteOutput;

use crate::AsyncCharsetTextWriter;
use crate::AsyncTextWriter;
use crate::LineEnding;

/// Asynchronous UTF-8 writer over a Qubit byte output.
///
/// This convenience wrapper fixes the codec to UTF-8 while preserving the
/// buffering, cancellation, finalization, and state-recovery behavior
/// of [`AsyncCharsetTextWriter`]. Methods of the wrapped writer are available
/// through [`Deref`] and [`DerefMut`]. After encoder finalization succeeds, all
/// text-writing methods reject input, including empty strings and character
/// slices; retry [`AsyncCharsetTextWriter::finish_async`] to resume delivery
/// after an output failure.
/// The same recovery limitation applies after a caught inner I/O panic:
/// pending bytes may overlap data already delivered. See
/// [`AsyncCharsetTextWriter::into_buffered_parts`] before retrying or
/// replaying.
///
/// # Type Parameters
///
/// - `O`: Asynchronous byte output that receives UTF-8 encoded data.
///
/// # Examples
///
/// ```no_run
/// use qubit_io::AsyncByteOutput;
/// use qubit_io_text::AsyncUtf8TextWriter;
///
/// async fn write_text<O>(output: O) -> std::io::Result<()>
/// where
///     O: AsyncByteOutput + Unpin,
/// {
///     let mut writer = AsyncUtf8TextWriter::new(output);
///     writer.write_str_fully_async("hello").await?;
///     writer.finish_async().await
/// }
/// ```
#[derive(Debug)]
pub struct AsyncUtf8TextWriter<O>(AsyncCharsetTextWriter<O, Utf8Codec>)
where
    O: AsyncByteOutput;

impl<O> AsyncUtf8TextWriter<O>
where
    O: AsyncByteOutput,
{
    /// Creates a strict UTF-8 writer with the default buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `output`: Asynchronous byte output that receives encoded data.
    ///
    /// # Returns
    ///
    /// Returns a writer that reports encoding errors and uses LF line endings.
    #[inline(always)]
    #[must_use]
    pub fn new(output: O) -> Self {
        Self(AsyncCharsetTextWriter::new(
            output,
            Utf8Codec,
            CharsetEncodePolicy::report(),
        ))
    }

    /// Creates a UTF-8 writer with an explicit byte capacity.
    ///
    /// # Parameters
    ///
    /// - `output`: Asynchronous byte output that receives encoded data.
    /// - `capacity`: Requested internal encoded-byte capacity. The wrapped
    ///   writer raises values that cannot hold one encoded character.
    ///
    /// # Returns
    ///
    /// Returns a writer configured with the effective capacity and LF line
    /// endings.
    #[inline(always)]
    #[must_use]
    pub fn with_capacity(output: O, capacity: usize) -> Self {
        Self(AsyncCharsetTextWriter::new_with_buffer_capacity(
            output,
            Utf8Codec,
            CharsetEncodePolicy::report(),
            capacity,
        ))
    }

    /// Sets the line ending used by future asynchronous line writes.
    ///
    /// # Parameters
    ///
    /// - `line_ending`: Line ending to append to future lines.
    ///
    /// # Returns
    ///
    /// This writer with the requested line-ending configuration.
    #[inline(always)]
    #[must_use]
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.0 = self.0.with_line_ending(line_ending);
        self
    }

    /// Consumes this wrapper and returns the generic charset writer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped writer with its output, encoder, and buffered state
    /// unchanged. This method performs no flush or finalization.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> AsyncCharsetTextWriter<O, Utf8Codec> {
        self.0
    }
}

impl<O> Deref for AsyncUtf8TextWriter<O>
where
    O: AsyncByteOutput,
{
    type Target = AsyncCharsetTextWriter<O, Utf8Codec>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<O> DerefMut for AsyncUtf8TextWriter<O>
where
    O: AsyncByteOutput,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<O> AsyncTextWriter for AsyncUtf8TextWriter<O>
where
    O: AsyncByteOutput + Unpin,
{
    type Error = std::io::Error;

    fn line_ending(&self) -> LineEnding {
        self.0.configured_line_ending()
    }

    async fn write_char_async(&mut self, ch: char) -> Result<(), Self::Error> {
        self.0.write_char_async(ch).await
    }

    async fn write_chars_async(&mut self, chars: &[char]) -> Result<usize, Self::Error> {
        self.0.write_chars_async(chars).await
    }

    async fn write_str_async(&mut self, text: &str) -> Result<usize, Self::Error> {
        self.0.write_str_async(text).await
    }

    async fn write_chars_fully_async(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        self.0.write_chars_fully_async(chars).await
    }

    async fn write_str_fully_async(&mut self, text: &str) -> Result<(), Self::Error> {
        self.0.write_str_fully_async(text).await
    }

    async fn write_line_fully_async(&mut self, line: &str) -> Result<(), Self::Error> {
        self.0.write_line_fully_async(line).await
    }

    async fn flush_async(&mut self) -> Result<(), Self::Error> {
        self.0.flush_async().await
    }

    async fn finish_async(&mut self) -> Result<(), Self::Error> {
        self.0.finish_async().await
    }
}
