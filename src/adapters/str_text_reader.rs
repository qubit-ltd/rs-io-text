// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::convert::Infallible;

use crate::TextLineRead;
use crate::TextRead;
use crate::adapters::text_cursor::read_char_at;
use crate::adapters::text_cursor::read_chars_at;
use crate::adapters::text_cursor::read_to_string_at;
use crate::line_ending_set::LineEndingSet;
use crate::line_ending_set::read_line_with;

/// Text reader over a borrowed string slice.
#[derive(Debug)]
pub struct StrTextReader<'a> {
    text: &'a str,
    position: usize,
    line_endings: LineEndingSet,
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
    pub const fn new(text: &'a str) -> Self {
        Self {
            text,
            position: 0,
            line_endings: LineEndingSet::ALL,
            pending_char: None,
        }
    }

    /// Returns the current byte position in the underlying string slice.
    ///
    /// # Returns
    /// The current byte position.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

    /// Sets the line endings recognized by [`TextLineRead::read_line`].
    ///
    /// # Parameters
    /// - `line_endings`: Accepted line-ending sequences.
    ///
    /// # Returns
    /// This reader with the requested line-ending configuration.
    #[must_use]
    pub const fn with_line_endings(mut self, line_endings: LineEndingSet) -> Self {
        self.line_endings = line_endings;
        self
    }

    /// Returns the line endings recognized by this reader.
    #[must_use]
    pub const fn line_endings(&self) -> LineEndingSet {
        self.line_endings
    }
}

impl TextRead for StrTextReader<'_> {
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

impl TextLineRead for StrTextReader<'_> {
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
