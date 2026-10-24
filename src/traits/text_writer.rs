// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use crate::LineEnding;

/// Writes Unicode text to a text sink.
///
/// `TextWriter` accepts Rust Unicode text and delegates byte encoding, storage,
/// logging, or other sink-specific behavior to the concrete implementation. An
/// error may occur after a prefix has been accepted, buffered, or delivered to
/// the underlying sink. The error does not report the consumed prefix, so
/// retrying the complete input can duplicate text. Successful acceptance does
/// not by itself guarantee that buffered output has been flushed.
///
/// # Examples
///
/// ```
/// use qubit_io_text::StringTextWriter;
/// use qubit_io_text::TextWriter;
///
/// let mut output = String::new();
/// let mut writer = StringTextWriter::new(&mut output);
/// writer.write_line("hello").expect("writing to a string succeeds");
///
/// assert_eq!(output, "hello\n");
/// ```
pub trait TextWriter {
    /// Error returned by this text sink.
    type Error;

    /// Returns the configured line ending for [`TextWriter::write_line`].
    ///
    /// # Returns
    /// The configured line ending. Implementations default to
    /// [`LineEnding::Lf`].
    #[inline]
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
    #[inline]
    fn write_char(&mut self, ch: char) -> Result<(), Self::Error> {
        let mut buffer = [0_u8; 4];
        self.write_str(ch.encode_utf8(&mut buffer))
    }

    /// Writes a slice of Unicode scalar values.
    ///
    /// # Parameters
    /// - `chars`: Characters to write in order.
    ///
    /// # Errors
    /// Returns an implementation-specific error when any character cannot be
    /// accepted by the sink. An error may occur after a prefix has been
    /// accepted, buffered, or delivered to the underlying sink. The error
    /// does not report the consumed prefix, so retrying the complete input
    /// can duplicate text. Successful acceptance does not by itself
    /// guarantee that buffered output has been flushed.
    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        for ch in chars {
            self.write_char(*ch)?;
        }
        Ok(())
    }

    /// Writes a string slice.
    ///
    /// # Parameters
    /// - `text`: Text to write.
    ///
    /// # Errors
    /// Returns an implementation-specific error when the text cannot be
    /// accepted by the sink. An error may occur after a prefix has been
    /// accepted, buffered, or delivered to the underlying sink. The error
    /// does not report the consumed prefix, so retrying the complete input
    /// can duplicate text. Successful acceptance does not by itself
    /// guarantee that buffered output has been flushed.
    fn write_str(&mut self, text: &str) -> Result<(), Self::Error>;

    /// Writes a line followed by the configured line ending.
    ///
    /// # Parameters
    /// - `line`: Line content to write. The configured line ending is appended.
    ///
    /// # Errors
    /// Returns an implementation-specific error when the line or line ending
    /// cannot be accepted by the sink. An error may occur after a prefix has
    /// been accepted, buffered, or delivered to the underlying sink. The error
    /// does not report the consumed prefix, so retrying the complete input can
    /// duplicate text. If the line content succeeds but its terminator fails,
    /// the content remains written. Successful acceptance does not by itself
    /// guarantee that buffered output has been flushed.
    #[inline]
    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.write_str(line)?;
        self.write_str(self.line_ending().as_str())
    }

    /// Flushes buffered text to the underlying sink.
    ///
    /// # Errors
    /// Returns an implementation-specific error when flushing fails.
    fn flush(&mut self) -> Result<(), Self::Error>;
}
