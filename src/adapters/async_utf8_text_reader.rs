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
use qubit_io::AsyncInput;

use crate::AsyncCharsetTextReader;
use crate::AsyncTextLineRead;
use crate::AsyncTextRead;
use crate::LineEndingSet;

/// Asynchronous UTF-8 reader over a Qubit byte input.
///
/// This convenience wrapper fixes the codec to UTF-8 while preserving the
/// policy, buffering, cancellation, and state-recovery behavior of
/// [`AsyncCharsetTextReader`]. Methods of the wrapped reader are available
/// through [`Deref`] and [`DerefMut`].
///
/// # Type Parameters
///
/// - `I`: Asynchronous byte input that supplies UTF-8 encoded data.
#[derive(Debug)]
pub struct AsyncUtf8TextReader<I>(AsyncCharsetTextReader<I, Utf8Codec>)
where
    I: AsyncInput<Item = u8>;

impl<I> AsyncUtf8TextReader<I>
where
    I: AsyncInput<Item = u8>,
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
    #[must_use]
    pub fn with_line_endings(mut self, line_endings: LineEndingSet) -> Self {
        self.0 = self.0.with_line_endings(line_endings);
        self
    }

    /// Returns the line endings recognized by this reader.
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
    I: AsyncInput<Item = u8>,
{
    type Target = AsyncCharsetTextReader<I, Utf8Codec>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<I> DerefMut for AsyncUtf8TextReader<I>
where
    I: AsyncInput<Item = u8>,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<I> AsyncTextRead for AsyncUtf8TextReader<I>
where
    I: AsyncInput<Item = u8> + Unpin,
{
    type Error = std::io::Error;

    async fn read_char_async(&mut self) -> Result<Option<char>, Self::Error> {
        self.0.read_char_async().await
    }

    async fn read_chars_async(&mut self, output: &mut Vec<char>, max: usize) -> Result<usize, Self::Error> {
        self.0.read_chars_async(output, max).await
    }

    async fn read_to_string_async(&mut self, output: &mut String) -> Result<usize, Self::Error> {
        self.0.read_to_string_async(output).await
    }
}

impl<I> AsyncTextLineRead for AsyncUtf8TextReader<I>
where
    I: AsyncInput<Item = u8> + Unpin,
{
    async fn read_line_async(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        self.0.read_line_async(output).await
    }
}
