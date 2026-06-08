// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use crate::TextRead;

/// Reads text by line while preserving line terminators.
///
/// A line is terminated by `\n`. If the input uses `\r\n`, both characters are
/// preserved in the output because `\n` is still the terminating scalar value.
pub trait TextLineRead: TextRead {
    /// Reads one line into `output`.
    ///
    /// # Parameters
    /// - `output`: Destination string. The line is appended to existing
    ///   content.
    ///
    /// # Returns
    /// `true` when a line or a final unterminated line was read, or `false` at
    /// EOF with no text appended.
    ///
    /// # Errors
    /// Returns an implementation-specific error when the source cannot be read
    /// or decoded.
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error>;
}
