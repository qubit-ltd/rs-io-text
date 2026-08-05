// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
use super::AsyncTextRead;

/// Asynchronously reads text by line while preserving line terminators.
///
/// Implementations define the accepted line-ending policy. Built-in readers
/// preserve a recognized terminator in the returned text.
/// Cancellation behavior is implementation-specific; callers must consult the
/// concrete reader before retrying a cancelled operation.
#[allow(async_fn_in_trait)]
pub trait AsyncTextLineRead: AsyncTextRead {
    /// Reads one line into `output`.
    ///
    /// The line is appended to `output`; its existing contents are preserved.
    ///
    /// # Parameters
    ///
    /// - `output` - Destination string receiving the next line.
    ///
    /// # Returns
    ///
    /// Returns `true` when a line or final unterminated text was appended, or
    /// `false` at EOF when no text was appended.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific source or decoding error. A line
    /// prefix can remain in `output` when a later read fails.
    async fn read_line_async(
        &mut self,
        output: &mut String,
    ) -> Result<bool, Self::Error>;
}
