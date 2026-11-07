// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::fmt;
use std::io;

use qubit_io::BoxOutput;
use qubit_io::BufferedOutput;
use qubit_io::CharOutput;

use crate::LineEnding;
use crate::TextWriter;

/// Default character chunk capacity for string writes.
const DEFAULT_CHAR_CHUNK_CAPACITY: usize = 256;

/// Text writer over a boxed `qubit_io::CharOutput`.
///
/// Unbuffered outputs are wrapped in [`BufferedOutput`] automatically, so a
/// successful write may only mean that characters entered the buffer. Call and
/// handle [`TextWriter::flush`] explicitly when delivery errors must be
/// observable. Dropping this writer cannot report a final flush error, and
/// [`OutputTextWriter::into_inner`] does not flush. A successful flush does not
/// guarantee persistence, and it does not finish a charset encoder lifecycle.
///
/// # Examples
///
/// ```
/// use qubit_io_text::OutputTextWriter;
/// use qubit_io_text::StringCharOutput;
/// use qubit_io_text::TextWriter;
///
/// let mut text = String::new();
/// {
///     let output = StringCharOutput::new(&mut text);
///     let mut writer = OutputTextWriter::new(output);
///     writer.write_line("hello 中")?;
///     writer.flush()?;
/// }
/// assert_eq!("hello 中\n", text);
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct OutputTextWriter<'a> {
    /// Owns the character output, including any automatically added buffer.
    /// Successful acceptance does not guarantee delivery to the original sink.
    output: Box<dyn CharOutput + 'a>,
    /// Terminator appended after the body of each complete line write.
    line_ending: LineEnding,
}

impl<'a> OutputTextWriter<'a> {
    /// Creates a text writer over a character output.
    ///
    /// Unbuffered outputs are wrapped in [`BufferedOutput`] automatically.
    ///
    /// # Type Parameters
    /// - `O`: Character output type owned by this writer.
    ///
    /// # Parameters
    /// - `output`: Character output to write.
    ///
    /// # Returns
    /// A text writer using LF line endings.
    #[must_use]
    pub fn new<O>(output: O) -> Self
    where
        O: CharOutput + 'a,
    {
        let output: Box<dyn CharOutput + 'a> = if output.is_buffered() {
            Box::new(output)
        } else {
            Box::new(BufferedOutput::new(output))
        };
        Self {
            output,
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
    pub fn from_boxed(output: Box<dyn CharOutput + 'a>) -> Self {
        let output = if output.is_buffered() {
            output
        } else {
            Box::new(BufferedOutput::new(BoxOutput::new(output)))
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
    #[inline]
    pub fn get_ref(&self) -> &(dyn CharOutput + 'a) {
        self.output.as_ref()
    }

    /// Returns a mutable reference to the currently wrapped character output.
    ///
    /// For an originally unbuffered output, this reference points to the
    /// automatically added [`BufferedOutput`]. Writes through this reference
    /// follow the same output's buffering order; they do not bypass its pending
    /// characters. Call [`TextWriter::flush`] when pending characters must be
    /// delivered or delivery errors must be observed.
    ///
    /// # Returns
    /// The current wrapped output trait object, including any automatic buffer.
    #[must_use]
    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut (dyn CharOutput + 'a) {
        self.output.as_mut()
    }

    /// Returns the wrapped output without flushing it.
    ///
    /// Call [`TextWriter::flush`] before this method when pending characters
    /// must reach the returned output before ownership is transferred.
    ///
    /// # Returns
    ///
    /// Returns the underlying boxed character output.
    #[must_use]
    #[inline(always)]
    pub fn into_inner(self) -> Box<dyn CharOutput + 'a> {
        self.output
    }
}

impl TextWriter for OutputTextWriter<'_> {
    /// I/O error type returned by writes and flushes to the wrapped output.
    type Error = io::Error;

    /// Returns the configured line terminator used by [`Self::write_line`].
    #[inline]
    fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    /// Writes one character to the wrapped output.
    ///
    /// An I/O error can occur after the output has accepted the character.
    ///
    /// # Parameters
    /// - `ch`: Character to write.
    ///
    /// # Errors
    /// Returns the wrapped output's error if it cannot accept the character.
    #[inline]
    fn write_char(&mut self, ch: char) -> Result<(), Self::Error> {
        self.output.write_fully(&[ch])
    }

    /// Writes the supplied characters to the wrapped output in order.
    ///
    /// An I/O error can occur after the output has accepted a prefix.
    ///
    /// # Parameters
    /// - `chars`: Characters to write in order.
    ///
    /// # Errors
    /// Returns the wrapped output's error if it cannot accept all characters.
    #[inline]
    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        self.output.write_fully(chars)
    }

    /// Writes UTF-8 text in bounded character chunks without allocating.
    ///
    /// An I/O error can occur after the output has accepted a prefix; the
    /// failing call does not report how many characters were accepted.
    ///
    /// # Parameters
    /// - `text`: UTF-8 text to write.
    ///
    /// # Errors
    /// Returns the wrapped output's error if it cannot accept all text.
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

    /// Writes the text followed by this writer's configured line terminator.
    ///
    /// If writing the terminator fails, the line content may already have
    /// been accepted by the output.
    ///
    /// # Parameters
    /// - `line`: Text to write before the configured line terminator.
    ///
    /// # Errors
    /// Returns the wrapped output's error if it cannot accept the line or its
    /// terminator. The line may have been accepted before a terminator error.
    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.write_str(line)?;
        self.write_str(self.line_ending.as_str())
    }

    /// Flushes buffered characters and reports delivery errors.
    ///
    /// # Errors
    /// Returns the wrapped output's error if flushing fails.
    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.output.flush()
    }
}

impl fmt::Debug for OutputTextWriter<'_> {
    /// Formats the writer's buffering state and configured line ending.
    ///
    /// # Parameters
    /// - `formatter`: Formatter receiving the debug representation.
    ///
    /// # Errors
    /// Returns the formatter's error if it cannot write the representation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OutputTextWriter")
            .field("is_buffered", &self.output.is_buffered())
            .field("line_ending", &self.line_ending)
            .finish()
    }
}
