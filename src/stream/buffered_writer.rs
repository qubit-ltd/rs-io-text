// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::{
    error::Error as StdError,
    io::{self, Write},
};

use qubit_codec::{TranscodeEncodeOutput, Transcoder};

use crate::{LineEnding, TextWrite};

/// Default byte buffer capacity used by buffered text writers.
const DEFAULT_BUFFER_CAPACITY: usize = 8 * 1024;

/// Default number of characters converted as one string-writing chunk.
const DEFAULT_CHAR_CHUNK_CAPACITY: usize = 256;

/// Buffered text writer driven by a character-to-byte transcoder.
///
/// This type owns a byte writer and a streaming encoder. Encoded bytes are
/// buffered by [`qubit_codec::TranscodeEncodeOutput`].
#[derive(Debug)]
pub struct BufferedWriter<W, E>
where
    W: Write,
{
    output: TranscodeEncodeOutput<W>,
    encoder: E,
    line_ending: LineEnding,
    char_buffer: Vec<char>,
    finished: bool,
}

impl<W, E> BufferedWriter<W, E>
where
    W: Write,
    E: Transcoder<char, u8>,
{
    /// Creates a buffered text writer with the default byte buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Byte writer that receives encoded bytes.
    /// - `encoder`: Streaming character-to-byte transcoder.
    ///
    /// # Returns
    ///
    /// Returns a buffered text writer using LF line endings.
    #[must_use]
    pub fn new(inner: W, encoder: E) -> Self {
        Self::with_capacity(inner, encoder, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates a buffered text writer with a requested byte buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Byte writer that receives encoded bytes.
    /// - `encoder`: Streaming character-to-byte transcoder.
    /// - `capacity`: Requested byte buffer capacity.
    ///
    /// # Returns
    ///
    /// Returns a buffered text writer. The byte buffer is raised to the maximum
    /// output needed for one input character when that can be computed.
    #[must_use]
    pub fn with_capacity(inner: W, encoder: E, capacity: usize) -> Self {
        let min_output_capacity = encoder.max_output_len(1).unwrap_or(1).max(1);
        let capacity = capacity.max(min_output_capacity);
        Self {
            output: TranscodeEncodeOutput::with_capacity(inner, capacity),
            encoder,
            line_ending: LineEnding::Lf,
            char_buffer: Vec::with_capacity(DEFAULT_CHAR_CHUNK_CAPACITY),
            finished: false,
        }
    }

    /// Sets the line ending for this writer.
    ///
    /// # Parameters
    ///
    /// - `line_ending`: Line ending used by [`TextWrite::write_line`].
    ///
    /// # Returns
    ///
    /// Returns this writer with the configured line ending.
    #[must_use]
    pub const fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Returns a shared reference to the wrapped byte writer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped writer. Pending bytes may still be buffered.
    #[must_use]
    pub const fn inner(&self) -> &W {
        self.output.inner()
    }

    /// Returns a mutable reference to the wrapped byte writer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped writer. Flush first if it must observe all prior
    /// text writes.
    pub fn inner_mut(&mut self) -> &mut W {
        self.output.inner_mut()
    }

    /// Returns the configured line ending.
    #[must_use]
    pub const fn configured_line_ending(&self) -> LineEnding {
        self.line_ending
    }

    /// Returns an error if this writer has already been finished.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::InvalidInput`] after [`Self::finish`]
    /// succeeds.
    fn ensure_open(&self) -> io::Result<()> {
        if self.finished {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot write after buffered text writer has been finished",
            ));
        }
        Ok(())
    }
}

impl<W, E> BufferedWriter<W, E>
where
    W: Write,
    E: Transcoder<char, u8>,
    E::Error: StdError + Send + Sync + 'static,
{
    /// Encodes a character slice into the shared output buffer.
    ///
    /// # Parameters
    ///
    /// - `chars`: Characters to encode.
    ///
    /// # Errors
    ///
    /// Returns encoding errors or I/O errors from the wrapped writer.
    fn encode_chars(&mut self, chars: &[char]) -> io::Result<()> {
        let written = unsafe {
            self.output.transcode_from(
                &mut self.encoder,
                &mut encode_error_to_io,
                chars,
                0,
                chars.len(),
            )?
        };
        if written != chars.len() {
            return Err(io::Error::other(
                "text encoder did not consume the complete character input",
            ));
        }
        Ok(())
    }

    /// Flushes one internal string-writing character chunk.
    ///
    /// # Errors
    ///
    /// Returns encoding errors or I/O errors from the wrapped writer.
    fn flush_char_chunk(&mut self) -> io::Result<()> {
        if self.char_buffer.is_empty() {
            return Ok(());
        }
        let chars = std::mem::take(&mut self.char_buffer);
        let result = self.encode_chars(chars.as_slice());
        self.char_buffer = chars;
        self.char_buffer.clear();
        result
    }

    /// Finishes codec-owned output and flushes pending bytes.
    ///
    /// # Errors
    ///
    /// Returns encoding finalization errors or I/O errors from the wrapped
    /// writer. After a successful finish, later write calls return
    /// [`io::ErrorKind::InvalidInput`].
    pub fn finish(&mut self) -> io::Result<()> {
        if !self.finished {
            self.output
                .finish(&mut self.encoder, &mut encode_error_to_io)?;
            self.finished = true;
        }
        self.output.flush()
    }

    /// Returns the wrapped byte writer after finishing and flushing.
    ///
    /// # Returns
    ///
    /// Returns the underlying byte writer after pending bytes reach it.
    ///
    /// # Errors
    ///
    /// Returns encoding finalization or I/O errors.
    pub fn into_inner(mut self) -> io::Result<W> {
        self.finish()?;
        let (inner, _) = self.output.into_parts();
        Ok(inner)
    }
}

impl<W, E> TextWrite for BufferedWriter<W, E>
where
    W: Write,
    E: Transcoder<char, u8>,
    E::Error: StdError + Send + Sync + 'static,
{
    type Error = io::Error;

    #[inline]
    fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    #[inline]
    fn write_char(&mut self, ch: char) -> Result<(), Self::Error> {
        self.write_chars(&[ch])
    }

    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        self.ensure_open()?;
        if chars.is_empty() {
            return Ok(());
        }
        self.encode_chars(chars)
    }

    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        self.ensure_open()?;
        if text.is_empty() {
            return Ok(());
        }
        for ch in text.chars() {
            self.char_buffer.push(ch);
            if self.char_buffer.len() == DEFAULT_CHAR_CHUNK_CAPACITY {
                self.flush_char_chunk()?;
            }
        }
        self.flush_char_chunk()
    }

    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.write_str(line)?;
        self.write_str(self.line_ending.as_str())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.output.flush()
    }
}

/// Converts encoder errors into I/O errors.
fn encode_error_to_io<E>(error: E) -> io::Error
where
    E: StdError + Send + Sync + 'static,
{
    io::Error::new(io::ErrorKind::InvalidInput, error)
}
