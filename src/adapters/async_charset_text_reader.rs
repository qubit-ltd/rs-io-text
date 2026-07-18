// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;

use qubit_codec::{
    TranscodeStatus,
    Transcoder,
};
use qubit_codec_text::{
    CharsetCodec,
    CharsetDecodePolicy,
    CharsetDecoder,
};
use qubit_io::{
    AsyncInput,
    AsyncInputExt,
};

use crate::CodingErrorPolicy;
use crate::io_error::{
    capacity_error_to_io,
    decode_error_to_io,
};

/// Default encoded-byte capacity used by asynchronous charset readers.
const DEFAULT_BUFFER_CAPACITY: usize = 8 * 1024;

/// Minimum capacity needed to preserve built-in Unicode codec tails.
const MIN_TEXT_BUFFER_CAPACITY: usize = 4;

/// Asynchronously decodes a byte input with a charset codec.
///
/// The codec itself remains synchronous, deterministic state. Only fetching
/// more encoded bytes is asynchronous. Bytes already obtained from the input
/// are committed to this reader before another suspension point, so cancelling
/// a read future does not discard a partially received encoded character.
#[derive(Debug)]
pub struct AsyncCharsetTextReader<I, C>
where
    I: AsyncInput<Item = u8>,
    C: CharsetCodec<Unit = u8>,
{
    input: I,
    decoder: CharsetDecoder<C>,
    policy: CodingErrorPolicy,
    bytes: Vec<u8>,
    byte_position: usize,
    byte_limit: usize,
    chars: Vec<char>,
    char_position: usize,
    char_limit: usize,
    started: bool,
    eof: bool,
    finished: bool,
}

impl<I, C> AsyncCharsetTextReader<I, C>
where
    I: AsyncInput<Item = u8>,
    C: CharsetCodec<Unit = u8>,
{
    /// Creates an asynchronous charset reader with the default buffer size.
    ///
    /// # Parameters
    ///
    /// - `input`: Asynchronous encoded-byte input.
    /// - `codec`: Charset codec used to decode the input.
    /// - `policy`: Malformed and incomplete input policy.
    ///
    /// # Returns
    ///
    /// Returns a reader whose construction performs no input operation.
    #[must_use]
    pub fn new(input: I, codec: C, policy: CodingErrorPolicy) -> Self {
        Self::new_with_buffer_capacity(
            input,
            codec,
            policy,
            DEFAULT_BUFFER_CAPACITY,
        )
    }

    /// Creates an asynchronous charset reader with a requested buffer size.
    ///
    /// # Parameters
    ///
    /// - `input`: Asynchronous encoded-byte input.
    /// - `codec`: Charset codec used to decode the input.
    /// - `policy`: Malformed and incomplete input policy.
    /// - `buffer_capacity`: Requested internal byte capacity.
    ///
    /// # Returns
    ///
    /// Returns a reader whose byte buffer is large enough to retain built-in
    /// Unicode codec tails.
    #[must_use]
    pub fn new_with_buffer_capacity(
        input: I,
        codec: C,
        policy: CodingErrorPolicy,
        buffer_capacity: usize,
    ) -> Self {
        let capacity = buffer_capacity.max(MIN_TEXT_BUFFER_CAPACITY);
        let decoder =
            CharsetDecoder::with_policy(codec, policy.decode_policy());
        Self {
            input,
            decoder,
            policy,
            bytes: vec![0; capacity],
            byte_position: 0,
            byte_limit: 0,
            chars: vec!['\0'; capacity],
            char_position: 0,
            char_limit: 0,
            started: false,
            eof: false,
            finished: false,
        }
    }

    /// Returns a shared reference to the asynchronous byte input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input. It may be positioned beyond encoded bytes
    /// retained by this reader.
    #[must_use]
    pub const fn input(&self) -> &I {
        &self.input
    }

    /// Returns a mutable reference to the asynchronous byte input.
    ///
    /// Mutating the input directly can invalidate the logical stream position
    /// represented by retained bytes.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input.
    pub fn input_mut(&mut self) -> &mut I {
        &mut self.input
    }

    /// Consumes this reader and returns its asynchronous byte input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input. Buffered encoded bytes and decoded
    /// characters are discarded.
    #[must_use]
    pub fn into_input(self) -> I {
        self.input
    }

    /// Returns whether at least one decoded character is buffered.
    #[inline]
    fn has_buffered_chars(&self) -> bool {
        self.char_position < self.char_limit
    }

    /// Clears the decoded-character window.
    #[inline]
    fn clear_chars(&mut self) {
        self.char_position = 0;
        self.char_limit = 0;
    }

    /// Returns the number of encoded bytes not yet consumed by the decoder.
    #[inline]
    fn unread_byte_count(&self) -> usize {
        self.byte_limit - self.byte_position
    }

    /// Moves unread encoded bytes to the start of their storage.
    fn compact_bytes(&mut self) {
        if self.byte_position == 0 {
            return;
        }
        self.bytes
            .copy_within(self.byte_position..self.byte_limit, 0);
        self.byte_limit -= self.byte_position;
        self.byte_position = 0;
    }

    /// Ensures the encoded-byte storage can expose `required` unread bytes.
    fn ensure_byte_capacity(&mut self, required: usize) {
        if self.bytes.len() >= required {
            return;
        }
        let grown = self
            .bytes
            .len()
            .saturating_mul(2)
            .max(required)
            .max(MIN_TEXT_BUFFER_CAPACITY);
        self.bytes.resize(grown, 0);
    }

    /// Starts the decoder lifecycle before the first input operation.
    fn ensure_started(&mut self) -> io::Result<()> {
        if self.started {
            return Ok(());
        }
        let required = self
            .decoder
            .max_reset_output_len()
            .map_err(capacity_error_to_io)?;
        if self.chars.len() < required {
            self.chars.resize(required, '\0');
        }
        let written = self
            .decoder
            .reset(self.chars.as_mut_slice(), 0)
            .map_err(decode_error_to_io)?;
        assert!(written <= required, "decoder reset exceeded its bound");
        self.started = true;
        self.char_position = 0;
        self.char_limit = written;
        Ok(())
    }

    /// Reads one additional encoded-byte chunk into retained storage.
    async fn read_more_async(&mut self) -> io::Result<()>
    where
        I: Unpin,
    {
        self.compact_bytes();
        // Callers either cleared an empty window or grew the compacted window
        // to the decoder's strictly larger input requirement.
        debug_assert!(self.byte_limit < self.bytes.len());
        loop {
            let read = {
                let input = &mut self.input;
                let destination = &mut self.bytes[self.byte_limit..];
                input.read_async(destination).await
            };
            match read {
                Ok(0) => {
                    self.eof = true;
                    return Ok(());
                }
                Ok(read) => {
                    self.byte_limit += read;
                    return Ok(());
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
    }

    /// Applies the configured policy to an incomplete encoded EOF tail.
    fn handle_incomplete_eof(&mut self) -> io::Result<bool> {
        match self.policy {
            CodingErrorPolicy::Strict => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "incomplete charset input at EOF",
            )),
            CodingErrorPolicy::Replace => {
                self.byte_position = self.byte_limit;
                debug_assert!(!self.chars.is_empty());
                self.chars[0] = CharsetDecodePolicy::DEFAULT_REPLACEMENT;
                self.char_position = 0;
                self.char_limit = 1;
                Ok(true)
            }
        }
    }

    /// Finishes decoder-owned output after all encoded input is consumed.
    fn finish_decoder(&mut self) -> io::Result<bool> {
        if self.finished {
            return Ok(false);
        }
        let required = self
            .decoder
            .max_finish_output_len()
            .map_err(capacity_error_to_io)?;
        if self.chars.len() < required {
            self.chars.resize(required, '\0');
        }
        let written = self
            .decoder
            .finish(self.chars.as_mut_slice(), 0)
            .map_err(decode_error_to_io)?;
        assert!(written <= required, "decoder finish exceeded its bound");
        self.finished = true;
        self.char_position = 0;
        self.char_limit = written;
        Ok(written > 0)
    }

    /// Decodes enough retained input to make one character available.
    async fn fill_chars_async(&mut self) -> io::Result<bool>
    where
        I: Unpin,
    {
        self.ensure_started()?;
        if self.has_buffered_chars() {
            return Ok(true);
        }
        self.clear_chars();

        loop {
            if self.unread_byte_count() == 0 {
                self.byte_position = 0;
                self.byte_limit = 0;
                if self.eof {
                    return self.finish_decoder();
                }
                self.read_more_async().await?;
                continue;
            }

            let input_index = self.byte_position;
            let progress = self
                .decoder
                .transcode(
                    &self.bytes[..self.byte_limit],
                    input_index,
                    self.chars.as_mut_slice(),
                    0,
                )
                .map_err(decode_error_to_io)?;
            // `CharsetDecoder` constructs validated progress from the codec
            // engine. Invalid counts are rejected at that boundary.
            self.byte_position += progress.read();
            self.char_position = 0;
            self.char_limit = progress.written();
            if self.has_buffered_chars() {
                return Ok(true);
            }

            let TranscodeStatus::NeedInput { required, .. } = progress.status()
            else {
                unreachable!(
                    "charset decoder without output must request more input",
                );
            };
            self.compact_bytes();
            self.ensure_byte_capacity(required.get());
            self.read_more_async().await?;
            if self.eof && self.unread_byte_count() < required.get() {
                return self.handle_incomplete_eof();
            }
        }
    }
}

impl<I, C> AsyncCharsetTextReader<I, C>
where
    I: AsyncInput<Item = u8> + Unpin,
    C: CharsetCodec<Unit = u8>,
{
    /// Asynchronously reads the next decoded character.
    ///
    /// # Returns
    ///
    /// Returns `Some(character)` when one is available, or `None` at EOF.
    ///
    /// # Errors
    ///
    /// Returns input, decoder, or incomplete-EOF errors.
    pub async fn read_char_async(&mut self) -> io::Result<Option<char>> {
        if !self.fill_chars_async().await? {
            return Ok(None);
        }
        let ch = self.chars[self.char_position];
        self.char_position += 1;
        Ok(Some(ch))
    }

    /// Asynchronously appends up to `max` decoded characters.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination character vector.
    /// - `max`: Maximum number of characters to append.
    ///
    /// # Returns
    ///
    /// Returns the number of appended characters.
    ///
    /// # Errors
    ///
    /// Returns input and decoding errors.
    pub async fn read_chars_async(
        &mut self,
        output: &mut Vec<char>,
        max: usize,
    ) -> io::Result<usize> {
        let mut count = 0;
        while count < max {
            match self.read_char_async().await? {
                Some(ch) => {
                    output.push(ch);
                    count += 1;
                }
                None => break,
            }
        }
        Ok(count)
    }

    /// Asynchronously appends all remaining decoded text to a string.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination string.
    ///
    /// # Returns
    ///
    /// Returns the number of appended Unicode scalar values.
    ///
    /// # Errors
    ///
    /// Returns input and decoding errors.
    pub async fn read_to_string_async(
        &mut self,
        output: &mut String,
    ) -> io::Result<usize> {
        let mut count = 0;
        while let Some(ch) = self.read_char_async().await? {
            output.push(ch);
            count += 1;
        }
        Ok(count)
    }

    /// Asynchronously appends one line, including a trailing newline.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination string.
    ///
    /// # Returns
    ///
    /// Returns `true` when at least one character was appended, or `false`
    /// when the reader was already at EOF.
    ///
    /// # Errors
    ///
    /// Returns input and decoding errors.
    pub async fn read_line_async(
        &mut self,
        output: &mut String,
    ) -> io::Result<bool> {
        let mut read = false;
        while let Some(ch) = self.read_char_async().await? {
            output.push(ch);
            read = true;
            if ch == '\n' {
                break;
            }
        }
        Ok(read)
    }
}
