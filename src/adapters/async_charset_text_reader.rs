// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
// qubit-style: allow coverage-cfg

#[cfg(coverage)]
use std::cell::Cell;
use std::io;

use qubit_codec::{
    AsyncTranscodeDecodeInput, AsyncTranscodeDecodeStep, TranscodeStatus, Transcoder,
};
use qubit_codec_text::{CharsetCodec, CharsetDecodePolicy, CharsetDecoder};
use qubit_io::AsyncInput;

use crate::TextReaderParts;
use crate::io_error::{capacity_error_to_io, decode_error_to_io};
use crate::line_ending_set::{LineEndingSet, append_limited_char};
use crate::{AsyncTextLineRead, AsyncTextRead, LineEnding};

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
/// use qubit_codec_text::{CharsetDecodePolicy, Utf8Codec};
/// use qubit_io::AsyncInput;
/// use qubit_io_text::AsyncCharsetTextReader;
///
/// async fn read_all<I>(input: I) -> std::io::Result<String>
/// where
///     I: AsyncInput<Item = u8> + Unpin,
/// {
///     let mut reader = AsyncCharsetTextReader::new(
///         input,
///         Utf8Codec,
///         CharsetDecodePolicy::report(),
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
    chars: Vec<char>,
    char_position: usize,
    char_limit: usize,
    line_endings: LineEndingSet,
    pending_char: Option<char>,
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
    pub fn new(input: I, codec: C, policy: CharsetDecodePolicy) -> Self {
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
        policy: CharsetDecodePolicy,
        buffer_capacity: usize,
    ) -> Self {
        let capacity = buffer_capacity.max(MIN_TEXT_BUFFER_CAPACITY);
        let decoder = CharsetDecoder::with_policy(codec, policy);
        Self {
            input: AsyncTranscodeDecodeInput::with_capacity(input, capacity),
            decoder,
            chars: vec!['\0'; capacity],
            char_position: 0,
            char_limit: 0,
            line_endings: LineEndingSet::ALL,
            pending_char: None,
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

    /// Sets the line endings recognized by asynchronous line reads.
    ///
    /// # Parameters
    /// - `line_endings`: Accepted line-ending sequences.
    ///
    /// # Returns
    /// This reader with the requested line-ending configuration.
    #[must_use]
    pub const fn with_line_endings(mut self, line_endings: LineEndingSet) -> Self {
        self.line_endings = line_endings;
        self
    }

    /// Returns the line endings recognized by this reader.
    #[must_use]
    pub const fn line_endings(&self) -> LineEndingSet {
        self.line_endings
    }

    /// Forces one EOF status for coverage-only branch tests.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_force_next_eof_status(mode: u8) {
        COVERAGE_FORCE_EOF_STATUS.with(|state| state.set(mode));
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
    /// Returns named components for the input, unread encoded bytes, decoder,
    /// and decoded characters not yet returned by this reader.
    #[must_use = "all returned reader state must be handled"]
    pub fn into_parts(self) -> TextReaderParts<I, CharsetDecoder<C>> {
        let (input, unread) = self.input.into_parts();
        let mut pending_chars = Vec::new();
        if let Some(ch) = self.pending_char {
            pending_chars.push(ch);
        }
        pending_chars.extend_from_slice(&self.chars[self.char_position..self.char_limit]);
        TextReaderParts {
            input,
            unread_bytes: unread,
            decoder: self.decoder,
            pending_chars,
        }
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

    /// Decodes retained EOF input according to the decoder's policy.
    ///
    /// # Returns
    ///
    /// Returns whether finalization made a decoded character available.
    ///
    /// # Errors
    ///
    /// Returns EOF decoding or decoder finalization errors.
    fn decode_eof(&mut self) -> io::Result<bool>
    where
        I: Unpin,
    {
        let char_capacity = self.chars.len();
        let progress = self.input.transcode_eof_step(
            &mut self.decoder,
            &mut decode_error_to_io,
            self.chars.as_mut_slice(),
            0,
            char_capacity,
        )?;
        self.char_position = 0;
        self.char_limit = progress.written();
        if self.has_buffered_chars() {
            return Ok(true);
        }
        match coverage_eof_status(progress.status()) {
            TranscodeStatus::NeedOutput { required, .. } => {
                self.chars.resize(required.get(), '\0');
                self.decode_eof()
            }
            TranscodeStatus::Complete => self.finish_decoder(),
            TranscodeStatus::NeedInput { .. } => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "decoder requested more input after EOF",
            )),
        }
    }

    /// Decodes enough retained input to make one character available.
    async fn fill_chars_async(&mut self) -> io::Result<bool>
    where
        I: Unpin,
    {
        if self.finished {
            return Ok(false);
        }
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
                    return self.decode_eof();
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
                                return self.decode_eof();
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

impl<I, C> AsyncTextRead for AsyncCharsetTextReader<I, C>
where
    I: AsyncInput<Item = u8> + Unpin,
    C: CharsetCodec<Unit = u8>,
{
    type Error = io::Error;

    async fn read_char_async(&mut self) -> Result<Option<char>, Self::Error> {
        AsyncCharsetTextReader::read_char_async(self).await
    }

    async fn read_chars_async(
        &mut self,
        output: &mut Vec<char>,
        max: usize,
    ) -> Result<usize, Self::Error> {
        AsyncCharsetTextReader::read_chars_async(self, output, max).await
    }

    async fn read_to_string_async(&mut self, output: &mut String) -> Result<usize, Self::Error> {
        AsyncCharsetTextReader::read_to_string_async(self, output).await
    }
}

impl<I, C> AsyncTextLineRead for AsyncCharsetTextReader<I, C>
where
    I: AsyncInput<Item = u8> + Unpin,
    C: CharsetCodec<Unit = u8>,
{
    async fn read_line_async(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        AsyncCharsetTextReader::read_line_async(self, output).await
    }
}

#[cfg(coverage)]
thread_local! {
    static COVERAGE_FORCE_EOF_STATUS: Cell<u8> = const { Cell::new(0) };
}

#[cfg(coverage)]
fn coverage_eof_status(status: TranscodeStatus) -> TranscodeStatus {
    match COVERAGE_FORCE_EOF_STATUS.with(|state| state.replace(0)) {
        1 => TranscodeStatus::NeedInput {
            required: std::num::NonZeroUsize::MIN,
        },
        2 => TranscodeStatus::NeedOutput {
            required: std::num::NonZeroUsize::MIN,
        },
        _ => status,
    }
}

#[cfg(not(coverage))]
fn coverage_eof_status(status: TranscodeStatus) -> TranscodeStatus {
    status
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
        if let Some(ch) = self.pending_char.take() {
            return Ok(Some(ch));
        }
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
        if max > 0
            && let Some(ch) = self.pending_char.take()
        {
            output.push(ch);
            count = 1;
        }
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
        if let Some(ch) = self.pending_char.take() {
            output.push(ch);
            count = 1;
        }
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
        while let Some(ch) = self.read_char_async().await? {
            if let Err(error) = append_limited_char(output, initial_len, max_append_len, ch) {
                self.pending_char = Some(ch);
                return Err(error);
            }
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
    ///
    /// # Cancellation safety
    ///
    /// Cancelling this future retains reader state, but `output` can already
    /// contain a successfully decoded line prefix. Resume on the same reader
    /// and do not append that prefix a second time.
    pub async fn read_line_async(&mut self, output: &mut String) -> io::Result<bool> {
        self.read_line_async_impl(output, None).await
    }

    /// Reads one line with an optional UTF-8 append limit.
    async fn read_line_async_impl(
        &mut self,
        output: &mut String,
        max_append_len: Option<usize>,
    ) -> io::Result<bool> {
        let initial_len = output.len();
        let mut read = false;
        while let Some(ch) = self.read_char_async().await? {
            read = true;
            if ch == '\r' && self.line_endings.contains(LineEnding::CrLf) {
                if let Some(max_append_len) = max_append_len
                    && ch.len_utf8() > max_append_len.saturating_sub(output.len() - initial_len)
                {
                    output.truncate(initial_len);
                    self.pending_char = Some(ch);
                    let error = crate::io_error::text_append_limit_error(max_append_len);
                    return self.discard_line_after_limit_async().await.and(Err(error));
                }

                // Keep CR pending while the CRLF lookahead may suspend. If
                // this future is cancelled, the next call can resume from CR.
                self.pending_char = Some('\r');
                let next = self.read_char_from_buffer_async().await?;
                self.pending_char = None;
                match next {
                    Some('\n') => {
                        // CR was checked before the await and output did not
                        // change during lookahead.
                        output.push('\r');
                        if let Err(error) =
                            self.append_line_char(output, initial_len, max_append_len, '\n')
                        {
                            self.pending_char = Some('\n');
                            return self.discard_line_after_limit_async().await.and(Err(error));
                        }
                        return Ok(true);
                    }
                    Some(next) => {
                        output.push('\r');
                        self.pending_char = Some(next);
                        if self.line_endings.contains(LineEnding::Cr) {
                            return Ok(true);
                        }
                        continue;
                    }
                    None => {
                        output.push('\r');
                        return Ok(true);
                    }
                }
            }

            if let Err(error) = self.append_line_char(output, initial_len, max_append_len, ch) {
                self.pending_char = Some(ch);
                return self.discard_line_after_limit_async().await.and(Err(error));
            }
            if ch == '\n' && self.line_endings.contains(LineEnding::Lf) {
                return Ok(true);
            }
            if ch == '\r' && self.line_endings.contains(LineEnding::Cr) {
                return Ok(true);
            }
        }
        Ok(read)
    }

    /// Asynchronously appends one decoded line while enforcing a UTF-8 byte
    /// limit.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination string. The line is appended to existing
    ///   content.
    /// - `max_append_len`: Maximum UTF-8 byte length appended by this call.
    ///
    /// # Returns
    ///
    /// Returns `true` when a line or final unterminated line was read, or
    /// `false` at EOF with no text appended.
    ///
    /// # Errors
    ///
    /// Returns input or decoding errors, or [`io::ErrorKind::InvalidData`]
    /// when the decoded line exceeds `max_append_len`. On overflow, `output`
    /// is restored to its original length and the remainder of the oversized
    /// line is consumed through its configured line ending.
    ///
    /// # Cancellation safety
    ///
    /// Cancelling this future retains reader state, but `output` can already
    /// contain a successfully decoded prefix. Resume on the same reader and
    /// do not append that prefix a second time.
    pub async fn read_line_limited_async(
        &mut self,
        output: &mut String,
        max_append_len: usize,
    ) -> io::Result<bool> {
        self.read_line_async_impl(output, Some(max_append_len))
            .await
    }

    /// Reads one decoded character without consulting `pending_char`.
    async fn read_char_from_buffer_async(&mut self) -> io::Result<Option<char>> {
        if !self.fill_chars_async().await? {
            return Ok(None);
        }
        let ch = self.chars[self.char_position];
        self.char_position += 1;
        Ok(Some(ch))
    }

    /// Discards the remainder of the current line after a bounded read fails.
    async fn discard_line_after_limit_async(&mut self) -> io::Result<()> {
        loop {
            let Some(ch) = self.read_char_async().await? else {
                return Ok(());
            };
            if ch == '\n' && self.line_endings.contains(LineEnding::Lf) {
                return Ok(());
            }
            if ch == '\r' {
                if self.line_endings.contains(LineEnding::CrLf) {
                    self.pending_char = Some('\r');
                    let next = self.read_char_from_buffer_async().await?;
                    self.pending_char = None;
                    match next {
                        Some('\n') => return Ok(()),
                        Some(next) => {
                            self.pending_char = Some(next);
                            if self.line_endings.contains(LineEnding::Cr) {
                                return Ok(());
                            }
                            continue;
                        }
                        None => return Ok(()),
                    }
                }
                if self.line_endings.contains(LineEnding::Cr) {
                    return Ok(());
                }
            }
        }
    }

    fn append_line_char(
        &mut self,
        output: &mut String,
        initial_len: usize,
        max_append_len: Option<usize>,
        ch: char,
    ) -> io::Result<()> {
        if let Some(max_append_len) = max_append_len {
            append_limited_char(output, initial_len, max_append_len, ch)
        } else {
            output.push(ch);
            Ok(())
        }
    }
}
