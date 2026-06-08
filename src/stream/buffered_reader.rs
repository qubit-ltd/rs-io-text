// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::{
    error::Error as StdError,
    io::{
        self,
        Read,
    },
};

use qubit_codec::{
    BufferedDecodeInput,
    BufferedTranscoder,
};
use qubit_codec_text::CharsetDecodePolicy;

use crate::{
    CodingErrorPolicy,
    TextLineRead,
    TextRead,
};

/// Default byte buffer capacity used by buffered text readers.
const DEFAULT_BUFFER_CAPACITY: usize = 8 * 1024;

/// Minimum byte buffer capacity for built-in byte-oriented text codecs.
const MIN_TEXT_BUFFER_CAPACITY: usize = 4;

/// Buffered text reader driven by a byte-to-character transcoder.
///
/// This type owns a byte reader and a streaming decoder. Encoded bytes are
/// buffered by [`qubit_codec::BufferedDecodeInput`], while decoded
/// characters are exposed through [`TextRead`].
#[derive(Debug)]
pub struct BufferedReader<R, D>
where
    R: Read,
{
    input: BufferedDecodeInput<R>,
    decoder: D,
    policy: CodingErrorPolicy,
    chars: Vec<char>,
    char_position: usize,
    char_limit: usize,
    finished: bool,
}

impl<R, D> BufferedReader<R, D>
where
    R: Read,
{
    /// Creates a buffered text reader with the default byte buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Byte reader to decode lazily.
    /// - `decoder`: Streaming byte-to-character transcoder.
    /// - `policy`: EOF incomplete-tail handling policy.
    ///
    /// # Returns
    ///
    /// Returns a buffered text reader. Construction does not read from
    /// `inner`.
    #[must_use]
    pub fn new(inner: R, decoder: D, policy: CodingErrorPolicy) -> Self {
        Self::with_capacity(inner, decoder, policy, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates a buffered text reader with a requested byte buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Byte reader to decode lazily.
    /// - `decoder`: Streaming byte-to-character transcoder.
    /// - `policy`: EOF incomplete-tail handling policy.
    /// - `capacity`: Requested byte buffer capacity.
    ///
    /// # Returns
    ///
    /// Returns a buffered text reader. The byte buffer is raised to at least
    /// four bytes so built-in Unicode byte codecs can retain incomplete tails.
    #[must_use]
    pub fn with_capacity(
        inner: R,
        decoder: D,
        policy: CodingErrorPolicy,
        capacity: usize,
    ) -> Self {
        let capacity = capacity.max(MIN_TEXT_BUFFER_CAPACITY);
        Self {
            input: BufferedDecodeInput::with_capacity(inner, capacity),
            decoder,
            policy,
            chars: vec!['\0'; capacity],
            char_position: 0,
            char_limit: 0,
            finished: false,
        }
    }

    /// Returns a shared reference to the wrapped byte reader.
    ///
    /// # Returns
    ///
    /// Returns the wrapped reader. It may already be positioned beyond bytes
    /// retained in this reader's internal buffer.
    #[must_use]
    pub const fn inner(&self) -> &R {
        self.input.inner()
    }

    /// Returns a mutable reference to the wrapped byte reader.
    ///
    /// # Returns
    ///
    /// Returns the wrapped reader. Mutating it directly can invalidate
    /// buffered bytes.
    pub fn inner_mut(&mut self) -> &mut R {
        self.input.inner_mut()
    }

    /// Consumes this reader and returns the wrapped byte reader.
    ///
    /// # Returns
    ///
    /// Returns the underlying reader. Buffered encoded bytes and decoded
    /// characters are discarded.
    #[must_use]
    pub fn into_inner(self) -> R {
        let (inner, _) = self.input.into_parts();
        inner
    }

    /// Returns whether decoded characters are currently buffered.
    ///
    /// # Returns
    ///
    /// Returns `true` if `read_char` can return without decoding more input.
    #[inline]
    fn has_buffered_chars(&self) -> bool {
        self.char_position < self.char_limit
    }

    /// Clears the decoded character buffer.
    #[inline]
    fn clear_chars(&mut self) {
        self.char_position = 0;
        self.char_limit = 0;
    }

    /// Consumes all currently buffered encoded input.
    fn consume_all_input(&mut self) {
        self.input.consume_available();
    }
}

impl<R, D> BufferedReader<R, D>
where
    R: Read,
    D: BufferedTranscoder<u8, char>,
    D::Error: StdError + Send + Sync + 'static,
{
    /// Handles an incomplete encoded tail after EOF.
    ///
    /// # Returns
    ///
    /// Returns `true` when replacement mode emitted one character.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::InvalidData`] in strict mode.
    fn handle_incomplete_eof(&mut self) -> io::Result<bool> {
        match self.policy {
            CodingErrorPolicy::Strict => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "incomplete charset input at EOF",
            )),
            CodingErrorPolicy::Replace => {
                self.consume_all_input();
                if self.chars.is_empty() {
                    self.chars.push(CharsetDecodePolicy::DEFAULT_REPLACEMENT);
                } else {
                    self.chars[0] = CharsetDecodePolicy::DEFAULT_REPLACEMENT;
                }
                self.char_position = 0;
                self.char_limit = 1;
                Ok(true)
            }
        }
    }

    /// Finishes decoder-owned output after EOF.
    ///
    /// # Returns
    ///
    /// Returns `true` when finalization emitted at least one character.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when capacity planning or finalization fails.
    fn finish_decoder(&mut self) -> io::Result<bool> {
        if self.finished {
            return Ok(false);
        }
        let capacity = self
            .decoder
            .max_finish_output_len()
            .map_err(capacity_error_to_io)?
            .max(1);
        if self.chars.len() < capacity {
            self.chars.resize(capacity, '\0');
        }
        let written = self.input.finish_into(
            &mut self.decoder,
            &mut decode_error_to_io,
            self.chars.as_mut_slice(),
            0,
            capacity,
        )?;
        self.finished = true;
        self.char_position = 0;
        self.char_limit = written;
        Ok(written > 0)
    }

    /// Decodes enough input to make at least one character available.
    ///
    /// # Returns
    ///
    /// Returns `true` when a decoded character is available, or `false` at EOF.
    ///
    /// # Errors
    ///
    /// Returns I/O and decoding errors from the wrapped reader or decoder.
    fn fill_chars(&mut self) -> io::Result<bool> {
        if self.has_buffered_chars() {
            return Ok(true);
        }
        self.clear_chars();
        let capacity = self.chars.len();
        let written = self.input.decode_into(
            &mut self.decoder,
            &mut decode_error_to_io,
            self.chars.as_mut_slice(),
            0,
            capacity,
        )?;
        self.char_position = 0;
        self.char_limit = written;
        if self.has_buffered_chars() {
            return Ok(true);
        }
        if self.input.available() == 0 {
            return self.finish_decoder();
        }
        self.handle_incomplete_eof()
    }
}

impl<R, D> TextRead for BufferedReader<R, D>
where
    R: Read,
    D: BufferedTranscoder<u8, char>,
    D::Error: StdError + Send + Sync + 'static,
{
    type Error = io::Error;

    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        if !self.fill_chars()? {
            return Ok(None);
        }
        let ch = self.chars[self.char_position];
        self.char_position += 1;
        Ok(Some(ch))
    }

    fn read_chars(
        &mut self,
        output: &mut Vec<char>,
        max: usize,
    ) -> Result<usize, Self::Error> {
        let mut count = 0;
        while count < max {
            match self.read_char()? {
                Some(ch) => {
                    output.push(ch);
                    count += 1;
                }
                None => break,
            }
        }
        Ok(count)
    }

    fn read_to_string(
        &mut self,
        output: &mut String,
    ) -> Result<usize, Self::Error> {
        let mut count = 0;
        while let Some(ch) = self.read_char()? {
            output.push(ch);
            count += 1;
        }
        Ok(count)
    }
}

impl<R, D> TextLineRead for BufferedReader<R, D>
where
    R: Read,
    D: BufferedTranscoder<u8, char>,
    D::Error: StdError + Send + Sync + 'static,
{
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

/// Converts decoder errors into I/O errors.
fn decode_error_to_io<E>(error: E) -> io::Error
where
    E: StdError + Send + Sync + 'static,
{
    io::Error::new(io::ErrorKind::InvalidData, error)
}

/// Converts codec capacity planning errors into I/O errors.
fn capacity_error_to_io(error: qubit_codec_text::CapacityError) -> io::Error {
    io::Error::new(io::ErrorKind::OutOfMemory, error)
}
