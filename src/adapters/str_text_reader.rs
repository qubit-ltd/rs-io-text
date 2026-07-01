// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::convert::Infallible;

use crate::{
    TextLineRead,
    TextRead,
    adapters::text_cursor::{
        read_char_at,
        read_chars_at,
        read_line_at,
        read_to_string_at,
    },
};

/// Text reader over a borrowed string slice.
#[derive(Debug)]
pub struct StrTextReader<'a> {
    text: &'a str,
    position: usize,
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
        Self { text, position: 0 }
    }

    /// Returns the current byte position in the underlying string slice.
    ///
    /// # Returns
    /// The current byte position.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }
}

impl TextRead for StrTextReader<'_> {
    type Error = Infallible;

    #[inline]
    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        Ok(read_char_at(self.text, &mut self.position))
    }

    #[inline]
    fn read_chars(
        &mut self,
        output: &mut Vec<char>,
        max: usize,
    ) -> Result<usize, Self::Error> {
        Ok(read_chars_at(self.text, &mut self.position, output, max))
    }

    #[inline]
    fn read_to_string(
        &mut self,
        output: &mut String,
    ) -> Result<usize, Self::Error> {
        Ok(read_to_string_at(self.text, &mut self.position, output))
    }
}

impl TextLineRead for StrTextReader<'_> {
    #[inline]
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        Ok(read_line_at(self.text, &mut self.position, output))
    }
}
