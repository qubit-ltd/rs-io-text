/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use crate::LineEnding;

/// Writes Unicode text to a text sink.
///
/// `TextWrite` accepts Rust Unicode text and delegates byte encoding, storage,
/// logging, or other sink-specific behavior to the concrete implementation.
pub trait TextWrite {
    /// Error returned by this text sink.
    type Error;

    /// Returns the configured line ending for [`TextWrite::write_line`].
    ///
    /// # Returns
    /// The configured line ending. Implementations default to [`LineEnding::Lf`].
    #[must_use]
    fn line_ending(&self) -> LineEnding {
        LineEnding::Lf
    }

    /// Writes one Unicode scalar value.
    ///
    /// # Parameters
    /// - `ch`: Character to write.
    ///
    /// # Errors
    /// Returns an implementation-specific error when the character cannot be
    /// accepted by the sink.
    fn write_char(&mut self, ch: char) -> Result<(), Self::Error>;

    /// Writes a slice of Unicode scalar values.
    ///
    /// # Parameters
    /// - `chars`: Characters to write in order.
    ///
    /// # Errors
    /// Returns an implementation-specific error when any character cannot be
    /// accepted by the sink.
    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error>;

    /// Writes a string slice.
    ///
    /// # Parameters
    /// - `text`: Text to write.
    ///
    /// # Errors
    /// Returns an implementation-specific error when the text cannot be accepted
    /// by the sink.
    fn write_str(&mut self, text: &str) -> Result<(), Self::Error>;

    /// Writes a line followed by the configured line ending.
    ///
    /// # Parameters
    /// - `line`: Line content to write. The configured line ending is appended.
    ///
    /// # Errors
    /// Returns an implementation-specific error when the line or line ending
    /// cannot be accepted by the sink.
    fn write_line(&mut self, line: &str) -> Result<(), Self::Error>;

    /// Flushes buffered text to the underlying sink.
    ///
    /// # Errors
    /// Returns an implementation-specific error when flushing fails.
    fn flush(&mut self) -> Result<(), Self::Error>;
}
