// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair

use std::io;

use qubit_codec::{
    AsyncTranscodeDecodeInput, AsyncTranscodeDecodeStep, TranscodeStatus, Transcoder,
};
use qubit_codec_text::{CharsetCodec, CharsetDecodePolicy, CharsetDecoder};
use qubit_io::{AsyncInput, Buffer};

use crate::CodingErrorPolicy;
use crate::io_error::{capacity_error_to_io, decode_error_to_io, text_append_limit_error};

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
///
/// # Examples
///
/// ```no_run
/// use qubit_codec_text::Utf8Codec;
/// use qubit_io::AsyncInput;
/// use qubit_io_text::{AsyncCharsetTextReader, CodingErrorPolicy};
///
/// async fn read_all<I>(input: I) -> std::io::Result<String>
/// where
///     I: AsyncInput<Item = u8> + Unpin,
/// {
///     let mut reader = AsyncCharsetTextReader::new(
///         input,
///         Utf8Codec,
///         CodingErrorPolicy::Strict,
///     );
///     let mut text = String::new();
///     reader.read_to_string_async(&mut text).await?;
///     Ok(text)
/// }
/// ```
#[derive(Debug)]
pub struct AsyncCharsetTextReader<I, C>
where
    I: AsyncInput<Item = u8>,
    C: CharsetCodec<Unit = u8>,
{
    input: AsyncTranscodeDecodeInput<I>,
    decoder: CharsetDecoder<C>,
    policy: CodingErrorPolicy,
    chars: Vec<char>,
    char_position: usize,
    char_limit: usize,
    started: bool,
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
        Self::new_with_buffer_capacity(input, codec, policy, DEFAULT_BUFFER_CAPACITY)
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
        let decoder = CharsetDecoder::with_policy(codec, policy.decode_policy());
        Self {
            input: AsyncTranscodeDecodeInput::with_capacity(input, capacity),
            decoder,
            policy,
            chars: vec!['\0'; capacity],
            char_position: 0,
            char_limit: 0,
            started: false,
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
        self.input.inner()
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
        self.input.inner_mut()
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
    /// Returns the wrapped input, unread encoded bytes, decoder, and decoded
    /// characters not yet returned by this reader, in that order.
    #[must_use = "all returned reader state must be handled"]
    pub fn into_parts(self) -> (I, Buffer<u8>, CharsetDecoder<C>, Vec<char>) {
        let (input, unread) = self.input.into_parts();
        let pending_chars = self.chars[self.char_position..self.char_limit].to_vec();
        (input, unread, self.decoder, pending_chars)
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
        self.input.unread_len()
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

    /// Applies the configured policy to an incomplete encoded EOF tail.
    fn handle_incomplete_eof(&mut self) -> io::Result<bool> {
        match self.policy {
            CodingErrorPolicy::Strict => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "incomplete charset input at EOF",
            )),
            CodingErrorPolicy::Replace => {
                let unread = self.input.unread_len();
                self.input.consume(unread);
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

    /// Finalizes decoding or applies the configured incomplete-EOF policy.
    ///
    /// # Returns
    ///
    /// Returns whether finalization made a decoded character available.
    ///
    /// # Errors
    ///
    /// Returns decoder finalization or strict incomplete-EOF errors.
    fn finish_at_eof(&mut self) -> io::Result<bool> {
        if self.unread_byte_count() == 0 {
            self.finish_decoder()
        } else {
            self.handle_incomplete_eof()
        }
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
        loop {
            self.clear_chars();
            let char_capacity = self.chars.len();
            match self
                .input
                .transcode_async(
                    &mut self.decoder,
                    &mut decode_error_to_io,
                    self.chars.as_mut_slice(),
                    0,
                    char_capacity,
                )
                .await?
            {
                AsyncTranscodeDecodeStep::EndOfInput => {
                    return self.finish_at_eof();
                }
                AsyncTranscodeDecodeStep::Progress(progress) => {
                    self.char_position = 0;
                    self.char_limit = progress.written();
                    if self.has_buffered_chars() {
                        return Ok(true);
                    }
                    match progress.status() {
                        TranscodeStatus::Complete => continue,
                        TranscodeStatus::NeedInput { required, .. } => {
                            if !self.input.fill_until_async(required.get()).await? {
                                return self.finish_at_eof();
                            }
                        }
                        TranscodeStatus::NeedOutput { required, .. } => {
                            if self.chars.len() < required.get() {
                                self.chars.resize(required.get(), '\0');
                            }
                        }
                    }
                }
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
    ///
    /// # Cancellation safety
    ///
    /// Cancelling this future retains fetched bytes and decoder state in this
    /// reader. Retrying on the same reader does not discard a partial encoded
    /// character.
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
    ///
    /// # Cancellation safety
    ///
    /// Cancelling this future retains reader state, but `output` can already
    /// contain a successfully decoded prefix. Resume on the same reader and
    /// do not append that prefix a second time.
    pub async fn read_chars_async(
        &mut self,
        output: &mut Vec<char>,
        max: usize,
    ) -> io::Result<usize> {
        let mut count = 0;
        while count < max && self.fill_chars_async().await? {
            let available = self.char_limit - self.char_position;
            let take = available.min(max - count);
            let end = self.char_position + take;
            output.extend_from_slice(&self.chars[self.char_position..end]);
            self.char_position = end;
            count += take;
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
    ///
    /// # Cancellation safety
    ///
    /// Cancelling this future retains reader state, but `output` can already
    /// contain a successfully decoded prefix. Resume on the same reader and
    /// do not append that prefix a second time.
    pub async fn read_to_string_async(&mut self, output: &mut String) -> io::Result<usize> {
        let mut count = 0;
        while self.fill_chars_async().await? {
            let chars = &self.chars[self.char_position..self.char_limit];
            output.extend(chars.iter());
            count += chars.len();
            self.char_position = self.char_limit;
        }
        Ok(count)
    }

    /// Asynchronously appends decoded text while enforcing a UTF-8 byte limit.
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
    /// Returns input and decoding errors. Returns
    /// [`io::ErrorKind::InvalidData`] and restores `output` to its original
    /// length when the next decoded character would exceed
    /// `max_append_len`. Previously consumed characters remain consumed by
    /// the reader.
    ///
    /// # Cancellation safety
    ///
    /// Cancelling this future retains reader state, but `output` can already
    /// contain a successfully decoded prefix. Resume on the same reader and
    /// do not append that prefix a second time.
    pub async fn read_to_string_limited_async(
        &mut self,
        output: &mut String,
        max_append_len: usize,
    ) -> io::Result<usize> {
        let initial_len = output.len();
        let mut count = 0;
        while self.fill_chars_async().await? {
            while self.char_position < self.char_limit {
                let ch = self.chars[self.char_position];
                let appended_len = output.len() - initial_len;
                if ch.len_utf8() > max_append_len.saturating_sub(appended_len) {
                    output.truncate(initial_len);
                    return Err(text_append_limit_error(max_append_len));
                }
                output.push(ch);
                self.char_position += 1;
                count += 1;
            }
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
    ///
    /// # Cancellation safety
    ///
    /// Cancelling this future retains reader state, but `output` can already
    /// contain a successfully decoded line prefix. Resume on the same reader
    /// and do not append that prefix a second time.
    pub async fn read_line_async(&mut self, output: &mut String) -> io::Result<bool> {
        let mut read = false;
        while self.fill_chars_async().await? {
            let chars = &self.chars[self.char_position..self.char_limit];
            let take = chars
                .iter()
                .position(|ch| *ch == '\n')
                .map_or(chars.len(), |index| index + 1);
            output.extend(chars[..take].iter());
            self.char_position += take;
            read = true;
            if chars.get(take - 1) == Some(&'\n') {
                return Ok(true);
            }
        }
        Ok(read)
    }
}
