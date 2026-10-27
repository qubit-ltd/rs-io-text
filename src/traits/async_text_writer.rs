// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use crate::LineEnding;

/// Asynchronously writes Unicode text to a text sink.
///
/// Implementations may suspend while waiting for sink capacity. Returned
/// futures are not required to be `Send`; callers that move writes between
/// threads must use an implementation that provides the required future
/// bounds.
/// Cancellation behavior is implementation-specific; callers must consult the
/// concrete writer before retrying a cancelled operation.
///
/// # Examples
///
/// This function writes `A中🙂A中🙂done\n`. String progress counts UTF-8 bytes,
/// while character-slice progress counts `char` elements.
///
/// ```no_run
/// use qubit_io::AsyncByteOutput;
/// use qubit_io_text::AsyncTextWriter;
/// use qubit_io_text::AsyncUtf8TextWriter;
///
/// async fn write_text<O>(output: O) -> std::io::Result<()>
/// where
///     O: AsyncByteOutput + Unpin,
/// {
///     let mut writer = AsyncUtf8TextWriter::new(output);
///     let text = "A中🙂";
///     let mut byte_offset = 0;
///     while byte_offset < text.len() {
///         let bytes = AsyncTextWriter::write_str_async(
///             &mut writer,
///             &text[byte_offset..],
///         )
///         .await?;
///         byte_offset += bytes;
///     }
///
///     let chars = ['A', '中', '🙂'];
///     let mut char_index = 0;
///     while char_index < chars.len() {
///         let count = AsyncTextWriter::write_chars_async(
///             &mut writer,
///             &chars[char_index..],
///         )
///         .await?;
///         char_index += count;
///     }
///
///     AsyncTextWriter::write_line_fully_async(&mut writer, "done").await?;
///     AsyncTextWriter::finish_async(&mut writer).await?;
///     Ok(())
/// }
/// ```
///
/// Advance a source suffix only after a successful return. Errors propagate
/// to the caller; these loops do not make replaying the entire input safe
/// after an error or cancellation.
#[allow(async_fn_in_trait)]
pub trait AsyncTextWriter {
    /// Error returned by this text sink.
    type Error;

    /// Returns the configured line ending.
    ///
    /// # Returns
    ///
    /// Returns the line ending appended by [`Self::write_line_fully_async`].
    #[must_use]
    #[inline]
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
    async fn write_chars_async(&mut self, chars: &[char]) -> Result<usize, Self::Error>;

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
    async fn write_str_async(&mut self, text: &str) -> Result<usize, Self::Error>;

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
    /// # Cancellation safety
    ///
    /// The default implementation is not cancellation-safe. Earlier successful
    /// steps can commit a source prefix, while cancelling this future discards
    /// its internal character-index cursor (measured in characters by
    /// [`Self::write_chars_async`]). Replaying the complete input can duplicate
    /// that prefix. For cancellation-sensitive code, maintain the cursor in the
    /// caller and advance it immediately after each successful single-step
    /// call. The single-step method's cancellation and error guarantees
    /// still depend on the concrete implementation. Overrides must document
    /// their own cancellation guarantees.
    ///
    /// # Panics
    ///
    /// Panics when [`Self::write_chars_async`] violates its nonzero-progress
    /// or bounded-progress contract for a nonempty input.
    async fn write_chars_fully_async(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        let mut index = 0;
        while index < chars.len() {
            let written = self.write_chars_async(&chars[index..]).await?;
            let remaining = chars.len() - index;
            assert!(
                written > 0,
                "AsyncTextWriter::write_chars_async returned zero for nonempty input"
            );
            assert!(
                written <= remaining,
                "AsyncTextWriter::write_chars_async returned more characters than supplied"
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
    /// # Cancellation safety
    ///
    /// The default implementation is not cancellation-safe. Earlier successful
    /// steps can commit a source prefix, while cancelling this future discards
    /// its internal UTF-8 byte-offset cursor (measured in bytes by
    /// [`Self::write_str_async`]). Replaying the complete input can duplicate
    /// that prefix. For cancellation-sensitive code, maintain the cursor in the
    /// caller and advance it immediately after each successful single-step
    /// call. The single-step method's cancellation and error guarantees
    /// still depend on the concrete implementation. Overrides must document
    /// their own cancellation guarantees.
    ///
    /// # Panics
    ///
    /// Panics when [`Self::write_str_async`] violates its nonzero-progress
    /// bounded-progress, or UTF-8 character-boundary contract for a nonempty
    /// input.
    async fn write_str_fully_async(&mut self, text: &str) -> Result<(), Self::Error> {
        let mut offset = 0;
        while offset < text.len() {
            let written = self.write_str_async(&text[offset..]).await?;
            let remaining = text.len() - offset;
            assert!(
                written > 0,
                "AsyncTextWriter::write_str_async returned zero for nonempty input"
            );
            assert!(
                written <= remaining,
                "AsyncTextWriter::write_str_async returned more bytes than supplied"
            );
            assert!(
                text.is_char_boundary(offset + written),
                "AsyncTextWriter::write_str_async returned a non-character-boundary prefix"
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
    async fn write_line_fully_async(&mut self, line: &str) -> Result<(), Self::Error>;

    /// Flushes pending encoded output.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific sink error. A pending-byte prefix
    /// can already be committed when an error is returned.
    async fn flush_async(&mut self) -> Result<(), Self::Error>;

    /// Finishes the encoder and flushes pending output.
    ///
    /// Encoder finalization may complete before all pending output has been
    /// delivered or flushed. Therefore, an error or cancellation does not
    /// imply that the writer is still open for text writes, and this method
    /// does not guarantee that the encoder's state is rolled back.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific encoder-finalization or output
    /// error. The writer's state after an error, including whether calling
    /// this method again can make progress, depends on the implementation.
    /// Callers must not assume they can continue writing or replay previously
    /// supplied text.
    ///
    /// For example, after [`crate::AsyncCharsetTextWriter::finish_async`] has
    /// finalized its encoder, an output-delivery error can be retried by
    /// calling `finish_async` again on the same writer; text writes remain
    /// rejected. This recovery behavior is specific to that implementation
    /// and does not cover encoder-finalization errors in general.
    ///
    /// # Cancellation safety
    ///
    /// Cancelling this future may already have finalized the encoder and
    /// delivered a prefix of pending output. Whether the writer can be
    /// resumed, and how, depends on the implementation. In particular, the
    /// [`crate::AsyncCharsetTextWriter::finish_async`] implementation supports
    /// retrying output delivery on the same writer after cancellation, but
    /// this is not a general guarantee for all implementations.
    async fn finish_async(&mut self) -> Result<(), Self::Error>;
}
