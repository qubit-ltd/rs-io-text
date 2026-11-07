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
/// of [`AsyncCharsetTextWriter`]. Methods of the wrapped writer that borrow it
/// are available through [`Deref`] and [`DerefMut`]. To consume the writer and
/// recover its parts, call `into_inner().into_parts()` or
/// `into_inner().into_buffered_parts()`. After encoder finalization succeeds,
/// all text-writing methods reject input, including empty strings and character
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
pub struct AsyncUtf8TextWriter<O>(
    /// Holds the charset writer that owns the output, UTF-8 encoder, and
    /// pending bytes.
    AsyncCharsetTextWriter<O, Utf8Codec>,
)
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
    /// The generic writer type exposed by dereferencing this UTF-8 wrapper.
    type Target = AsyncCharsetTextWriter<O, Utf8Codec>;

    /// Borrows the wrapped charset writer to expose its configuration and
    /// state.
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<O> DerefMut for AsyncUtf8TextWriter<O>
where
    O: AsyncByteOutput,
{
    /// Mutably borrows the wrapped writer so delegated operations remain
    /// available.
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<O> AsyncTextWriter for AsyncUtf8TextWriter<O>
where
    O: AsyncByteOutput + Unpin,
{
    /// Error type used by asynchronous writes; encoding and output failures use
    /// I/O errors.
    type Error = std::io::Error;

    /// Returns the line ending configured for subsequent line writes.
    ///
    /// # Returns
    ///
    /// Returns the current line-ending setting.
    fn line_ending(&self) -> LineEnding {
        self.0.configured_line_ending()
    }

    /// Encodes and writes one character, returning after the wrapped writer
    /// accepts its progress.
    ///
    /// # Parameters
    ///
    /// - `ch`: Character to encode.
    ///
    /// # Errors
    ///
    /// Returns an encoding or output error.
    ///
    /// # Returns
    ///
    /// Returns success after the character is accepted.
    async fn write_char_async(&mut self, ch: char) -> Result<(), Self::Error> {
        self.0.write_char_async(ch).await
    }

    /// Encodes a character prefix and returns the number of characters
    /// accepted.
    ///
    /// # Parameters
    ///
    /// - `chars`: Characters to encode.
    ///
    /// # Returns
    ///
    /// Returns the accepted character count, which can be smaller than the
    /// input length.
    ///
    /// # Errors
    ///
    /// Returns an encoding or output error; a prefix may already have been
    /// accepted.
    ///
    /// # Returns
    ///
    /// Returns success after progress is accepted; inspect the count to resume
    /// a partial write.
    async fn write_chars_async(&mut self, chars: &[char]) -> Result<usize, Self::Error> {
        self.0.write_chars_async(chars).await
    }

    /// Encodes a UTF-8 byte prefix of `text` and returns its committed byte
    /// length.
    ///
    /// # Parameters
    ///
    /// - `text`: UTF-8 text to encode.
    ///
    /// # Returns
    ///
    /// Returns the accepted prefix length in UTF-8 bytes.
    ///
    /// # Errors
    ///
    /// Returns an encoding or output error; a prefix may already have been
    /// accepted.
    ///
    /// # Returns
    ///
    /// Returns success after progress is accepted; use the byte count to resume
    /// at a UTF-8 boundary.
    async fn write_str_async(&mut self, text: &str) -> Result<usize, Self::Error> {
        self.0.write_str_async(text).await
    }

    /// Repeats character writes until all input is accepted. Cancellation may
    /// lose the cursor.
    ///
    /// # Parameters
    ///
    /// - `chars`: Characters to encode and write completely.
    ///
    /// # Errors
    ///
    /// Returns an encoding or output error; earlier characters may already have
    /// been accepted.
    ///
    /// # Returns
    ///
    /// Returns success after all characters are accepted.
    async fn write_chars_fully_async(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        self.0.write_chars_fully_async(chars).await
    }

    /// Repeats string writes until all input is accepted. Cancellation may lose
    /// the cursor.
    ///
    /// # Parameters
    ///
    /// - `text`: UTF-8 text to encode and write completely.
    ///
    /// # Errors
    ///
    /// Returns an encoding or output error; earlier characters may already have
    /// been accepted.
    ///
    /// # Returns
    ///
    /// Returns success after all text is accepted.
    async fn write_str_fully_async(&mut self, text: &str) -> Result<(), Self::Error> {
        self.0.write_str_fully_async(text).await
    }

    /// Writes the line and configured ending as separate operations.
    /// Cancellation may interrupt either operation.
    ///
    /// # Parameters
    ///
    /// - `line`: Text to write before the configured line ending.
    ///
    /// # Errors
    ///
    /// Returns an encoding or output error; the line or its ending may be
    /// partially accepted.
    ///
    /// # Returns
    ///
    /// Returns success after both the line and its ending are accepted.
    async fn write_line_fully_async(&mut self, line: &str) -> Result<(), Self::Error> {
        self.0.write_line_fully_async(line).await
    }

    /// Drains pending encoded bytes and flushes the wrapped output without
    /// finalizing the encoder.
    ///
    /// # Errors
    ///
    /// Returns an output write or flush error.
    ///
    /// # Returns
    ///
    /// Returns success after pending bytes are drained and the wrapped output
    /// is flushed.
    async fn flush_async(&mut self) -> Result<(), Self::Error> {
        self.0.flush_async().await
    }

    /// Finalizes UTF-8 encoding and flushes output. Failed delivery can be
    /// retried on this writer.
    ///
    /// # Errors
    ///
    /// Returns an encoding finalization or output error. After finalization
    /// succeeds, text writes are rejected even if output delivery still
    /// needs retrying.
    ///
    /// # Returns
    ///
    /// Returns success after finalization and output flushing complete.
    async fn finish_async(&mut self) -> Result<(), Self::Error> {
        self.0.finish_async().await
    }
}
