// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::error::Error as StdError;
use std::io;

use qubit_codec::TranscodeDomainError;
use qubit_codec::TranscodeEncodeError;
use qubit_codec::TranscodeEncodeOutput;
use qubit_codec::TranscodeEncoder;
use qubit_io::BufferedOutputRecovery;
use qubit_io::ByteOutput;
use qubit_utils::nonzero;

use crate::LineEnding;
use crate::TextWriter;
use crate::io_error::encode_error_to_io as shared_encode_error_to_io;

/// Default byte buffer capacity used by buffered text writers.
const DEFAULT_BUFFER_CAPACITY: usize = 8 * 1024;

/// Default number of characters converted as one string-writing chunk.
const DEFAULT_CHAR_CHUNK_CAPACITY: usize = 256;

/// Buffered text writer driven by a character-to-byte transcoder.
///
/// This type owns a byte writer and a streaming encoder. Encoded bytes are
/// buffered by [`qubit_codec::TranscodeEncodeOutput`].
/// Encoder reset is started lazily before the first non-empty write, or before
/// finishing an empty stream.
///
/// Main-stage input-character errors from [`TextWriter::write_str`] identify
/// zero-based Unicode scalar (`char`) positions relative to that call, and each
/// call starts counting at zero. [`TextWriter::write_line`] encodes its body
/// and line ending as separate string writes, so a body error is relative to
/// the body and the ending has its own coordinate range. For custom encoders,
/// this writer adjusts the outer transcode input index; an opaque domain error
/// source retains its own index semantics.
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
///
/// use qubit_codec_text::{CharsetEncoder, Utf8Codec};
/// use qubit_io_text::{BufferedTextWriter, TextWriter};
///
/// let encoder = CharsetEncoder::new(Utf8Codec);
/// let mut writer = BufferedTextWriter::new(Cursor::new(Vec::new()), encoder);
/// writer.write_str("hello")?;
/// writer.finish()?;
/// let (output, recovery) = writer.into_parts();
/// let qubit_io::BufferedOutputRecovery::Pending(pending) = recovery else {
///     unreachable!("no inner write panicked");
/// };
/// assert!(pending.readable().is_empty());
/// assert_eq!(b"hello", output.get_ref().as_slice());
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct BufferedTextWriter<W, E>
where
    W: ByteOutput,
{
    /// Owns the byte output and retains encoded bytes awaiting delivery.
    output: TranscodeEncodeOutput<W>,
    /// Retains encoder state across reset, text conversion, and finalization.
    encoder: E,
    /// Terminator appended by line-writing operations.
    line_ending: LineEnding,
    /// Reusable staging storage for one string-writing character chunk.
    /// Ordinary chunk returns clear it; it is not a recovery progress log.
    char_buffer: Vec<char>,
    /// Records that the initial encoder reset returned successfully.
    /// A failed reset is not guaranteed to leave encoder or output state
    /// intact.
    started: bool,
    /// Records successful buffering of encoder-finalization output.
    /// Set before delivery and flush, so pending bytes may remain; retrying
    /// finish completes delivery without finalizing the encoder again.
    finished: bool,
}

impl<W, E> BufferedTextWriter<W, E>
where
    W: ByteOutput,
    E: TranscodeEncoder<Input = char, Output = u8>,
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
        let one = nonzero(1).get();
        let min_output_capacity = encoder.max_transcode_output_len(one).unwrap_or(one).max(one);
        let capacity = capacity.max(min_output_capacity);
        Self {
            output: TranscodeEncodeOutput::with_capacity(inner, capacity),
            encoder,
            line_ending: LineEnding::Lf,
            char_buffer: Vec::new(),
            started: false,
            finished: false,
        }
    }

    /// Sets the line ending for this writer.
    ///
    /// # Parameters
    ///
    /// - `line_ending`: Line ending used by [`TextWriter::write_line`].
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
    #[inline]
    pub const fn inner(&self) -> &W {
        self.output.inner()
    }

    /// Returns the configured line ending.
    #[must_use]
    #[inline]
    pub const fn configured_line_ending(&self) -> LineEnding {
        self.line_ending
    }
    /// Recovers the wrapped output and the status of encoded bytes retained by
    /// this writer.
    ///
    /// This method performs no I/O and does not call [`Self::finish`] or flush
    /// the wrapped output. `Pending` bytes were not reported as written and may
    /// be retried. `Indeterminate` means an inner write panicked and may have
    /// produced an external prefix; do not replay it automatically. Call
    /// `finish` first for normal completion. Recovery does not replace finish:
    /// encoder lifecycle output not yet emitted may not be present in the
    /// returned byte buffer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output and pending or indeterminate encoded bytes in
    /// logical write order.
    #[must_use = "the wrapped output and recovery status must be handled"]
    #[inline(always)]
    pub fn into_parts(self) -> (W, BufferedOutputRecovery<u8>) {
        self.output.into_parts()
    }

    /// Returns an error if this writer has already been finished.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::InvalidInput`] after encoder finalization
    /// succeeds in [`Self::finish`].
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

impl<W, E> BufferedTextWriter<W, E>
where
    W: ByteOutput,
    E: TranscodeEncoder<Input = char, Output = u8>,
    E::Error: StdError + Send + Sync + 'static,
    E::EncodeError: Send + Sync + 'static,
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
        self.ensure_started()?;
        self.output
            .transcode(&mut self.encoder, &mut encode_error_to_io, chars, 0, chars.len())?;
        Ok(())
    }

    /// Appends already encoded UTF-8 bytes to the buffered output.
    ///
    /// This is crate-private because bypassing the encoder is only valid for a
    /// stateless UTF-8 encoder whose input is already a valid Rust string.
    pub(crate) fn write_encoded_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.ensure_open()?;
        if bytes.is_empty() {
            return Ok(());
        }
        self.ensure_started()?;
        let mut offset = 0;
        while offset < bytes.len() {
            let requested = (bytes.len() - offset).min(DEFAULT_BUFFER_CAPACITY);
            self.output.ensure_spare_capacity(requested)?;
            let (units, output_index, available) = self.output.spare_raw_parts_mut();
            let count = requested.min(available);
            units[output_index..output_index + count].copy_from_slice(&bytes[offset..offset + count]);
            // SAFETY: `count` units were initialized in the spare window above.
            unsafe {
                self.output.advance(count);
            }
            offset += count;
        }
        Ok(())
    }

    /// Starts the encoder lifecycle before the first non-empty write.
    ///
    /// # Errors
    ///
    /// Returns capacity, reset, or output errors produced while emitting the
    /// encoder's stream-start units.
    fn ensure_started(&mut self) -> io::Result<()> {
        if self.started {
            return Ok(());
        }
        self.output.reset(&mut self.encoder, &mut encode_error_to_io)?;
        self.started = true;
        Ok(())
    }

    /// Flushes one internal string-writing character chunk and maps its input
    /// positions to the enclosing string call.
    ///
    /// # Errors
    ///
    /// Returns encoding errors or I/O errors from the wrapped writer.
    fn flush_char_chunk<M>(&mut self, base: usize, map_domain: &mut M) -> io::Result<()>
    where
        M: FnMut(E::EncodeError, usize) -> E::EncodeError,
    {
        if self.char_buffer.is_empty() {
            return Ok(());
        }
        let chars = std::mem::take(&mut self.char_buffer);
        let result = (|| {
            self.ensure_started()?;
            let mut map_error = |error| encode_error_to_io(offset_main_encode_error(error, base, map_domain));
            self.output
                .transcode(&mut self.encoder, &mut map_error, &chars, 0, chars.len())?;
            Ok(())
        })();
        self.char_buffer = chars;
        self.char_buffer.clear();
        result
    }

    /// Encodes a string while mapping domain input positions to the string's
    /// coordinate system.
    ///
    /// The mapper is called only for main-phase domain errors and receives the
    /// chunk's base character index. Reset, finish, framework, capacity, and
    /// output errors retain their original meaning.
    ///
    /// # Errors
    ///
    /// Returns the mapped encoding error or an I/O error from the wrapped
    /// output.
    pub(crate) fn write_str_with_domain_offset<M>(&mut self, text: &str, mut map_domain: M) -> io::Result<()>
    where
        M: FnMut(E::EncodeError, usize) -> E::EncodeError,
    {
        self.ensure_open()?;
        if text.is_empty() {
            return Ok(());
        }
        // Reserve the fixed chunk window on first use, then reuse it.
        if self.char_buffer.capacity() < DEFAULT_CHAR_CHUNK_CAPACITY {
            self.char_buffer.reserve_exact(DEFAULT_CHAR_CHUNK_CAPACITY);
        }
        let mut base = 0_usize;
        for ch in text.chars() {
            self.char_buffer.push(ch);
            if self.char_buffer.len() == DEFAULT_CHAR_CHUNK_CAPACITY {
                let count = self.char_buffer.len();
                self.flush_char_chunk(base, &mut map_domain)?;
                base = base.saturating_add(count);
            }
        }
        self.flush_char_chunk(base, &mut map_domain)
    }

    /// Finishes codec-owned output and flushes pending bytes.
    ///
    /// If finalization succeeds but delivery fails, calling this method again
    /// retries delivery without finalizing the encoder a second time.
    /// Once encoder finalization succeeds, later text writes are rejected, even
    /// if this call returns an output delivery or flush error. Retry
    /// `finish` on the same writer to complete delivery without finalizing
    /// the encoder again. Do not replay the original text. An
    /// encoder-finalization error itself does not imply transactional
    /// rollback or a generally safe retry protocol.
    ///
    /// # Errors
    ///
    /// Returns encoding finalization errors or I/O errors from the wrapped
    /// writer. After a successful finish, later write calls return
    /// [`io::ErrorKind::InvalidInput`].
    pub fn finish(&mut self) -> io::Result<()> {
        if !self.finished {
            self.ensure_started()?;
            self.output
                .finish_to_buffer(&mut self.encoder, &mut encode_error_to_io)?;
            self.finished = true;
        }
        self.output.flush()
    }
}

impl<W, E> TextWriter for BufferedTextWriter<W, E>
where
    W: ByteOutput,
    E: TranscodeEncoder<Input = char, Output = u8>,
    E::Error: StdError + Send + Sync + 'static,
    E::EncodeError: Send + Sync + 'static,
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
        self.write_str_with_domain_offset(text, |source, _| source)
    }

    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.write_str(line)?;
        self.write_str(self.line_ending.as_str())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.output.flush()
    }
}

/// Adds a chunk's base character position to errors whose indices describe
/// input characters, leaving lifecycle and framework failures unchanged.
fn offset_main_encode_error<D, M>(
    error: TranscodeEncodeError<D, char>,
    base: usize,
    map_domain: &mut M,
) -> TranscodeEncodeError<D, char>
where
    M: FnMut(D, usize) -> D,
{
    match error {
        TranscodeEncodeError::Unencodable { input_index, value } => TranscodeEncodeError::Unencodable {
            input_index: base.saturating_add(input_index),
            value,
        },
        TranscodeEncodeError::Domain(TranscodeDomainError::Main {
            source,
            input_index,
            input_consumed,
        }) => TranscodeEncodeError::Domain(TranscodeDomainError::Main {
            source: map_domain(source, base),
            input_index: base.saturating_add(input_index),
            input_consumed,
        }),
        other => other,
    }
}

/// Converts encoder errors at the buffered-writer boundary.
fn encode_error_to_io<E>(error: E) -> io::Error
where
    E: StdError + Send + Sync + 'static,
{
    shared_encode_error_to_io(error)
}
