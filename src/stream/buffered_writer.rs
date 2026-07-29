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

use qubit_codec::{
    CapacityError,
    TranscodeEncodeOutput,
    TranscodeEncoder,
    nz,
};
use qubit_io::{
    Buffer,
    Output,
};

use crate::{
    LineEnding,
    TextWrite,
    io_error::{
        capacity_error_to_io as shared_capacity_error_to_io,
        encode_error_to_io as shared_encode_error_to_io,
    },
};

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
#[derive(Debug)]
pub struct BufferedWriter<W, E>
where
    W: Output<Item = u8>,
{
    output: TranscodeEncodeOutput<W>,
    encoder: E,
    line_ending: LineEnding,
    char_buffer: Vec<char>,
    started: bool,
    finished: bool,
}

impl<W, E> BufferedWriter<W, E>
where
    W: Output<Item = u8>,
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
        let one = nz(1).get();
        let min_output_capacity = encoder
            .max_transcode_output_len(one)
            .unwrap_or(one)
            .max(one);
        let capacity = capacity.max(min_output_capacity);
        Self {
            output: TranscodeEncodeOutput::with_capacity(inner, capacity),
            encoder,
            line_ending: LineEnding::Lf,
            char_buffer: Vec::with_capacity(DEFAULT_CHAR_CHUNK_CAPACITY),
            started: false,
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

    /// Returns the wrapped byte writer and every encoded byte still pending.
    ///
    /// This method performs no I/O and does not finalize the encoder. Call
    /// [`Self::finish`] first for normal completion; after a successful finish,
    /// the returned buffer is empty. Calling this method before finishing
    /// explicitly abandons encoder lifecycle output that has not yet been
    /// emitted while transferring already encoded pending bytes to the caller.
    ///
    /// # Returns
    ///
    /// Returns the wrapped byte writer and pending encoded bytes in logical
    /// write order.
    #[must_use = "the returned inner writer and pending buffer must be handled"]
    #[inline(always)]
    pub fn into_parts(self) -> (W, Buffer<u8>) {
        self.output.into_parts()
    }

    /// Makes the next reset-capacity check fail in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_fail_next_reset_reserve() {
        COVERAGE_FAIL_NEXT_RESET_RESERVE.with(|state| state.set(true));
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
    W: Output<Item = u8>,
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
        self.output.transcode_from(
            &mut self.encoder,
            &mut encode_error_to_io,
            chars,
            0,
            chars.len(),
        )?;
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
        let required = self
            .encoder
            .max_reset_output_len()
            .map_err(capacity_error_to_io)?;
        let reserve_result = self.output.ensure_spare_capacity(required);
        #[cfg(coverage)]
        let reserve_result = if coverage_take_reset_reserve_failure() {
            Err(io::Error::from(io::ErrorKind::OutOfMemory))
        } else {
            reserve_result
        };
        reserve_result?;
        let (units, output_index, available) =
            self.output.spare_raw_parts_mut();
        assert!(
            available >= required,
            "insufficient reset capacity reserved in spare output buffer",
        );
        let written = self
            .encoder
            .reset(units, output_index)
            .map_err(encode_error_to_io)?;
        assert!(written <= required, "reset wrote beyond its bound");
        unsafe {
            // SAFETY: Reset output is bounded by the capacity reserved above.
            self.output.advance(written);
        }
        self.started = true;
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
            self.ensure_started()?;
            self.output
                .finish(&mut self.encoder, &mut encode_error_to_io)?;
            self.finished = true;
        }
        self.output.flush()
    }
}

impl<W, E> TextWrite for BufferedWriter<W, E>
where
    W: Output<Item = u8>,
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

/// Converts encoder errors at the buffered-writer boundary.
fn encode_error_to_io<E>(error: E) -> io::Error
where
    E: StdError + Send + Sync + 'static,
{
    shared_encode_error_to_io(error)
}

/// Converts capacity errors at the buffered-writer boundary.
fn capacity_error_to_io(error: CapacityError) -> io::Error {
    shared_capacity_error_to_io(error)
}

#[cfg(coverage)]
thread_local! {
    static COVERAGE_FAIL_NEXT_RESET_RESERVE: Cell<bool> = const { Cell::new(false) };
}

/// Returns and clears the synthetic reset-reserve failure request.
#[cfg(coverage)]
fn coverage_take_reset_reserve_failure() -> bool {
    COVERAGE_FAIL_NEXT_RESET_RESERVE.with(|state| state.replace(false))
}
