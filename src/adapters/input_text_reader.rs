// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::collections::VecDeque;
use std::fmt;
use std::io;

use qubit_io::BoxInput;
use qubit_io::BufferedInput;
use qubit_io::CharInput;

use crate::LineEnding;
use crate::TextLineReader;
use crate::TextReader;
use crate::line_ending_set::LineEndingSet;

/// Default character chunk capacity for text reads.
const DEFAULT_CHAR_CHUNK_CAPACITY: usize = 256;

/// Text reader over a boxed `qubit_io::CharInput`.
///
/// The reader buffers characters internally so line reads can preserve text
/// fetched beyond a line ending. Use [`into_parts`](Self::into_parts) when
/// consuming the reader if those pending characters must be retained.
///
/// # Examples
///
/// ```
/// use qubit_io_text::{InputTextReader, StringCharInput, TextReader};
///
/// let input = StringCharInput::new("hello".to_owned());
/// let mut reader = InputTextReader::new(input);
/// let mut text = String::new();
/// reader.read_to_string(&mut text)?;
/// assert_eq!(text, "hello");
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct InputTextReader<'a> {
    /// Owns the character input, including any buffer added during
    /// construction. Characters in the pending queue logically precede
    /// further input reads.
    input: Box<dyn CharInput + 'a>,
    /// Retains read-ahead characters in the order they must be returned.
    pending: VecDeque<char>,
    /// Selects terminators for line reads without changing character reads.
    line_endings: LineEndingSet,
}

impl<'a> InputTextReader<'a> {
    /// Creates a text reader over a character input.
    ///
    /// Unbuffered inputs are wrapped in [`BufferedInput`] automatically.
    ///
    /// # Parameters
    /// - `input`: Character input to read.
    ///
    /// # Returns
    /// A text reader wrapping `input`.
    #[must_use]
    #[inline]
    pub fn new<I>(input: I) -> Self
    where
        I: CharInput + 'a,
    {
        let input: Box<dyn CharInput + 'a> = if input.is_buffered() {
            Box::new(input)
        } else {
            Box::new(BufferedInput::new(input))
        };
        Self {
            input,
            pending: VecDeque::new(),
            line_endings: LineEndingSet::ALL,
        }
    }

    /// Creates a text reader from an already boxed character input.
    ///
    /// Unbuffered boxed inputs are wrapped in [`BufferedInput`] automatically.
    ///
    /// # Parameters
    /// - `input`: Boxed character input to read.
    ///
    /// # Returns
    /// A text reader wrapping `input`.
    #[must_use]
    #[inline]
    pub fn from_boxed(input: Box<dyn CharInput + 'a>) -> Self {
        let input = if input.is_buffered() {
            input
        } else {
            Box::new(BufferedInput::new(BoxInput::new(input)))
        };
        Self {
            input,
            pending: VecDeque::new(),
            line_endings: LineEndingSet::ALL,
        }
    }

    /// Returns a shared reference to the wrapped input.
    ///
    /// # Returns
    /// The wrapped input trait object.
    #[must_use]
    #[inline]
    pub fn get_ref(&self) -> &(dyn CharInput + 'a) {
        self.input.as_ref()
    }

    /// Returns a mutable reference to the wrapped input.
    ///
    /// Characters retained in this reader's pending queue are logically
    /// earlier than the wrapped input's current position. Reading the wrapped
    /// input directly can therefore reorder or skip text that this reader has
    /// already fetched.
    ///
    /// # Returns
    /// The wrapped input trait object.
    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut (dyn CharInput + 'a) {
        self.input.as_mut()
    }

    /// Returns the wrapped input and characters retained after a line boundary.
    ///
    /// # Returns
    ///
    /// Returns the underlying boxed character input and pending characters in
    /// logical read order.
    #[must_use = "the returned input and pending characters must be handled"]
    #[inline]
    pub fn into_parts(self) -> (Box<dyn CharInput + 'a>, Vec<char>) {
        (self.input, self.pending.into_iter().collect())
    }

    /// Returns the wrapped input while discarding pending characters.
    ///
    /// Pending characters already read past a line boundary are discarded;
    /// the returned input remains positioned after those characters. Use
    /// [`Self::into_parts`] when those characters must be preserved.
    ///
    /// # Returns
    /// The underlying boxed character input.
    #[must_use]
    #[inline]
    pub fn into_inner(self) -> Box<dyn CharInput + 'a> {
        self.input
    }

    /// Sets the line endings recognized by [`TextLineReader::read_line`].
    ///
    /// # Parameters
    /// - `line_endings`: Accepted line-ending sequences.
    ///
    /// # Returns
    /// This reader with the requested line-ending configuration.
    #[must_use]
    #[inline]
    pub fn with_line_endings(mut self, line_endings: LineEndingSet) -> Self {
        self.line_endings = line_endings;
        self
    }

    /// Returns the line endings recognized by this reader.
    #[must_use]
    #[inline]
    pub const fn line_endings(&self) -> LineEndingSet {
        self.line_endings
    }

    /// Reads one available character chunk, retrying interrupted source calls.
    ///
    /// # Parameters
    /// - `output`: Destination storage receiving the confirmed source prefix.
    ///
    /// # Returns
    /// Returns the number of characters transferred by one successful read.
    ///
    /// # Errors
    /// Returns the first non-interrupted source error without discarding
    /// earlier chunks.
    fn read_chunk(&mut self, output: &mut [char]) -> io::Result<usize> {
        loop {
            match self.input.read(output) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                result => return result,
            }
        }
    }

    /// Moves up to `max` queued characters into `output` and returns the count.
    fn drain_pending_chars(&mut self, output: &mut Vec<char>, max: usize) -> usize {
        let mut count = 0;
        while count < max {
            let Some(ch) = self.pending.pop_front() else {
                break;
            };
            output.push(ch);
            count += 1;
        }
        count
    }

    /// Moves every queued character into `output` and returns the count.
    fn drain_pending_string(&mut self, output: &mut String) -> usize {
        let mut count = 0;
        while let Some(ch) = self.pending.pop_front() {
            output.push(ch);
            count += 1;
        }
        count
    }
}

impl TextReader for InputTextReader<'_> {
    type Error = io::Error;

    #[inline]
    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        if let Some(ch) = self.pending.pop_front() {
            return Ok(Some(ch));
        }
        let mut ch = ['\0'];
        if self.read_chunk(&mut ch)? == 0 {
            Ok(None)
        } else {
            Ok(Some(ch[0]))
        }
    }

    fn read_chars(&mut self, output: &mut Vec<char>, max: usize) -> Result<usize, Self::Error> {
        let mut count = self.drain_pending_chars(output, max);
        let mut chars = ['\0'; DEFAULT_CHAR_CHUNK_CAPACITY];
        while count < max {
            let requested = (max - count).min(chars.len());
            let read = self.read_chunk(&mut chars[..requested])?;
            if read == 0 {
                break;
            }
            output.extend_from_slice(&chars[..read]);
            count += read;
        }
        Ok(count)
    }

    fn read_to_string(&mut self, output: &mut String) -> Result<usize, Self::Error> {
        let mut count = self.drain_pending_string(output);
        let mut chars = ['\0'; DEFAULT_CHAR_CHUNK_CAPACITY];
        loop {
            let read = self.read_chunk(&mut chars)?;
            if read == 0 {
                return Ok(count);
            }
            output.extend(chars[..read].iter().copied());
            count += read;
        }
    }
}

impl TextLineReader for InputTextReader<'_> {
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        let line_endings = self.line_endings;
        let mut pending = std::mem::take(&mut self.pending);
        let mut read = false;
        let mut chars = ['\0'; DEFAULT_CHAR_CHUNK_CAPACITY];
        loop {
            if pending.is_empty() {
                let count = self.read_chunk(&mut chars)?;
                if count == 0 {
                    self.pending = pending;
                    return Ok(read);
                }
                pending.extend(chars[..count].iter().copied());
            }

            let ch = pending.pop_front().expect("pending character is available");
            read = true;
            if ch == '\n' && line_endings.contains(LineEnding::Lf) {
                output.push(ch);
                self.pending = pending;
                return Ok(true);
            }
            if ch == '\r' {
                if line_endings.contains(LineEnding::CrLf) {
                    if pending.is_empty() {
                        let count = match self.read_chunk(&mut chars) {
                            Ok(count) => count,
                            Err(error) => {
                                pending.push_front('\r');
                                self.pending = pending;
                                return Err(error);
                            }
                        };
                        pending.extend(chars[..count].iter().copied());
                    }
                    match pending.pop_front() {
                        Some('\n') => {
                            output.push('\r');
                            output.push('\n');
                            self.pending = pending;
                            return Ok(true);
                        }
                        Some(next) => {
                            output.push('\r');
                            pending.push_front(next);
                            if line_endings.contains(LineEnding::Cr) {
                                self.pending = pending;
                                return Ok(true);
                            }
                            continue;
                        }
                        None => {
                            output.push('\r');
                            self.pending = pending;
                            return Ok(true);
                        }
                    }
                }
                if line_endings.contains(LineEnding::Cr) {
                    output.push('\r');
                    self.pending = pending;
                    return Ok(true);
                }
            }
            output.push(ch);
        }
    }
}

impl fmt::Debug for InputTextReader<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InputTextReader")
            .field("is_buffered", &self.input.is_buffered())
            .field("pending_len", &self.pending.len())
            .finish()
    }
}
