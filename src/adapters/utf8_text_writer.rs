/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::io::{
    self,
    Write,
};

use crate::{
    LineEnding,
    TextWrite,
};

/// Text writer that encodes text as UTF-8 bytes.
#[derive(Debug)]
pub struct Utf8TextWriter<W> {
    inner: W,
    line_ending: LineEnding,
}

impl<W> Utf8TextWriter<W>
where
    W: Write,
{
    /// Creates a UTF-8 text writer.
    ///
    /// # Parameters
    /// - `inner`: Byte writer to receive UTF-8 bytes.
    ///
    /// # Returns
    /// A text writer using LF line endings.
    #[must_use]
    pub const fn new(inner: W) -> Self {
        Self {
            inner,
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

    /// Returns a shared reference to the wrapped byte writer.
    ///
    /// # Returns
    /// The wrapped byte writer.
    #[must_use]
    pub const fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped byte writer.
    ///
    /// # Returns
    /// The wrapped byte writer.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Returns the wrapped byte writer.
    ///
    /// # Returns
    /// The underlying byte writer.
    #[must_use]
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W> TextWrite for Utf8TextWriter<W>
where
    W: Write,
{
    type Error = io::Error;

    #[inline]
    fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    #[inline]
    fn write_char(&mut self, ch: char) -> Result<(), Self::Error> {
        let mut buffer = [0_u8; 4];
        self.write_str(ch.encode_utf8(&mut buffer))
    }

    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        for ch in chars {
            self.write_char(*ch)?;
        }
        Ok(())
    }

    #[inline]
    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        self.inner.write_all(text.as_bytes())
    }

    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.write_str(line)?;
        self.write_str(self.line_ending.as_str())
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush()
    }
}
