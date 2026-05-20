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

use encoding_rs::Encoding;

use crate::{
    CodingErrorPolicy,
    LineEnding,
    TextWrite,
};

/// Text writer that encodes Unicode text with an explicit encoding.
#[derive(Debug)]
pub struct EncodedTextWriter<W> {
    inner: W,
    encoding: &'static Encoding,
    policy: CodingErrorPolicy,
    line_ending: LineEnding,
}

impl<W> EncodedTextWriter<W>
where
    W: Write,
{
    /// Creates an encoded text writer.
    ///
    /// # Parameters
    /// - `inner`: Byte writer to receive encoded bytes.
    /// - `encoding`: Encoding used for outgoing text.
    /// - `policy`: Unencodable text handling policy.
    ///
    /// # Returns
    /// A text writer using LF line endings.
    #[must_use]
    pub const fn new(inner: W, encoding: &'static Encoding, policy: CodingErrorPolicy) -> Self {
        Self {
            inner,
            encoding,
            policy,
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

impl<W> TextWrite for EncodedTextWriter<W>
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
        let mut text = String::new();
        for ch in chars {
            text.push(*ch);
        }
        self.write_str(&text)
    }

    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        let (bytes, _, had_errors) = self.encoding.encode(text);
        if had_errors && self.policy == CodingErrorPolicy::Strict {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("text cannot be encoded as {}", self.encoding.name()),
            ));
        }
        self.inner.write_all(bytes.as_ref())
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
