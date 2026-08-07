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
///
/// Implementations may suspend while waiting for sink capacity. Returned
/// futures are not required to be `Send`; callers that move writes between
/// threads must use an implementation that provides the required future
/// bounds.
/// Cancellation behavior is implementation-specific; callers must consult the
/// concrete writer before retrying a cancelled operation.
#[allow(async_fn_in_trait)]
pub trait AsyncTextWrite {
    /// Error returned by this text sink.
    type Error;

    /// Returns the configured line ending.
    ///
    /// # Returns
    ///
    /// Returns the line ending appended by [`Self::write_line_fully_async`].
    fn line_ending(&self) -> LineEnding {
        LineEnding::Lf
    }

    /// Writes one Unicode scalar value.
    ///
    /// # Parameters
    ///
    /// - `ch` - Scalar value to write.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific encoding or sink error.
    async fn write_char_async(&mut self, ch: char) -> Result<(), Self::Error>;

    /// Writes one step of a character slice and returns the consumed count.
    ///
    /// A successful call with a nonempty input must consume at least one
    /// character. Callers can resume a partial write with the unconsumed
    /// suffix.
    ///
    /// # Parameters
    ///
    /// - `chars` - Characters to write in order.
    ///
    /// # Returns
    ///
    /// Returns the number of consumed characters in `0..=chars.len()`.
    /// Returns zero only when `chars` is empty.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific encoding or sink error. A source
    /// prefix can already be committed when an error is returned.
    async fn write_chars_async(
        &mut self,
        chars: &[char],
    ) -> Result<usize, Self::Error>;

    /// Writes one step of a UTF-8 string and returns the consumed byte count.
    ///
    /// A successful call with a nonempty input must consume a nonempty,
    /// character-boundary prefix. Callers can resume a partial write with
    /// `&text[consumed..]`.
    ///
    /// # Parameters
    ///
    /// - `text` - UTF-8 text to write.
    ///
    /// # Returns
    ///
    /// Returns the byte length of the consumed prefix in `0..=text.len()`.
    /// Returns zero only when `text` is empty.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific encoding or sink error. A source
    /// prefix can already be committed when an error is returned.
    async fn write_str_async(
        &mut self,
        text: &str,
    ) -> Result<usize, Self::Error>;

    /// Writes an entire character slice.
    ///
    /// # Parameters
    ///
    /// - `chars` - Characters to write in order.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific encoding or sink error. A prefix can
    /// already be committed when an error is returned.
    ///
    /// # Panics
    ///
    /// Panics when [`Self::write_chars_async`] violates its nonzero-progress
    /// or bounded-progress contract for a nonempty input.
    async fn write_chars_fully_async(
        &mut self,
        chars: &[char],
    ) -> Result<(), Self::Error> {
        let mut index = 0;
        while index < chars.len() {
            let written = self.write_chars_async(&chars[index..]).await?;
            let remaining = chars.len() - index;
            assert!(
                written > 0,
                "AsyncTextWrite::write_chars_async returned zero for nonempty input"
            );
            assert!(
                written <= remaining,
                "AsyncTextWrite::write_chars_async returned more characters than supplied"
            );
            index += written;
        }
        Ok(())
    }

    /// Writes an entire UTF-8 string.
    ///
    /// # Parameters
    ///
    /// - `text` - UTF-8 text to write.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific encoding or sink error. A prefix can
    /// already be committed when an error is returned.
    ///
    /// # Panics
    ///
    /// Panics when [`Self::write_str_async`] violates its nonzero-progress
    ///, bounded-progress, or UTF-8 character-boundary contract for a nonempty
    /// input.
    async fn write_str_fully_async(
        &mut self,
        text: &str,
    ) -> Result<(), Self::Error> {
        let mut offset = 0;
        while offset < text.len() {
            let written = self.write_str_async(&text[offset..]).await?;
            let remaining = text.len() - offset;
            assert!(
                written > 0,
                "AsyncTextWrite::write_str_async returned zero for nonempty input"
            );
            assert!(
                written <= remaining,
                "AsyncTextWrite::write_str_async returned more bytes than supplied"
            );
            assert!(
                text.is_char_boundary(offset + written),
                "AsyncTextWrite::write_str_async returned a non-character-boundary prefix"
            );
            offset += written;
        }
        Ok(())
    }

    /// Writes a complete line and its configured line ending.
    ///
    /// # Parameters
    ///
    /// - `line` - Line content without the appended line ending.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific encoding or sink error. The line
    /// content can already be committed when writing its terminator fails.
    async fn write_line_fully_async(
        &mut self,
        line: &str,
    ) -> Result<(), Self::Error>;

    /// Flushes pending encoded output.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific sink error. A pending-byte prefix
    /// can already be committed when an error is returned.
    async fn flush_async(&mut self) -> Result<(), Self::Error>;

    /// Finishes the encoder and flushes pending output.
    ///
    /// After successful completion, implementations can reject later text
    /// writes because encoder finalization is terminal.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific encoder-finalization or sink error.
    async fn finish_async(&mut self) -> Result<(), Self::Error>;
}
