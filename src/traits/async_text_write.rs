// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
use crate::LineEnding;

/// Asynchronously writes Unicode text to a text sink.
#[allow(async_fn_in_trait)]
pub trait AsyncTextWrite {
    /// Error returned by this text sink.
    type Error;

    /// Returns the configured line ending.
    fn line_ending(&self) -> LineEnding {
        LineEnding::Lf
    }

    /// Writes one Unicode scalar value.
    async fn write_char_async(&mut self, ch: char) -> Result<(), Self::Error>;

    /// Writes one step of a character slice and returns the consumed count.
    async fn write_chars_async(
        &mut self,
        chars: &[char],
    ) -> Result<usize, Self::Error>;

    /// Writes one step of a UTF-8 string and returns the consumed byte count.
    async fn write_str_async(
        &mut self,
        text: &str,
    ) -> Result<usize, Self::Error>;

    /// Writes a complete line and its configured line ending.
    async fn write_line_fully_async(
        &mut self,
        line: &str,
    ) -> Result<(), Self::Error>;

    /// Flushes pending encoded output.
    async fn flush_async(&mut self) -> Result<(), Self::Error>;

    /// Finishes the encoder and flushes pending output.
    async fn finish_async(&mut self) -> Result<(), Self::Error>;
}
