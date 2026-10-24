// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::convert::Infallible;

use crate::TextLineReader;
use crate::TextReader;
use crate::adapters::text_cursor::read_char_at;
use crate::adapters::text_cursor::read_chars_at;
use crate::adapters::text_cursor::read_to_string_at;
use crate::line_ending_set::LineEndingSet;
use crate::line_ending_set::read_line_with;

/// Text reader over a borrowed string slice.
///
/// The reader borrows its input and tracks its physical UTF-8 byte offset,
/// including any character read ahead while recognizing line endings. It
/// recognizes LF, CRLF, and CR line endings by default.
///
/// # Type Parameters
///
/// - `'a`: Lifetime of the borrowed text.
///
/// # Examples
///
/// ```
/// use qubit_io_text::StrTextReader;
/// use qubit_io_text::TextReader;
///
/// let mut reader = StrTextReader::new("hello");
/// let mut output = String::new();
/// let count = reader.read_to_string(&mut output)?;
/// assert_eq!(count, 5);
/// assert_eq!(output, "hello");
/// # Ok::<(), std::convert::Infallible>(())
/// ```
#[derive(Debug)]
pub struct StrTextReader<'a> {
    /// Borrowed text being consumed by this reader.
    text: &'a str,
    /// Physical UTF-8 byte offset, including line-ending lookahead.
    position: usize,
    /// Line terminators recognized by [`TextLineReader::read_line`].
    line_endings: LineEndingSet,
    /// Character retained after line-ending lookahead for the next read.
    pending_char: Option<char>,
}

impl<'a> StrTextReader<'a> {
    /// Creates a reader over `text`.
    ///
    /// # Parameters
    /// - `text`: Borrowed text to read.
    ///
    /// # Returns
    /// A reader positioned at the start of the text.
    #[must_use]
    #[inline]
    pub const fn new(text: &'a str) -> Self {
        Self {
            text,
            position: 0,
            line_endings: LineEndingSet::ALL,
            pending_char: None,
        }
    }

    /// Returns the physical UTF-8 byte position in the underlying string.
    ///
    /// Line-ending detection may read one character ahead and retain it for the
    /// next read. This position includes that character and can equal the input
    /// length before all characters have been returned. Reading a retained
    /// character does not advance this position. Do not use this value as a
    /// checkpoint of text already delivered to the caller.
    ///
    /// # Returns
    /// The underlying byte offset, including any line-ending lookahead.
    #[must_use]
    #[inline]
    pub const fn position(&self) -> usize {
        self.position
    }

    /// Returns the line endings recognized by this reader.
    #[must_use]
    #[inline]
    pub const fn line_endings(&self) -> LineEndingSet {
        self.line_endings
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
    pub const fn with_line_endings(mut self, line_endings: LineEndingSet) -> Self {
        self.line_endings = line_endings;
        self
    }
}

impl TextReader for StrTextReader<'_> {
    type Error = Infallible;

    #[inline]
    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        if let Some(ch) = self.pending_char.take() {
            return Ok(Some(ch));
        }
        Ok(read_char_at(self.text, &mut self.position))
    }

    #[inline]
    fn read_chars(&mut self, output: &mut Vec<char>, max: usize) -> Result<usize, Self::Error> {
        let mut count = 0;
        if max > 0
            && let Some(ch) = self.pending_char.take()
        {
            output.push(ch);
            count = 1;
        }
        Ok(count + read_chars_at(self.text, &mut self.position, output, max.saturating_sub(count)))
    }

    #[inline]
    fn read_to_string(&mut self, output: &mut String) -> Result<usize, Self::Error> {
        let mut count = 0;
        if let Some(ch) = self.pending_char.take() {
            output.push(ch);
            count = 1;
        }
        Ok(count + read_to_string_at(self.text, &mut self.position, output))
    }
}

impl TextLineReader for StrTextReader<'_> {
    #[inline]
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        let text = self.text;
        let position = &mut self.position;
        let mut pending_char = self.pending_char.take();
        let line_endings = self.line_endings;
        let result = read_line_with(line_endings, output, &mut pending_char, || {
            Ok(read_char_at(text, position))
        });
        self.pending_char = pending_char;
        result
    }
}
