// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io;

use qubit_codec_text::{
    CharsetDecodePolicy,
    CharsetDecoder,
    Utf8Codec,
};
use qubit_io::{
    Buffer,
    Input,
};

use crate::{
    CharsetTextReader,
    TextLineRead,
    TextRead,
};

/// Streaming UTF-8 text reader over a Qubit byte input.
///
/// This is the strict UTF-8 convenience form of [`CharsetTextReader`]. It
/// shares the same buffering and decoder state machine instead of introducing
/// a separate `std::io::Read`-based stream boundary.
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
///
/// use qubit_io_text::{TextRead, Utf8TextReader};
///
/// let mut reader = Utf8TextReader::new(Cursor::new("hello".as_bytes().to_vec()));
/// let mut text = String::new();
/// reader.read_to_string(&mut text)?;
/// assert_eq!("hello", text);
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct Utf8TextReader<I>
where
    I: Input<Item = u8>,
{
    reader: CharsetTextReader<I, Utf8Codec>,
}

impl<I> Utf8TextReader<I>
where
    I: Input<Item = u8>,
{
    /// Creates a strict UTF-8 text reader with the default buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `input`: Qubit byte input to decode lazily.
    ///
    /// # Returns
    ///
    /// Returns a streaming reader. Construction does not read from `input`.
    #[inline]
    #[must_use]
    pub fn new(input: I) -> Self {
        Self {
            reader: CharsetTextReader::new(
                input,
                Utf8Codec,
                CharsetDecodePolicy::report(),
            ),
        }
    }

    /// Creates a strict UTF-8 reader with a requested byte buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `input`: Qubit byte input to decode lazily.
    /// - `buffer_capacity`: Requested internal byte buffer capacity.
    ///
    /// # Returns
    ///
    /// Returns a streaming reader. The generic buffered layer raises a
    /// too-small capacity enough to retain an incomplete UTF-8 scalar.
    #[inline]
    #[must_use]
    pub fn with_capacity(input: I, buffer_capacity: usize) -> Self {
        Self {
            reader: CharsetTextReader::new_with_buffer_capacity(
                input,
                Utf8Codec,
                CharsetDecodePolicy::report(),
                buffer_capacity,
            ),
        }
    }

    /// Returns a shared reference to the wrapped byte input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input. It may already be positioned beyond bytes
    /// retained in the reader's internal buffer.
    #[inline(always)]
    #[must_use]
    pub const fn input(&self) -> &I {
        self.reader.input()
    }

    /// Consumes this reader and returns every component that may contain
    /// unread logical input.
    ///
    /// The returned characters must be processed before continuing the text
    /// stream. The returned byte buffer must be consumed before reading from
    /// the wrapped input because the wrapped input can be physically ahead of
    /// that buffer. This method does not finish the decoder.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input, unread UTF-8 bytes, decoder, and decoded
    /// characters not yet returned by this reader, in that order.
    #[must_use = "all returned reader state must be handled"]
    #[inline]
    pub fn into_parts(
        self,
    ) -> (I, Buffer<u8>, CharsetDecoder<Utf8Codec>, Vec<char>) {
        self.reader.into_parts()
    }

    /// Appends decoded UTF-8 text while enforcing a UTF-8 byte limit.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination string to append to.
    /// - `max_append_len`: Maximum UTF-8 byte length appended by this call.
    ///
    /// # Returns
    ///
    /// Returns the number of Unicode scalar values appended to `output`.
    ///
    /// # Errors
    ///
    /// Returns input or UTF-8 errors. Returns [`io::ErrorKind::InvalidData`]
    /// and restores `output` to its original length when the decoded text
    /// exceeds `max_append_len`. Previously consumed characters remain
    /// consumed by the reader.
    pub fn read_to_string_limited(
        &mut self,
        output: &mut String,
        max_append_len: usize,
    ) -> io::Result<usize> {
        self.reader.read_to_string_limited(output, max_append_len)
    }

    /// Appends one decoded UTF-8 line while enforcing a byte limit.
    ///
    /// The limit applies only to text appended by this call. On overflow the
    /// destination is restored, while already consumed decoded characters
    /// remain consumed.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination string to append to.
    /// - `max_append_len`: Maximum UTF-8 byte length appended by this call.
    ///
    /// # Returns
    ///
    /// Returns `true` when a line or final unterminated line was read, or
    /// `false` at EOF with no text appended.
    ///
    /// # Errors
    ///
    /// Returns input or UTF-8 errors, or [`io::ErrorKind::InvalidData`]
    /// when the decoded line exceeds `max_append_len`.
    pub fn read_line_limited(
        &mut self,
        output: &mut String,
        max_append_len: usize,
    ) -> io::Result<bool> {
        self.reader.read_line_limited(output, max_append_len)
    }
}

impl<I> TextRead for Utf8TextReader<I>
where
    I: Input<Item = u8>,
{
    type Error = io::Error;

    #[inline]
    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        self.reader.read_char()
    }

    #[inline]
    fn read_chars(
        &mut self,
        output: &mut Vec<char>,
        max: usize,
    ) -> Result<usize, Self::Error> {
        self.reader.read_chars(output, max)
    }

    #[inline]
    fn read_to_string(
        &mut self,
        output: &mut String,
    ) -> Result<usize, Self::Error> {
        self.reader.read_to_string(output)
    }
}

impl<I> TextLineRead for Utf8TextReader<I>
where
    I: Input<Item = u8>,
{
    #[inline]
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        self.reader.read_line(output)
    }
}
