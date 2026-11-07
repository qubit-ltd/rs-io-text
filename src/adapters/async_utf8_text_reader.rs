// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::ops::Deref;
use std::ops::DerefMut;

use qubit_codec_text::CharsetDecodePolicy;
use qubit_codec_text::Utf8Codec;
use qubit_io::AsyncByteInput;

use crate::AsyncCharsetTextReader;
use crate::AsyncTextLineReader;
use crate::AsyncTextReader;
use crate::LineEndingSet;

/// Asynchronous UTF-8 reader over a Qubit byte input.
///
/// This convenience wrapper fixes the codec to UTF-8 while preserving the
/// policy, buffering, cancellation, and state-recovery behavior of
/// [`AsyncCharsetTextReader`]. Methods of the wrapped reader that borrow it are
/// available through [`Deref`] and [`DerefMut`]. To consume the reader and
/// recover its parts, call `into_inner().into_parts()`.
///
/// # Type Parameters
///
/// - `I`: Asynchronous byte input that supplies UTF-8 encoded data.
///
/// # Examples
///
/// ```no_run
/// use qubit_io::AsyncByteInput;
/// use qubit_io_text::AsyncUtf8TextReader;
///
/// async fn read_all<I>(input: I) -> std::io::Result<String>
/// where
///     I: AsyncByteInput + Unpin,
/// {
///     let mut reader = AsyncUtf8TextReader::new(input);
///     let mut text = String::new();
///     reader.read_to_string_async(&mut text).await?;
///     Ok(text)
/// }
/// ```
#[derive(Debug)]
pub struct AsyncUtf8TextReader<I>(
    /// Stores the generic reader that owns the input, UTF-8 decoder, and any
    /// buffered decode state.
    AsyncCharsetTextReader<I, Utf8Codec>,
)
where
    I: AsyncByteInput;

impl<I> AsyncUtf8TextReader<I>
where
    I: AsyncByteInput,
{
    /// Creates a strict UTF-8 reader with the default buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `input`: Asynchronous byte input to decode lazily.
    ///
    /// # Returns
    ///
    /// Returns a reader that reports malformed or incomplete UTF-8 input.
    #[inline(always)]
    #[must_use]
    pub fn new(input: I) -> Self {
        Self::with_policy(input, CharsetDecodePolicy::report())
    }

    /// Creates a UTF-8 reader with an explicit error policy.
    ///
    /// # Parameters
    ///
    /// - `input`: Asynchronous byte input to decode lazily.
    /// - `policy`: Policy for malformed and incomplete UTF-8 input.
    ///
    /// # Returns
    ///
    /// Returns a reader using the default buffer capacity and `policy`.
    #[inline(always)]
    #[must_use]
    pub fn with_policy(input: I, policy: CharsetDecodePolicy) -> Self {
        Self(AsyncCharsetTextReader::new(input, Utf8Codec, policy))
    }

    /// Creates a UTF-8 reader with an explicit byte capacity.
    ///
    /// # Parameters
    ///
    /// - `input`: Asynchronous byte input to decode lazily.
    /// - `policy`: Policy for malformed and incomplete UTF-8 input.
    /// - `capacity`: Requested internal encoded-byte capacity. The wrapped
    ///   reader raises values that are too small to preserve a UTF-8 tail.
    ///
    /// # Returns
    ///
    /// Returns a reader configured with `policy` and the effective capacity.
    #[inline(always)]
    #[must_use]
    pub fn with_capacity(input: I, policy: CharsetDecodePolicy, capacity: usize) -> Self {
        Self(AsyncCharsetTextReader::new_with_buffer_capacity(
            input, Utf8Codec, policy, capacity,
        ))
    }

    /// Sets the line endings recognized by asynchronous line reads.
    ///
    /// # Parameters
    /// - `line_endings`: Accepted line-ending sequences.
    ///
    /// # Returns
    /// This reader with the requested line-ending configuration.
    #[inline]
    #[must_use]
    pub fn with_line_endings(mut self, line_endings: LineEndingSet) -> Self {
        self.0 = self.0.with_line_endings(line_endings);
        self
    }

    /// Returns the line endings recognized by this reader.
    #[inline]
    #[must_use]
    pub const fn line_endings(&self) -> LineEndingSet {
        self.0.line_endings()
    }

    /// Consumes this wrapper and returns the generic charset reader.
    ///
    /// # Returns
    ///
    /// Returns the wrapped reader with its input, decoder, and buffered state
    /// unchanged.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> AsyncCharsetTextReader<I, Utf8Codec> {
        self.0
    }
}

impl<I> Deref for AsyncUtf8TextReader<I>
where
    I: AsyncByteInput,
{
    /// The generic UTF-8 reader exposed by this wrapper's dereference methods.
    type Target = AsyncCharsetTextReader<I, Utf8Codec>;

    /// Borrows the wrapped reader so callers can use its charset-reader API.
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<I> DerefMut for AsyncUtf8TextReader<I>
where
    I: AsyncByteInput,
{
    /// Mutably borrows the wrapped reader, including its input and decode
    /// state.
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<I> AsyncTextReader for AsyncUtf8TextReader<I>
where
    I: AsyncByteInput + Unpin,
{
    /// Error type returned when asynchronous text reads fail.
    type Error = std::io::Error;

    /// Reads the next decoded Unicode scalar value.
    ///
    /// # Returns
    ///
    /// Returns `Some(character)` when a character is available, or `None` after
    /// end of input has been reached.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when fetching or decoding input fails.
    async fn read_char_async(&mut self) -> Result<Option<char>, Self::Error> {
        self.0.read_char_async().await
    }

    /// Appends up to `max` decoded characters to `output`.
    ///
    /// # Parameters
    ///
    /// - `output`: Buffer that receives the decoded characters; existing
    ///   contents are preserved.
    /// - `max`: Maximum number of characters to append.
    ///
    /// # Returns
    ///
    /// Returns the number of characters appended, which may be zero at EOF.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when fetching or decoding input fails.
    async fn read_chars_async(&mut self, output: &mut Vec<char>, max: usize) -> Result<usize, Self::Error> {
        self.0.read_chars_async(output, max).await
    }

    /// Appends decoded text to `output` until the input reaches EOF.
    ///
    /// # Parameters
    ///
    /// - `output`: String that receives decoded text; existing contents are
    ///   preserved.
    ///
    /// # Returns
    ///
    /// Returns the number of characters appended.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when fetching or decoding input fails.
    async fn read_to_string_async(&mut self, output: &mut String) -> Result<usize, Self::Error> {
        self.0.read_to_string_async(output).await
    }
}

impl<I> AsyncTextLineReader for AsyncUtf8TextReader<I>
where
    I: AsyncByteInput + Unpin,
{
    /// Reads one configured line and appends its contents to `output`.
    ///
    /// # Parameters
    ///
    /// - `output`: String that receives the line contents; existing contents
    ///   are preserved.
    ///
    /// # Returns
    ///
    /// Returns `true` when a line was read and `false` at EOF before another
    /// line was available.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when fetching or decoding input fails.
    async fn read_line_async(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        self.0.read_line_async(output).await
    }
}
