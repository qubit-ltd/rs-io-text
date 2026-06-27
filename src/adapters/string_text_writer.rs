// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::convert::Infallible;

use crate::{LineEnding, OutputTextWriter, StringCharOutput, TextWrite};

/// Text writer over a borrowed [`String`] with configurable line endings.
#[derive(Debug)]
pub struct StringTextWriter<'a> {
    writer: OutputTextWriter<StringCharOutput<'a>>,
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
    pub fn new(output: &'a mut String) -> Self {
        Self {
            writer: OutputTextWriter::new(StringCharOutput::new(output)),
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
    pub const fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.writer = self.writer.with_line_ending(line_ending);
        self
    }

    /// Returns the wrapped string.
    ///
    /// # Returns
    /// A shared reference to the destination string.
    #[must_use]
    pub fn get_ref(&self) -> &String {
        self.writer.get_ref().get_ref()
    }

    /// Returns the wrapped string mutably.
    ///
    /// # Returns
    /// A mutable reference to the destination string.
    pub fn get_mut(&mut self) -> &mut String {
        self.writer.get_mut().get_mut()
    }
}

impl TextWrite for StringTextWriter<'_> {
    type Error = std::io::Error;

    #[inline]
    fn line_ending(&self) -> LineEnding {
        self.writer.line_ending()
    }

    #[inline]
    fn write_char(&mut self, ch: char) -> Result<(), Self::Error> {
        self.writer.write_char(ch)
    }

    #[inline]
    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        self.writer.write_chars(chars)
    }

    #[inline]
    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        self.writer.write_str(text)
    }

    #[inline]
    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.writer.write_line(line)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.writer.flush()
    }
}

impl TextWrite for String {
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
