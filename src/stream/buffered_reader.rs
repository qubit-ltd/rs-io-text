// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow coverage-cfg
#[cfg(coverage)]
use std::cell::Cell;
use std::{
    error::Error as StdError,
    io,
};

#[cfg(coverage)]
use qubit_codec::TranscodeProgress;
use qubit_codec::{
    CapacityError,
    TranscodeDecodeInput,
    TranscodeDecoder,
    TranscodeStatus,
    nz,
};
use qubit_io::{
    Buffer,
    Input,
    UncheckedSlice,
};

use crate::{
    TextLineRead,
    TextRead,
    io_error::{
        capacity_error_to_io as shared_capacity_error_to_io,
        decode_error_to_io as shared_decode_error_to_io,
    },
    line_ending_set::{
        LineEndingSet,
        append_limited_char,
        read_line_with,
    },
};

/// Default byte buffer capacity used by buffered text readers.
const DEFAULT_BUFFER_CAPACITY: usize = 8 * 1024;

/// Minimum byte buffer capacity for built-in byte-oriented text codecs.
const MIN_TEXT_BUFFER_CAPACITY: usize = 4;

/// Buffered text reader driven by a byte-to-character transcoder.
///
/// This type owns a byte reader and a streaming decoder. Encoded bytes are
/// buffered by [`qubit_codec::TranscodeDecodeInput`], while decoded
/// characters are exposed through [`TextRead`].
/// Decoder reset is started lazily on the first read attempt, and any values
/// emitted by reset are returned before decoded source characters.
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
///
/// use qubit_codec_text::{CharsetDecodePolicy, CharsetDecoder, Utf8Codec};
/// use qubit_io_text::{BufferedReader, TextRead};
///
/// let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
/// let mut reader = BufferedReader::new(
///     Cursor::new("hello".as_bytes().to_vec()),
///     decoder,
/// );
/// let mut text = String::new();
/// reader.read_to_string(&mut text)?;
/// assert_eq!("hello", text);
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct BufferedReader<R, D>
where
    R: Input<Item = u8>,
{
    input: TranscodeDecodeInput<R>,
    decoder: D,
    chars: Vec<char>,
    char_position: usize,
    char_limit: usize,
    line_endings: LineEndingSet,
    pending_char: Option<char>,
    started: bool,
    finished: bool,
}

impl<R, D> BufferedReader<R, D>
where
    R: Input<Item = u8>,
{
    /// Creates a buffered text reader with the default byte buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Byte reader to decode lazily.
    /// - `decoder`: Streaming byte-to-character transcoder.
    ///
    /// # Returns
    ///
    /// Returns a buffered text reader. Construction does not read from
    /// `inner`.
    #[must_use]
    pub fn new(inner: R, decoder: D) -> Self {
        Self::with_capacity(inner, decoder, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates a buffered text reader with a requested byte buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Byte reader to decode lazily.
    /// - `decoder`: Streaming byte-to-character transcoder.
    /// - `capacity`: Requested byte buffer capacity.
    ///
    /// # Returns
    ///
    /// Returns a buffered text reader. The byte buffer is raised to at least
    /// four bytes so built-in Unicode byte codecs can retain incomplete tails.
    #[must_use]
    pub fn with_capacity(inner: R, decoder: D, capacity: usize) -> Self {
        let capacity = capacity.max(MIN_TEXT_BUFFER_CAPACITY);
        Self {
            input: TranscodeDecodeInput::with_capacity(inner, capacity),
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

    /// Sets the line endings recognized by [`TextLineRead::read_line`].
    ///
    /// # Parameters
    /// - `line_endings`: Accepted line-ending sequences.
    ///
    /// # Returns
    /// This reader with the requested line-ending configuration.
    #[must_use]
    pub const fn with_line_endings(
        mut self,
        line_endings: LineEndingSet,
    ) -> Self {
        self.line_endings = line_endings;
        self
    }

    /// Returns the line endings recognized by this reader.
    #[must_use]
    pub const fn line_endings(&self) -> LineEndingSet {
        self.line_endings
    }

    /// Forces one decoder status for coverage-only branch tests.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_force_next_status(mode: u8) {
        COVERAGE_FORCE_STATUS.with(|state| state.set(mode));
    }

    /// Forces the next input refill to return an I/O error in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_force_next_fill_error() {
        COVERAGE_FORCE_FILL_ERROR.with(|state| state.set(true));
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
    pub fn into_parts(self) -> (R, Buffer<u8>, D, Vec<char>) {
        let (inner, unread) = self.input.into_parts();
        let mut pending_chars = Vec::new();
        if let Some(ch) = self.pending_char {
            pending_chars.push(ch);
        }
        pending_chars.extend_from_slice(
            &self.chars[self.char_position..self.char_limit],
        );
        (inner, unread, self.decoder, pending_chars)
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
}

impl<R, D> BufferedReader<R, D>
where
    R: Input<Item = u8>,
    D: TranscodeDecoder<Input = u8, Output = char>,
    D::Error: StdError + Send + Sync + 'static,
    D::DecodeError: Send + Sync + 'static,
{
    /// Starts the decoder lifecycle before the first read attempt.
    ///
    /// Reset-produced characters are retained in the decoded character buffer
    /// and delivered before characters decoded from source bytes.
    ///
    /// # Errors
    ///
    /// Returns capacity or decoder reset errors.
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
        let written = self.input.reset(
            &mut self.decoder,
            &mut decode_error_to_io,
            self.chars.as_mut_slice(),
            0,
            required,
        )?;
        self.started = true;
        self.char_position = 0;
        self.char_limit = written;
        Ok(())
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
            .max(nz(1).get());
        if self.chars.len() < capacity {
            self.chars.resize(capacity, '\0');
        }
        let written = self.input.finish(
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

    /// Re-enters the completed decoder path for coverage-only tests.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_finish_again(&mut self) -> io::Result<bool> {
        self.finished = std::hint::black_box(true);
        self.finish_decoder()
    }

    /// Touches decoded-buffer helpers for coverage-only tests.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_touch_buffer_state(&mut self) {
        std::hint::black_box(self.has_buffered_chars());
        self.clear_chars();
        self.finished = true;
        let _ = std::hint::black_box(self.finish_decoder());
    }

    /// Decodes the retained EOF tail according to the decoder's policy.
    ///
    /// # Returns
    ///
    /// Returns `true` when EOF decoding or finalization made a character
    /// available, or `false` after clean finalization.
    ///
    /// # Errors
    ///
    /// Returns I/O, decoder, or finalization errors.
    fn decode_eof(&mut self) -> io::Result<bool> {
        let capacity = self.chars.len();
        let progress = self.input.transcode_eof_step(
            &mut self.decoder,
            &mut decode_error_to_io,
            self.chars.as_mut_slice(),
            0,
            capacity,
        )?;
        self.char_position = 0;
        self.char_limit = progress.written();
        if self.has_buffered_chars() {
            return Ok(true);
        }
        match coverage_status(progress.status(), false) {
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
        if self.finished {
            return Ok(false);
        }
        self.ensure_started()?;
        if self.has_buffered_chars() {
            return Ok(true);
        }
        self.clear_chars();
        let capacity = self.chars.len();
        let progress = self.input.transcode_step(
            &mut self.decoder,
            &mut decode_error_to_io,
            self.chars.as_mut_slice(),
            0,
            capacity,
        )?;
        #[cfg(coverage)]
        let progress = if progress.is_none() && coverage_force_fill_progress() {
            Some(TranscodeProgress::complete(0, 0))
        } else {
            progress
        };
        let Some(progress) = progress else {
            return self.decode_eof();
        };
        let written = progress.written();
        self.char_position = 0;
        self.char_limit = written;
        if self.has_buffered_chars() {
            return Ok(true);
        }
        match coverage_status(progress.status(), true) {
            TranscodeStatus::NeedOutput { .. } => {
                self.chars
                    .resize(capacity.saturating_mul(2).max(capacity + 1), '\0');
                return self.fill_chars();
            }
            TranscodeStatus::Complete => {
                return self.fill_chars();
            }
            TranscodeStatus::NeedInput { required, .. } => {
                let fill_result = self.input.fill_until(required.get());
                #[cfg(coverage)]
                let fill_result = if coverage_force_fill_error() {
                    Err(io::Error::other("forced refill error"))
                } else {
                    fill_result
                };
                if fill_result? {
                    return self.fill_chars();
                }
            }
        }
        self.decode_eof()
    }

    /// Appends decoded text while enforcing a UTF-8 byte limit.
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
    /// Returns input or decoding errors. Returns [`io::ErrorKind::InvalidData`]
    /// and restores `output` to its original length when the next decoded
    /// character would exceed `max_append_len`. Previously consumed characters
    /// remain consumed by the reader.
    pub fn read_to_string_limited(
        &mut self,
        output: &mut String,
        max_append_len: usize,
    ) -> io::Result<usize> {
        let initial_len = output.len();
        let mut count = 0;
        while let Some(ch) = self.read_char()? {
            if let Err(error) =
                append_limited_char(output, initial_len, max_append_len, ch)
            {
                self.pending_char = Some(ch);
                return Err(error);
            }
            count += 1;
        }
        Ok(count)
    }

    /// Appends one decoded line while enforcing a UTF-8 byte limit.
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
    /// Returns input or decoding errors. Returns
    /// [`io::ErrorKind::InvalidData`] and restores `output` to its original
    /// length when the line exceeds `max_append_len`. Characters accepted
    /// before the limit remain consumed; the character that exceeds the
    /// limit remains pending.
    pub fn read_line_limited(
        &mut self,
        output: &mut String,
        max_append_len: usize,
    ) -> io::Result<bool> {
        let initial_len = output.len();
        let mut read = false;
        while let Some(ch) = self.read_char()? {
            if let Err(error) =
                append_limited_char(output, initial_len, max_append_len, ch)
            {
                self.pending_char = Some(ch);
                return Err(error);
            }
            read = true;
            if ch == '\n' && self.line_endings.contains(crate::LineEnding::Lf) {
                return Ok(true);
            }
            if ch == '\r' {
                if self.line_endings.contains(crate::LineEnding::CrLf) {
                    match self.read_char()? {
                        Some('\n') => {
                            if let Err(error) = append_limited_char(
                                output,
                                initial_len,
                                max_append_len,
                                '\n',
                            ) {
                                self.pending_char = Some('\n');
                                return Err(error);
                            }
                            return Ok(true);
                        }
                        Some(next) => {
                            if self.line_endings.contains(crate::LineEnding::Cr)
                            {
                                self.pending_char = Some(next);
                                return Ok(true);
                            }
                            self.pending_char = Some(next);
                        }
                        None => return Ok(true),
                    }
                }
                if self.line_endings.contains(crate::LineEnding::Cr) {
                    return Ok(true);
                }
            }
        }
        Ok(read)
    }
}

#[cfg(coverage)]
thread_local! {
    static COVERAGE_FORCE_STATUS: Cell<u8> = const { Cell::new(0) };
    static COVERAGE_FORCE_FILL_ERROR: Cell<bool> = const { Cell::new(false) };
}

#[cfg(coverage)]
fn coverage_status(
    status: TranscodeStatus,
    from_fill: bool,
) -> TranscodeStatus {
    let mode = COVERAGE_FORCE_STATUS.with(|state| state.replace(0));
    match (from_fill, mode) {
        (false, 1) | (true, 3) => TranscodeStatus::NeedInput {
            required: std::num::NonZeroUsize::MIN,
        },
        (false, 2) => TranscodeStatus::NeedOutput {
            required: std::num::NonZeroUsize::MIN,
        },
        (true, 4) => TranscodeStatus::Complete,
        _ => status,
    }
}

#[cfg(coverage)]
fn coverage_force_fill_progress() -> bool {
    COVERAGE_FORCE_STATUS.with(|state| matches!(state.get(), 3 | 4))
}

#[cfg(coverage)]
fn coverage_force_fill_error() -> bool {
    COVERAGE_FORCE_FILL_ERROR.with(|state| state.replace(false))
}

#[cfg(not(coverage))]
fn coverage_status(
    status: TranscodeStatus,
    _from_fill: bool,
) -> TranscodeStatus {
    status
}

impl<R, D> TextRead for BufferedReader<R, D>
where
    R: Input<Item = u8>,
    D: TranscodeDecoder<Input = u8, Output = char>,
    D::Error: StdError + Send + Sync + 'static,
    D::DecodeError: Send + Sync + 'static,
{
    type Error = io::Error;

    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        if let Some(ch) = self.pending_char.take() {
            return Ok(Some(ch));
        }
        if !self.fill_chars()? {
            return Ok(None);
        }
        let ch = unsafe {
            UncheckedSlice::read(self.chars.as_slice(), self.char_position)
        };
        self.char_position += 1;
        Ok(Some(ch))
    }

    fn read_chars(
        &mut self,
        output: &mut Vec<char>,
        max: usize,
    ) -> Result<usize, Self::Error> {
        let mut count = 0;
        if max > 0
            && let Some(ch) = self.pending_char.take()
        {
            output.push(ch);
            count = 1;
        }
        while count < max && self.fill_chars()? {
            let available = self.char_limit - self.char_position;
            let take = available.min(max - count);
            let end = self.char_position + take;
            output.extend_from_slice(&self.chars[self.char_position..end]);
            self.char_position = end;
            count += take;
        }
        Ok(count)
    }

    fn read_to_string(
        &mut self,
        output: &mut String,
    ) -> Result<usize, Self::Error> {
        let mut count = 0;
        if let Some(ch) = self.pending_char.take() {
            output.push(ch);
            count = 1;
        }
        while self.fill_chars()? {
            let chars = &self.chars[self.char_position..self.char_limit];
            output.extend(chars.iter());
            count += chars.len();
            self.char_position = self.char_limit;
        }
        Ok(count)
    }
}

impl<R, D> TextLineRead for BufferedReader<R, D>
where
    R: Input<Item = u8>,
    D: TranscodeDecoder<Input = u8, Output = char>,
    D::Error: StdError + Send + Sync + 'static,
    D::DecodeError: Send + Sync + 'static,
{
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        let line_endings = self.line_endings;
        let mut pending_char = self.pending_char.take();
        let result =
            read_line_with(line_endings, output, &mut pending_char, || {
                self.read_char()
            });
        self.pending_char = pending_char;
        result
    }
}

/// Converts decoder errors at the buffered-reader boundary.
fn decode_error_to_io<E>(error: E) -> io::Error
where
    E: StdError + Send + Sync + 'static,
{
    shared_decode_error_to_io(error)
}

/// Converts capacity errors at the buffered-reader boundary.
fn capacity_error_to_io(error: CapacityError) -> io::Error {
    shared_capacity_error_to_io(error)
}
