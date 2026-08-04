// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
use crate::TextRead;

/// Reads text by line while preserving line terminators.
///
/// Built-in text readers accept LF, CRLF, and CR by default, and preserve the
/// complete terminator in the output. Readers expose a line-ending set when a
/// narrower or broader policy is required. Custom implementations may retain
/// their own line-boundary policy.
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
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        let mut read = false;
        while let Some(ch) = self.read_char()? {
            output.push(ch);
            read = true;
            if ch == '\n' {
                break;
            }
        }
        Ok(read)
    }
}
