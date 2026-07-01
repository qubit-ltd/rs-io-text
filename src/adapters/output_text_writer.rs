// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::fmt;
use std::io;

use qubit_io::{BufferedOutput, Output};

use crate::{LineEnding, TextWrite};

/// Default character chunk capacity for string writes.
const DEFAULT_CHAR_CHUNK_CAPACITY: usize = 256;

/// Text writer over a boxed `qubit_io::Output<Item = char>`.
pub struct OutputTextWriter<'a> {
    output: Box<dyn Output<Item = char> + 'a>,
    line_ending: LineEnding,
}

impl<'a> OutputTextWriter<'a> {
    /// Creates a text writer over a character output.
    ///
    /// Unbuffered outputs are wrapped in [`BufferedOutput`] automatically.
    ///
    /// # Parameters
    /// - `output`: Character output to write.
    ///
    /// # Returns
    /// A text writer using LF line endings.
    #[must_use]
    pub fn new<O>(output: O) -> Self
    where
        O: Output<Item = char> + 'a,
    {
        Self {
            output: box_output(output),
            line_ending: LineEnding::Lf,
        }
    }

    /// Creates a text writer from an already boxed character output.
    ///
    /// Unbuffered boxed outputs are wrapped in [`BufferedOutput`]
    /// automatically.
    ///
    /// # Parameters
    /// - `output`: Boxed character output to write.
    ///
    /// # Returns
    /// A text writer using LF line endings.
    #[must_use]
    pub fn from_boxed(output: Box<dyn Output<Item = char> + 'a>) -> Self {
        let output = if output.is_buffered() {
            output
        } else {
            Box::new(BufferedOutput::new(BoxedCharOutput { output }))
        };
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
    /// The wrapped output trait object.
    #[must_use]
    pub fn get_ref(&self) -> &(dyn Output<Item = char> + 'a) {
        self.output.as_ref()
    }

    /// Returns a mutable reference to the wrapped output.
    ///
    /// # Returns
    /// The wrapped output trait object.
    pub fn get_mut(&mut self) -> &mut (dyn Output<Item = char> + 'a) {
        self.output.as_mut()
    }

    /// Flushes this writer and returns the wrapped output.
    ///
    /// # Returns
    /// The underlying boxed character output.
    ///
    /// # Errors
    /// Returns any error produced while flushing buffered characters.
    pub fn into_inner(mut self) -> io::Result<Box<dyn Output<Item = char> + 'a>> {
        self.output.flush()?;
        Ok(self.output)
    }
}

impl TextWrite for OutputTextWriter<'_> {
    type Error = io::Error;

    #[inline]
    fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    #[inline]
    fn write_char(&mut self, ch: char) -> Result<(), Self::Error> {
        self.output.write_fully(&[ch])
    }

    #[inline]
    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        self.output.write_fully(chars)
    }

    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        let mut chars = ['\0'; DEFAULT_CHAR_CHUNK_CAPACITY];
        let mut count = 0;
        for ch in text.chars() {
            chars[count] = ch;
            count += 1;
            if count == chars.len() {
                self.output.write_fully(&chars)?;
                count = 0;
            }
        }
        if count > 0 {
            self.output.write_fully(&chars[..count])?;
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

impl fmt::Debug for OutputTextWriter<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OutputTextWriter")
            .field("is_buffered", &self.output.is_buffered())
            .field("line_ending", &self.line_ending)
            .finish()
    }
}

struct BoxedCharOutput<'a> {
    output: Box<dyn Output<Item = char> + 'a>,
}

impl Output for BoxedCharOutput<'_> {
    type Item = char;

    #[inline]
    unsafe fn write_unchecked(
        &mut self,
        input: &[char],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        // SAFETY: Forwarded from the caller.
        unsafe { self.output.write_unchecked(input, index, count) }
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

fn box_output<'a, O>(output: O) -> Box<dyn Output<Item = char> + 'a>
where
    O: Output<Item = char> + 'a,
{
    if output.is_buffered() {
        Box::new(output)
    } else {
        Box::new(BufferedOutput::new(output))
    }
}
