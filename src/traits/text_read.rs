// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
/// Reads Unicode scalar values and strings from a text source.
///
/// `TextRead` operates on Rust [`char`] and [`str`] values. Implementations are
/// responsible for decoding any external byte encoding before exposing text to
/// callers.
pub trait TextRead {
    /// Error returned by this text source.
    type Error;

    /// Reads the next Unicode scalar value.
    ///
    /// # Returns
    /// `Ok(Some(ch))` when a scalar value was read, or `Ok(None)` at EOF.
    ///
    /// # Errors
    /// Returns an implementation-specific error when the source cannot be read
    /// or cannot be decoded as text according to its policy.
    fn read_char(&mut self) -> Result<Option<char>, Self::Error>;

    /// Reads up to `max` Unicode scalar values into `output`.
    ///
    /// # Parameters
    /// - `output`: Destination vector. Read characters are appended.
    /// - `max`: Maximum number of characters to read.
    ///
    /// # Returns
    /// The number of characters appended to `output`.
    ///
    /// # Errors
    /// Returns an implementation-specific error when reading a character fails.
    fn read_chars(
        &mut self,
        output: &mut Vec<char>,
        max: usize,
    ) -> Result<usize, Self::Error>;

    /// Reads all remaining text into `output`.
    ///
    /// # Parameters
    /// - `output`: Destination string. Text is appended to the existing
    ///   content.
    ///
    /// # Returns
    /// The number of Unicode scalar values appended to `output`.
    ///
    /// # Errors
    /// Returns an implementation-specific error when the source cannot be read
    /// or decoded.
    fn read_to_string(
        &mut self,
        output: &mut String,
    ) -> Result<usize, Self::Error>;
}
