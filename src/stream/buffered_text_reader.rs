// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::error::Error as StdError;
use std::io;

use qubit_codec::CapacityError;
use qubit_codec::TranscodeDecodeInput;
use qubit_codec::TranscodeDecoder;
use qubit_codec::TranscodeStatus;
use qubit_io::ByteInput;
use qubit_utils::UncheckedSlice;
use qubit_utils::nonzero;

use crate::TextLineReader;
use crate::TextReader;
use crate::TextReaderParts;
use crate::io_error::capacity_error_to_io as shared_capacity_error_to_io;
use crate::io_error::decode_error_to_io as shared_decode_error_to_io;
use crate::line_ending_set::LineEndingSet;
use crate::line_ending_set::append_limited_char;
use crate::line_ending_set::discard_line_with;
use crate::line_ending_set::read_line_with;

/// Default byte buffer capacity used by buffered text readers.
const DEFAULT_BUFFER_CAPACITY: usize = 8 * 1024;

/// Minimum byte buffer capacity for built-in byte-oriented text codecs.
const MIN_TEXT_BUFFER_CAPACITY: usize = 4;

/// Buffered text reader driven by a byte-to-character transcoder.
///
/// This type owns a byte reader and a streaming decoder. Encoded bytes are
/// buffered by [`qubit_codec::TranscodeDecodeInput`], while decoded
/// characters are exposed through [`TextReader`].
/// Decoder reset is started lazily on the first read attempt, and any values
/// emitted by reset are returned before decoded source characters.
/// Buffered decoding may inspect bytes beyond the requested character or line.
/// Under a reporting policy, malformed data later in the same decoding window
/// may cause an error before an earlier valid character or complete line is
/// returned. How much text has been delivered when an error occurs can depend
/// on input chunking. An error does not guarantee delivery of every valid
/// prefix, input rollback, or a generally safe retry; decoded characters not
/// registered as readable are distinct from text already appended to the
/// caller's output.
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
///
/// use qubit_codec_text::{CharsetDecodePolicy, CharsetDecoder, Utf8Codec};
/// use qubit_io_text::{BufferedTextReader, TextReader};
///
/// let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
/// let mut reader = BufferedTextReader::new(
///     Cursor::new("hello".as_bytes().to_vec()),
///     decoder,
/// );
/// let mut text = String::new();
/// reader.read_to_string(&mut text)?;
/// assert_eq!("hello", text);
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct BufferedTextReader<R, D>
where
    R: ByteInput,
{
    /// Owns the byte reader and preserves encoded bytes not yet consumed by the
    /// decoder.
    input: TranscodeDecodeInput<R>,
    /// Converts the buffered encoded bytes into Unicode scalar values.
    decoder: D,
    /// Stores decoded characters awaiting delivery to the caller.
    chars: Vec<char>,
    /// Index of the next decoded character to return.
    char_position: usize,
    /// Exclusive end of the currently buffered decoded characters.
    char_limit: usize,
    /// Line-ending sequences recognized by line-reading methods.
    line_endings: LineEndingSet,
    /// Holds a character consumed ahead while resolving a line-ending sequence
    /// or limit.
    pending_char: Option<char>,
    /// Stores the original limit and line-ending policy while an oversized line
    /// is being discarded.
    rejected_line: Option<(usize, LineEndingSet)>,
    /// Records whether decoder reset has completed.
    started: bool,
    /// Records whether EOF decoding has started.
    eof_started: bool,
    /// Records whether EOF decoding has completed.
    eof_complete: bool,
    /// Records whether decoder finalization has completed.
    finished: bool,
}

impl<R, D> BufferedTextReader<R, D>
where
    R: ByteInput,
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
            rejected_line: None,
            started: false,
            eof_started: false,
            eof_complete: false,
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
    #[inline]
    pub const fn inner(&self) -> &R {
        self.input.inner()
    }

    /// Sets the line endings recognized by [`TextLineReader::read_line`].
    ///
    /// # Parameters
    /// - `line_endings`: Accepted line-ending sequences.
    ///
    /// # Returns
    /// This reader with the requested line-ending configuration.
    #[must_use]
    #[inline]
    pub const fn with_line_endings(mut self, line_endings: LineEndingSet) -> Self {
        self.line_endings = line_endings;
        self
    }

    /// Returns the line endings recognized by this reader.
    #[must_use]
    #[inline]
    pub const fn line_endings(&self) -> LineEndingSet {
        self.line_endings
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
    pub fn into_parts(self) -> TextReaderParts<R, D> {
        let (inner, unread) = self.input.into_parts();
        let mut pending_chars = Vec::new();
        if let Some(ch) = self.pending_char {
            pending_chars.push(ch);
        }
        pending_chars.extend_from_slice(&self.chars[self.char_position..self.char_limit]);
        TextReaderParts {
            input: inner,
            unread_bytes: unread,
            decoder: self.decoder,
            pending_chars,
        }
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

impl<R, D> BufferedTextReader<R, D>
where
    R: ByteInput,
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
        let required = self.decoder.max_reset_output_len().map_err(capacity_error_to_io)?;
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
            .max(nonzero(1).get());
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

    /// Discards the remainder of the current line after a bounded read fails.
    fn discard_line_after_limit(&mut self, line_endings: LineEndingSet) -> io::Result<()> {
        let mut pending = self.pending_char.take();
        let result = discard_line_with(line_endings, &mut pending, || self.read_char());
        self.pending_char = pending;
        result
    }

    /// Resumes discarding a rejected line and reports its original limit error
    /// once.
    fn finish_rejected_line(&mut self) -> io::Result<io::Error> {
        let (limit, line_endings) = self
            .rejected_line
            .expect("a rejected line must exist before discard resumes");
        self.discard_line_after_limit(line_endings)?;
        self.rejected_line = None;
        Ok(crate::io_error::text_append_limit_error(limit))
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
        self.eof_started = true;
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
        if progress.status() == TranscodeStatus::Complete {
            self.eof_complete = true;
        }
        if self.has_buffered_chars() {
            return Ok(true);
        }
        match progress.status() {
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
        if self.has_buffered_chars() {
            return Ok(true);
        }
        if self.finished {
            return Ok(false);
        }
        self.ensure_started()?;
        if self.has_buffered_chars() {
            return Ok(true);
        }
        if self.eof_complete {
            return self.finish_decoder();
        }
        if self.eof_started {
            return self.decode_eof();
        }
        loop {
            self.clear_chars();
            let capacity = self.chars.len();
            let progress = self.input.transcode_step(
                &mut self.decoder,
                &mut decode_error_to_io,
                self.chars.as_mut_slice(),
                0,
                capacity,
            )?;
            let Some(progress) = progress else {
                return self.decode_eof();
            };
            let written = progress.written();
            self.char_position = 0;
            self.char_limit = written;
            if self.has_buffered_chars() {
                return Ok(true);
            }
            match progress.status() {
                TranscodeStatus::NeedOutput { required, .. } => {
                    self.chars.resize(required.get(), '\0');
                    continue;
                }
                TranscodeStatus::Complete => continue,
                TranscodeStatus::NeedInput { required, .. } => {
                    let fill_result = self.input.fill_until(required.get());
                    if fill_result? {
                        continue;
                    }
                    break;
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
    pub fn read_to_string_limited(&mut self, output: &mut String, max_append_len: usize) -> io::Result<usize> {
        let initial_len = output.len();
        let mut count = 0;
        while let Some(ch) = self.read_char()? {
            if let Err(error) = append_limited_char(output, initial_len, max_append_len, ch) {
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
    /// length when the line exceeds `max_append_len`. The remainder of the
    /// oversized line is consumed through its configured line ending before
    /// the error is returned. If discarding hits an input error, retry either
    /// line-reading method on this reader: it resumes with the original limit
    /// and line-ending policy, then reports the original `InvalidData` once
    /// before reading the next line. Character-reading methods and
    /// [`Self::into_parts`] do not resume this line-level recovery.
    pub fn read_line_limited(&mut self, output: &mut String, max_append_len: usize) -> io::Result<bool> {
        if self.rejected_line.is_some() {
            return Err(self.finish_rejected_line()?);
        }
        let initial_len = output.len();
        let mut read = false;
        while let Some(ch) = self.read_char()? {
            if let Err(error) = append_limited_char(output, initial_len, max_append_len, ch) {
                self.pending_char = Some(ch);
                self.rejected_line = Some((max_append_len, self.line_endings));
                return match self.finish_rejected_line() {
                    Err(discard_error) => Err(discard_error),
                    Ok(_) => Err(error),
                };
            }
            read = true;
            if ch == '\n' && self.line_endings.contains(crate::LineEnding::Lf) {
                return Ok(true);
            }
            if ch == '\r' {
                if self.line_endings.contains(crate::LineEnding::CrLf) {
                    let next = match self.read_char() {
                        Ok(next) => next,
                        Err(error) => {
                            // CR was appended immediately before this lookahead.
                            output.truncate(output.len() - '\r'.len_utf8());
                            self.pending_char = Some('\r');
                            return Err(error);
                        }
                    };
                    match next {
                        Some('\n') => {
                            append_limited_char(output, initial_len, max_append_len, '\n')?;
                            return Ok(true);
                        }
                        Some(next) => {
                            if self.line_endings.contains(crate::LineEnding::Cr) {
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

impl<R, D> TextReader for BufferedTextReader<R, D>
where
    R: ByteInput,
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
        // SAFETY: `fill_chars()` returned true, so `char_position < char_limit`.
        // The decode driver validates transcode output counts and checks reset
        // and finish bounds; the corresponding storage is available before each
        // call. Thus `char_limit <= chars.len()`, and this index is in bounds.
        let ch = unsafe { UncheckedSlice::read(self.chars.as_slice(), self.char_position) };
        self.char_position += 1;
        Ok(Some(ch))
    }

    fn read_chars(&mut self, output: &mut Vec<char>, max: usize) -> Result<usize, Self::Error> {
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

    fn read_to_string(&mut self, output: &mut String) -> Result<usize, Self::Error> {
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

impl<R, D> TextLineReader for BufferedTextReader<R, D>
where
    R: ByteInput,
    D: TranscodeDecoder<Input = u8, Output = char>,
    D::Error: StdError + Send + Sync + 'static,
    D::DecodeError: Send + Sync + 'static,
{
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        if self.rejected_line.is_some() {
            return Err(self.finish_rejected_line()?);
        }
        let line_endings = self.line_endings;
        let mut pending_char = self.pending_char.take();
        let result = read_line_with(line_endings, output, &mut pending_char, || self.read_char());
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
