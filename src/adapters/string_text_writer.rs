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
    type Error = io::Error;

    #[inline]
    fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    #[inline]
    fn write_char(&mut self, ch: char) -> Result<(), Self::Error> {
        try_reserve_string(self.output, ch.len_utf8())?;
        self.output.push(ch);
        Ok(())
    }

    #[inline]
    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        let additional = chars.iter().map(|ch| ch.len_utf8()).sum();
        try_reserve_string(self.output, additional)?;
        self.output.extend(chars.iter().copied());
        Ok(())
    }

    #[inline]
    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        try_reserve_string(self.output, text.len())?;
        self.output.push_str(text);
        Ok(())
    }

    #[inline]
    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.write_str(line)?;
        self.write_str(self.line_ending.as_str())
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl TextWriter for String {
    type Error = Infallible;

    #[inline]
    fn write_char(&mut self, ch: char) -> Result<(), Self::Error> {
        self.push(ch);
        Ok(())
    }

    #[inline]
    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        self.extend(chars.iter().copied());
        Ok(())
    }

    #[inline]
    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        self.push_str(text);
        Ok(())
    }

    #[inline]
    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.push_str(line);
        self.push_str(self.line_ending().as_str());
        Ok(())
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
