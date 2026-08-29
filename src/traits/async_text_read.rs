// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
/// Asynchronously reads Unicode scalar values and strings from a text source.
///
/// Implementations may suspend while waiting for source data. Returned futures
/// are not required to be `Send`; callers that move reads between threads must
/// use an implementation that provides the required future bounds.
/// Cancellation behavior is implementation-specific; callers must consult the
/// concrete reader before retrying a cancelled operation.
#[allow(async_fn_in_trait)]
pub trait AsyncTextRead {
    /// Error returned by this text source.
    type Error;

    /// Reads the next Unicode scalar value.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(ch))` when one scalar value was read, or `Ok(None)` at
    /// EOF.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific source or decoding error.
    async fn read_char_async(&mut self) -> Result<Option<char>, Self::Error>;

    /// Reads up to `max` Unicode scalar values into `output`.
    ///
    /// Read characters are appended to `output`; its existing contents are
    /// preserved.
    ///
    /// # Parameters
    ///
    /// - `output` - Destination vector receiving appended characters.
    /// - `max` - Maximum number of characters to append. A value of zero does
    ///   not read from the source.
    ///
    /// # Returns
    ///
    /// Returns the number of characters appended before reaching `max` or EOF.
    ///
    /// # Errors
    ///
    /// Returns the first error reported while reading. A successfully appended
    /// prefix remains in `output` when a later read fails.
    async fn read_chars_async(&mut self, output: &mut Vec<char>, max: usize) -> Result<usize, Self::Error> {
        let mut count = 0;
        while count < max {
            let Some(ch) = self.read_char_async().await? else {
                break;
            };
            output.push(ch);
            count += 1;
        }
        Ok(count)
    }

    /// Reads all remaining text into `output`.
    ///
    /// The remaining decoded text is appended to `output`; its existing
    /// contents are preserved.
    ///
    /// # Parameters
    ///
    /// - `output` - Destination string receiving appended text.
    ///
    /// # Returns
    ///
    /// Returns the number of Unicode scalar values appended before EOF.
    ///
    /// # Errors
    ///
    /// Returns the first source or decoding error. A successfully appended
    /// prefix remains in `output` when a later read fails.
    async fn read_to_string_async(&mut self, output: &mut String) -> Result<usize, Self::Error> {
        let mut count = 0;
        while let Some(ch) = self.read_char_async().await? {
            output.push(ch);
            count += 1;
        }
        Ok(count)
    }
}
