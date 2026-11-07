// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::convert::Infallible;
use std::io;

use qubit_utils::try_reserve_string;

use crate::LineEnding;
use crate::TextWriter;

/// Text writer over a borrowed [`String`] with configurable line endings.
///
/// # Examples
///
/// ```
/// use qubit_io_text::StringTextWriter;
/// use qubit_io_text::TextWriter;
///
/// let mut output = String::new();
/// let mut writer = StringTextWriter::new(&mut output);
/// writer.write_line("hello")?;
/// assert_eq!(output, "hello\n");
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct StringTextWriter<'a> {
    /// Destination string that receives each written character.
    output: &'a mut String,
    /// Terminator appended by [`TextWriter::write_line`].
    line_ending: LineEnding,
}

impl<'a> StringTextWriter<'a> {
    /// Creates a writer over `output`.
    ///
    /// # Parameters
    /// - `output`: Destination string.
    ///
    /// # Returns
    /// A string text writer using LF line endings.
    #[must_use]
    #[inline]
    pub fn new(output: &'a mut String) -> Self {
        Self {
            output,
            line_ending: LineEnding::Lf,
        }
    }

    /// Sets the line ending for this writer.
    ///
    /// # Parameters
    /// - `line_ending`: Line ending to use for subsequent lines.
    ///
    /// # Returns
    /// This writer with the configured line ending.
    #[must_use]
    #[inline]
    pub const fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Returns the wrapped string.
    ///
    /// # Returns
    /// A shared reference to the destination string.
    #[must_use]
    #[inline]
    pub fn get_ref(&self) -> &String {
        self.output
    }

    /// Returns the wrapped string mutably.
    ///
    /// # Returns
    /// A mutable reference to the destination string.
    #[must_use]
    #[inline]
    pub fn get_mut(&mut self) -> &mut String {
        self.output
    }
}

impl TextWriter for StringTextWriter<'_> {
    /// Error type used when reserving capacity in the destination string fails.
    type Error = io::Error;

    /// Returns the line ending configured when this writer was constructed.
    ///
    /// # Returns
    /// The configured line ending.
    #[inline]
    fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    /// Appends one character after reserving enough space for its UTF-8 bytes.
    ///
    /// # Parameters
    /// - `ch`: Character to append.
    ///
    /// # Errors
    /// Returns an I/O error if reserving the required capacity fails. The
    /// destination is unchanged when reservation fails.
    #[inline]
    fn write_char(&mut self, ch: char) -> Result<(), Self::Error> {
        try_reserve_string(self.output, ch.len_utf8())?;
        self.output.push(ch);
        Ok(())
    }

    /// Appends characters in order after reserving space for their UTF-8 bytes.
    ///
    /// # Parameters
    /// - `chars`: Characters to append in order.
    ///
    /// # Errors
    /// Returns an I/O error if reserving capacity fails. No characters are
    /// appended when reservation fails.
    #[inline]
    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        let additional = chars.iter().map(|ch| ch.len_utf8()).sum();
        try_reserve_string(self.output, additional)?;
        self.output.extend(chars.iter().copied());
        Ok(())
    }

    /// Appends text after reserving space for its UTF-8 bytes.
    ///
    /// # Parameters
    /// - `text`: Text to append.
    ///
    /// # Errors
    /// Returns an I/O error if reserving capacity fails. The destination is
    /// unchanged when reservation fails.
    #[inline]
    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        try_reserve_string(self.output, text.len())?;
        self.output.push_str(text);
        Ok(())
    }

    /// Appends the line and then the configured line ending.
    ///
    /// # Parameters
    /// - `line`: Line content to append before the configured terminator.
    ///
    /// # Errors
    /// Returns an I/O error if capacity reservation fails. If reserving the
    /// terminator fails, the line content has already been appended.
    #[inline]
    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.write_str(line)?;
        self.write_str(self.line_ending.as_str())
    }

    /// Completes the write operation; a `String` has no buffered sink to flush.
    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl TextWriter for String {
    /// Error type for string writes, which cannot fail.
    type Error = Infallible;

    /// Appends one character to the string.
    ///
    /// # Parameters
    /// - `ch`: Character to append.
    #[inline]
    fn write_char(&mut self, ch: char) -> Result<(), Self::Error> {
        self.push(ch);
        Ok(())
    }

    /// Appends characters to the string in slice order.
    ///
    /// # Parameters
    /// - `chars`: Characters to append in order.
    #[inline]
    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        self.extend(chars.iter().copied());
        Ok(())
    }

    /// Appends the supplied text to the string.
    ///
    /// # Parameters
    /// - `text`: Text to append.
    #[inline]
    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        self.push_str(text);
        Ok(())
    }

    /// Appends the line followed by the default LF line ending.
    ///
    /// # Parameters
    /// - `line`: Line content to append before the LF terminator.
    #[inline]
    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.push_str(line);
        self.push_str(self.line_ending().as_str());
        Ok(())
    }

    /// Completes immediately because a `String` has no buffered sink.
    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
