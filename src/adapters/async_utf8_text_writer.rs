// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::ops::{
    Deref,
    DerefMut,
};

use qubit_codec_text::{
    CharsetEncodePolicy,
    Utf8Codec,
};
use qubit_io::AsyncOutput;

use crate::AsyncCharsetTextWriter;

/// Asynchronous UTF-8 writer over a Qubit byte output.
///
/// This convenience wrapper fixes the codec to UTF-8 while preserving the
/// policy, buffering, cancellation, finalization, and state-recovery behavior
/// of [`AsyncCharsetTextWriter`]. Methods of the wrapped writer are available
/// through [`Deref`] and [`DerefMut`].
///
/// # Type Parameters
///
/// - `O`: Asynchronous byte output that receives UTF-8 encoded data.
#[derive(Debug)]
pub struct AsyncUtf8TextWriter<O>(AsyncCharsetTextWriter<O, Utf8Codec>)
where
    O: AsyncOutput<Item = u8>;

impl<O> AsyncUtf8TextWriter<O>
where
    O: AsyncOutput<Item = u8>,
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
        Self::with_policy(output, CharsetEncodePolicy::report())
    }

    /// Creates a UTF-8 writer with an explicit error policy.
    ///
    /// # Parameters
    ///
    /// - `output`: Asynchronous byte output that receives encoded data.
    /// - `policy`: Policy for characters that cannot be encoded.
    ///
    /// # Returns
    ///
    /// Returns a writer using the default buffer capacity, `policy`, and LF
    /// line endings.
    #[inline(always)]
    #[must_use]
    pub fn with_policy(output: O, policy: CharsetEncodePolicy) -> Self {
        Self(AsyncCharsetTextWriter::new(output, Utf8Codec, policy))
    }

    /// Creates a UTF-8 writer with an explicit byte capacity.
    ///
    /// # Parameters
    ///
    /// - `output`: Asynchronous byte output that receives encoded data.
    /// - `policy`: Policy for characters that cannot be encoded.
    /// - `capacity`: Requested internal encoded-byte capacity. The wrapped
    ///   writer raises values that cannot hold one encoded character.
    ///
    /// # Returns
    ///
    /// Returns a writer configured with `policy`, the effective capacity, and
    /// LF line endings.
    #[inline(always)]
    #[must_use]
    pub fn with_capacity(
        output: O,
        policy: CharsetEncodePolicy,
        capacity: usize,
    ) -> Self {
        Self(AsyncCharsetTextWriter::new_with_buffer_capacity(
            output, Utf8Codec, policy, capacity,
        ))
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
    O: AsyncOutput<Item = u8>,
{
    type Target = AsyncCharsetTextWriter<O, Utf8Codec>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<O> DerefMut for AsyncUtf8TextWriter<O>
where
    O: AsyncOutput<Item = u8>,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
