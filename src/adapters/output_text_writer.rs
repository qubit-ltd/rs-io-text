// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io;

use qubit_io::{
    Output,
    OutputExt,
};

use crate::{
    LineEnding,
    TextWrite,
};

/// Default character chunk capacity for string writes.
const DEFAULT_CHAR_CHUNK_CAPACITY: usize = 256;

/// Text writer over a `qubit_io::Output<Item = char>`.
#[derive(Debug)]
pub struct OutputTextWriter<O>
where
    O: Output<Item = char>,
{
    output: O,
    line_ending: LineEnding,
}

impl<O> OutputTextWriter<O>
where
    O: Output<Item = char>,
{
    /// Creates a text writer over a character output.
    ///
    /// # Parameters
    /// - `output`: Character output to write.
    ///
    /// # Returns
    /// A text writer using LF line endings.
    #[must_use]
    pub const fn new(output: O) -> Self {
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
    pub const fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Returns a shared reference to the wrapped output.
    ///
    /// # Returns
    /// The wrapped output.
    #[must_use]
    pub const fn get_ref(&self) -> &O {
        &self.output
    }

    /// Returns a mutable reference to the wrapped output.
    ///
    /// # Returns
    /// The wrapped output.
    pub fn get_mut(&mut self) -> &mut O {
        &mut self.output
    }

    /// Returns the wrapped output.
    ///
    /// # Returns
    /// The underlying character output.
    #[must_use]
    pub fn into_inner(self) -> O {
        self.output
    }
}

impl<O> TextWrite for OutputTextWriter<O>
where
    O: Output<Item = char>,
{
    type Error = io::Error;

    #[inline]
    fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    #[inline]
    fn write_char(&mut self, ch: char) -> Result<(), Self::Error> {
        self.output.write_all(&[ch])
    }

    #[inline]
    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        self.output.write_all(chars)
    }

    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        let mut chars = ['\0'; DEFAULT_CHAR_CHUNK_CAPACITY];
        let mut count = 0;
        for ch in text.chars() {
            chars[count] = ch;
            count += 1;
            if count == chars.len() {
                self.output.write_all(&chars)?;
                count = 0;
            }
        }
        if count > 0 {
            self.output.write_all(&chars[..count])?;
        }
        Ok(())
    }

    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.write_str(line)?;
        self.write_str(self.line_ending.as_str())
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.output.flush()
    }
}
