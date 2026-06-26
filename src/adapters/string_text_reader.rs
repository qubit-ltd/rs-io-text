// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use crate::{InputTextReader, StringInput, TextLineRead, TextRead};

/// Text reader over an owned string.
#[derive(Debug)]
pub struct StringTextReader {
    reader: InputTextReader<StringInput>,
}

impl StringTextReader {
    /// Creates a reader over owned text.
    ///
    /// # Parameters
    /// - `text`: Text to own and read.
    ///
    /// # Returns
    /// A reader positioned at the start of the text.
    #[must_use]
    pub fn new(text: String) -> Self {
        Self {
            reader: InputTextReader::new(StringInput::new(text)),
        }
    }

    /// Returns the current byte position in the underlying string.
    ///
    /// # Returns
    /// The current byte position.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.reader.get_ref().position()
    }

    /// Returns the owned string.
    ///
    /// # Returns
    /// The original string owned by this reader.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.reader.into_inner().into_inner()
    }
}

impl TextRead for StringTextReader {
    type Error = std::io::Error;

    #[inline]
    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        self.reader.read_char()
    }

    #[inline]
    fn read_chars(&mut self, output: &mut Vec<char>, max: usize) -> Result<usize, Self::Error> {
        self.reader.read_chars(output, max)
    }

    #[inline]
    fn read_to_string(&mut self, output: &mut String) -> Result<usize, Self::Error> {
        self.reader.read_to_string(output)
    }
}

impl TextLineRead for StringTextReader {
    #[inline]
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        self.reader.read_line(output)
    }
}
