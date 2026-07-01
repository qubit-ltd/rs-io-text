// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use qubit_io::Input;

use crate::{StringCharInput, TextLineRead, TextRead};

/// Default character chunk capacity for owned string reads.
const DEFAULT_CHAR_CHUNK_CAPACITY: usize = 256;

/// Text reader over an owned string.
#[derive(Debug)]
pub struct StringTextReader {
    input: StringCharInput,
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
            input: StringCharInput::new(text),
        }
    }

    /// Returns the current byte position in the underlying string.
    ///
    /// # Returns
    /// The current byte position.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.input.position()
    }

    /// Returns the owned string.
    ///
    /// # Returns
    /// The original string owned by this reader.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.input.into_inner()
    }
}

impl TextRead for StringTextReader {
    type Error = std::io::Error;

    #[inline]
    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        let mut ch = ['\0'];
        if self.input.read(&mut ch)? == 0 {
            Ok(None)
        } else {
            Ok(Some(ch[0]))
        }
    }

    #[inline]
    fn read_chars(&mut self, output: &mut Vec<char>, max: usize) -> Result<usize, Self::Error> {
        let mut count = 0;
        let mut chars = ['\0'; DEFAULT_CHAR_CHUNK_CAPACITY];
        while count < max {
            let requested = (max - count).min(chars.len());
            let read = self.input.read_fully(&mut chars[..requested])?;
            if read == 0 {
                break;
            }
            output.extend_from_slice(&chars[..read]);
            count += read;
        }
        Ok(count)
    }

    #[inline]
    fn read_to_string(&mut self, output: &mut String) -> Result<usize, Self::Error> {
        let mut count = 0;
        let mut chars = ['\0'; DEFAULT_CHAR_CHUNK_CAPACITY];
        loop {
            let read = self.input.read_fully(&mut chars)?;
            if read == 0 {
                return Ok(count);
            }
            output.extend(chars[..read].iter().copied());
            count += read;
        }
    }
}

impl TextLineRead for StringTextReader {
    #[inline]
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        let mut read = false;
        while let Some(ch) = self.read_char()? {
            output.push(ch);
            read = true;
            if ch == '\n' {
                break;
            }
        }
        Ok(read)
    }
}
