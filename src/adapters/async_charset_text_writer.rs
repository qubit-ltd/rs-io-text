// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;

use qubit_codec::AsyncTranscodeEncodeOutput;
use qubit_codec::TranscodeDomainError;
use qubit_codec::TranscodeEncodeError;
use qubit_codec::Transcoder;
use qubit_codec_text::CharsetCodec;
use qubit_codec_text::CharsetEncodeError;
use qubit_codec_text::CharsetEncodeErrorKind;
use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::CharsetEncoder;
use qubit_io::AsyncByteOutput;
use qubit_io::Buffer;

use crate::AsyncTextWriter;
use crate::LineEnding;
use crate::io_error::encode_error_to_io;

/// Default encoded-byte capacity used by asynchronous charset writers.
const DEFAULT_BUFFER_CAPACITY: usize = 8 * 1024;

/// Number of string characters converted in one bounded chunk.
const DEFAULT_CHAR_CHUNK_CAPACITY: usize = 256;

/// Asynchronously encodes Unicode text into a charset byte output.
///
/// Encoded bytes live in this writer until the wrapped output accepts them.
/// Successful write calls mean text was encoded and accepted into this
/// buffer, not delivered to the output. Capacity pressure may deliver older
/// bytes during a later write; call [`Self::flush_async`] when delivery must be
/// observable and [`Self::finish_async`] to finalize and deliver the stream.
/// Each `write_*_async` operation returns after one encoder step, before any
/// later output poll, and reports exactly how much source it committed. Use
/// the explicit `*_fully_async` methods when a complete source range is
/// required; those convenience loops can commit a prefix before cancellation.
/// After encoder finalization succeeds, every text write rejects input,
/// including empty strings and character slices. This remains true if output
/// delivery later fails; call `finish_async` again to resume delivery.
///
/// # Type Parameters
///
/// - `O`: Asynchronous byte output that receives encoded data.
/// - `C`: Charset codec that converts Unicode characters to bytes.
///
/// # Examples
///
/// ```no_run
/// use qubit_codec_text::{CharsetEncodePolicy, Utf8Codec};
/// use qubit_io::AsyncByteOutput;
/// use qubit_io_text::AsyncCharsetTextWriter;
///
/// async fn write_all<O>(output: O) -> std::io::Result<O>
/// where
///     O: AsyncByteOutput + Unpin,
/// {
///     let mut writer = AsyncCharsetTextWriter::new(
///         output,
///         Utf8Codec,
///         CharsetEncodePolicy::report(),
///     );
///     writer.write_str_fully_async("hello").await?;
///     writer.finish_async().await?;
///     let (output, pending) = writer.into_parts();
///     debug_assert!(pending.is_empty());
///     Ok(output)
/// }
/// ```
#[derive(Debug)]
pub struct AsyncCharsetTextWriter<O, C>
where
    O: AsyncByteOutput,
    C: CharsetCodec<Unit = u8>,
{
    /// Holds encoded bytes until the wrapped output accepts them.
    output: AsyncTranscodeEncodeOutput<O>,
    /// Maintains charset encoding state across write operations.
    encoder: CharsetEncoder<C>,
    /// Supplies the terminator appended by complete line writes.
    line_ending: LineEnding,
    /// Records whether encoder initialization has run.
    started: bool,
    /// Records whether encoder finalization has succeeded.
    finished: bool,
}

impl<O, C> AsyncCharsetTextWriter<O, C>
where
    O: AsyncByteOutput,
    C: CharsetCodec<Unit = u8>,
{
    /// Creates an asynchronous charset writer with the default buffer size.
    ///
    /// # Parameters
    ///
    /// - `output`: Asynchronous byte output.
    /// - `codec`: Charset codec used to encode text.
    /// - `policy`: Unmappable-character policy.
    ///
    /// # Returns
    ///
    /// Returns a writer using LF line endings.
    ///
    /// # Panics
    ///
    /// In replacement mode, panics when the codec cannot encode a supported
    /// replacement character.
    #[must_use]
    pub fn new(output: O, codec: C, policy: CharsetEncodePolicy) -> Self {
        Self::try_new(output, codec, policy).expect("charset encode policy replacement must be encodable")
    }

    /// Creates an asynchronous charset writer and reports an invalid
    /// replacement policy.
    ///
    /// # Parameters
    ///
    /// - `output`: Asynchronous byte output that receives encoded bytes.
    /// - `codec`: Charset codec used to encode text.
    /// - `policy`: Unmappable-character policy.
    ///
    /// # Returns
    ///
    /// Returns a writer whose buffer uses the default capacity and whose line
    /// ending is LF.
    ///
    /// # Errors
    ///
    /// Returns [`CharsetEncodeError`] when replacement mode cannot encode the
    /// configured replacement character.
    pub fn try_new(output: O, codec: C, policy: CharsetEncodePolicy) -> Result<Self, CharsetEncodeError> {
        Self::try_new_with_buffer_capacity(output, codec, policy, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates an asynchronous charset writer with a requested buffer size.
    ///
    /// # Parameters
    ///
    /// - `output`: Asynchronous byte output.
    /// - `codec`: Charset codec used to encode text.
    /// - `policy`: Unmappable-character policy.
    /// - `buffer_capacity`: Requested encoded-byte capacity.
    ///
    /// # Returns
    ///
    /// Returns a writer whose buffer can hold at least one encoded character.
    ///
    /// # Panics
    ///
    /// In replacement mode, panics when the codec cannot encode a supported
    /// replacement character.
    #[must_use]
    pub fn new_with_buffer_capacity(output: O, codec: C, policy: CharsetEncodePolicy, buffer_capacity: usize) -> Self {
        Self::try_new_with_buffer_capacity(output, codec, policy, buffer_capacity)
            .expect("charset encode policy replacement must be encodable")
    }

    /// Creates an asynchronous charset writer with a requested capacity and
    /// reports an invalid replacement policy.
    ///
    /// # Parameters
    ///
    /// - `output`: Asynchronous byte output that receives encoded bytes.
    /// - `codec`: Charset codec used to encode text.
    /// - `policy`: Unmappable-character policy.
    /// - `buffer_capacity`: Requested encoded-byte capacity, raised if needed
    ///   to hold one encoded character.
    ///
    /// # Returns
    ///
    /// Returns a writer using the requested effective capacity and LF line
    /// endings.
    ///
    /// # Errors
    ///
    /// Returns [`CharsetEncodeError`] when replacement mode cannot encode the
    /// configured replacement character.
    pub fn try_new_with_buffer_capacity(
        output: O,
        codec: C,
        policy: CharsetEncodePolicy,
        buffer_capacity: usize,
    ) -> Result<Self, CharsetEncodeError> {
        let encoder = CharsetEncoder::with_policy(codec, policy)?;
        let one_character = encoder.max_transcode_output_len(1).unwrap_or(1);
        let capacity = buffer_capacity.max(one_character).max(1);
        Ok(Self {
            output: AsyncTranscodeEncodeOutput::with_capacity(output, capacity),
            encoder,
            line_ending: LineEnding::Lf,
            started: false,
            finished: false,
        })
    }

    /// Returns a shared reference to the asynchronous byte output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output. Encoded bytes may still be pending in this
    /// writer.
    #[must_use = "use the returned output reference to access the wrapped output"]
    #[inline]
    pub const fn output(&self) -> &O {
        self.output.inner()
    }

    /// Returns a mutable reference to the asynchronous byte output.
    ///
    /// Direct output operations can be ordered before bytes retained by this
    /// writer. Call [`Self::flush_async`] first when ordering matters.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output.
    #[must_use = "use the returned output reference to access the wrapped output"]
    #[inline]
    pub fn output_mut(&mut self) -> &mut O {
        self.output.inner_mut()
    }

    /// Returns the configured line ending.
    ///
    /// # Returns
    ///
    /// Returns the line ending used by future line writes.
    #[must_use]
    #[inline]
    pub const fn configured_line_ending(&self) -> LineEnding {
        self.line_ending
    }

    /// Sets the line ending used by [`Self::write_line_fully_async`].
    ///
    /// # Parameters
    ///
    /// - `line_ending`: Line ending for subsequent line writes.
    ///
    /// # Returns
    ///
    /// Returns this configured writer.
    #[must_use]
    #[inline]
    pub const fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Returns the wrapped output and every encoded byte still pending as a
    /// vector for convenience. This copies the readable window; use
    /// [`Self::into_buffered_parts`] to recover the existing buffer without
    /// copying.
    ///
    /// This method does not call [`Self::finish_async`] or
    /// [`qubit_io::AsyncOutput::flush_async`] on the wrapped output. Call
    /// [`Self::finish_async`] first for normal completion; after a successful
    /// finish, the returned byte vector is empty. Calling this method first
    /// explicitly abandons encoder lifecycle output that has not been emitted
    /// while transferring already encoded bytes to the caller.
    /// Recovery assumes that inner I/O calls obey their normal return
    /// contracts. The asynchronous buffer does not track poisoning or
    /// indeterminate delivery after an inner I/O panic. If such a panic is
    /// caught, the returned pending bytes may overlap a prefix already
    /// delivered. Reconcile external side effects before retrying or replaying
    /// them.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output and the bytes pending after inner I/O calls
    /// that obeyed their normal return contracts, in logical write order.
    #[must_use = "the returned output and pending bytes must be handled"]
    pub fn into_parts(self) -> (O, Vec<u8>) {
        let (output, pending) = self.into_buffered_parts();
        (output, pending.readable().to_vec())
    }

    /// Recovers the output and its owned pending-byte buffer without copying.
    ///
    /// This method performs no I/O, allocation for a new pending buffer, or
    /// encoder finalization. Use `pending.readable()` for the logical pending
    /// window; the backing storage may also contain a consumed prefix and
    /// spare capacity. Recovery assumes that inner I/O calls obey their normal
    /// return contracts. The asynchronous buffer does not track poisoning or
    /// indeterminate delivery after an inner I/O panic. If such a panic is
    /// caught, the returned pending bytes may overlap a prefix already
    /// delivered. Reconcile external side effects before retrying or replaying
    /// them. Call `finish_async` first for normal completion. Otherwise,
    /// encoder lifecycle output that has not been emitted is abandoned.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output and the existing pending-byte buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_codec_text::CharsetEncodePolicy;
    /// use qubit_codec_text::Utf8Codec;
    /// use qubit_io::AsyncByteOutput;
    /// use qubit_io_text::AsyncCharsetTextWriter;
    ///
    /// fn recover<O: AsyncByteOutput>(output: O) {
    ///     let writer = AsyncCharsetTextWriter::new(
    ///         output, Utf8Codec, CharsetEncodePolicy::report(),
    ///     );
    ///     let (_output, pending) = writer.into_buffered_parts();
    ///     assert!(pending.is_empty());
    /// }
    /// ```
    #[must_use = "the output and pending-byte buffer must be handled"]
    #[inline]
    pub fn into_buffered_parts(self) -> (O, Buffer<u8>) {
        self.output.into_parts()
    }

    /// Returns an error when encoding has already been finished.
    fn ensure_open(&self) -> io::Result<()> {
        if self.finished {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot write after asynchronous text writer has been finished",
            ));
        }
        Ok(())
    }
}

impl<O, C> AsyncTextWriter for AsyncCharsetTextWriter<O, C>
where
    O: AsyncByteOutput + Unpin,
    C: CharsetCodec<Unit = u8>,
{
    type Error = io::Error;

    fn line_ending(&self) -> LineEnding {
        self.configured_line_ending()
    }

    async fn write_char_async(&mut self, ch: char) -> Result<(), Self::Error> {
        AsyncCharsetTextWriter::write_char_async(self, ch).await
    }

    async fn write_chars_async(&mut self, chars: &[char]) -> Result<usize, Self::Error> {
        AsyncCharsetTextWriter::write_chars_async(self, chars).await
    }

    async fn write_str_async(&mut self, text: &str) -> Result<usize, Self::Error> {
        AsyncCharsetTextWriter::write_str_async(self, text).await
    }

    async fn write_chars_fully_async(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        AsyncCharsetTextWriter::write_chars_fully_async(self, chars).await
    }

    async fn write_str_fully_async(&mut self, text: &str) -> Result<(), Self::Error> {
        AsyncCharsetTextWriter::write_str_fully_async(self, text).await
    }

    async fn write_line_fully_async(&mut self, line: &str) -> Result<(), Self::Error> {
        AsyncCharsetTextWriter::write_line_fully_async(self, line).await
    }

    async fn flush_async(&mut self) -> Result<(), Self::Error> {
        AsyncCharsetTextWriter::flush_async(self).await
    }

    async fn finish_async(&mut self) -> Result<(), Self::Error> {
        AsyncCharsetTextWriter::finish_async(self).await
    }
}

impl<O, C> AsyncCharsetTextWriter<O, C>
where
    O: AsyncByteOutput + Unpin,
    C: CharsetCodec<Unit = u8>,
{
    /// Asynchronously writes one character.
    ///
    /// # Parameters
    ///
    /// - `ch`: Character to encode and write.
    ///
    /// A successful write accepts encoded bytes into the buffer without
    /// guaranteeing delivery. Capacity pressure may deliver older bytes;
    /// call [`Self::flush_async`] when delivery must be observable.
    ///
    /// # Errors
    ///
    /// Returns encoding or output errors, or an invalid-input error after the
    /// writer has been finished. Main-stage encoding indices use Rust `char`
    /// positions relative to the supplied character slice.
    ///
    /// # Cancellation safety
    ///
    /// This operation consumes its one supplied character before any later
    /// output poll, so cancellation does not leave an ambiguous source cursor.
    pub async fn write_char_async(&mut self, ch: char) -> io::Result<()> {
        let consumed = self.write_chars_async(&[ch]).await?;
        debug_assert_eq!(1, consumed);
        Ok(())
    }

    /// Asynchronously writes a character slice.
    ///
    /// # Parameters
    ///
    /// - `chars`: Characters to encode and write.
    ///
    /// A successful write accepts encoded bytes into the buffer without
    /// guaranteeing delivery. Capacity pressure may deliver older bytes;
    /// call [`Self::flush_async`] when delivery must be observable.
    ///
    /// # Errors
    ///
    /// Returns encoding or output errors, or an invalid-input error after the
    /// writer has been finished.
    ///
    /// # Cancellation safety
    ///
    /// This operation returns after one encoder step. Advance the caller's
    /// source cursor by the returned count before calling it again.
    pub async fn write_chars_async(&mut self, chars: &[char]) -> io::Result<usize> {
        self.ensure_open()?;
        if chars.is_empty() {
            return Ok(0);
        }
        self.encode_chars_async(chars, 0).await
    }

    /// Asynchronously writes an entire character slice.
    ///
    /// # Parameters
    ///
    /// - `chars`: Characters to encode and write.
    ///
    /// # Errors
    ///
    /// Returns encoding or output errors. A character prefix may already be
    /// committed when an error is returned. Returns an invalid-input error if
    /// the writer has been finished, including when `chars` is empty.
    /// Main-stage encoding indices are relative to this complete call and use
    /// Rust `char` positions; they do not report a safely retryable cursor.
    ///
    /// # Cancellation safety
    ///
    /// This convenience loop is not cancellation-safe. After cancellation,
    /// its source position cannot be recovered reliably; use the single-step
    /// API for cancellation-sensitive code.
    pub async fn write_chars_fully_async(&mut self, chars: &[char]) -> io::Result<()> {
        self.ensure_open()?;
        let mut index = 0;
        while index < chars.len() {
            index += self.encode_chars_async(&chars[index..], index).await?;
        }
        Ok(())
    }

    /// Asynchronously writes a UTF-8 Rust string through the selected charset.
    ///
    /// # Parameters
    ///
    /// - `text`: Unicode text to encode and write.
    ///
    /// A successful write accepts encoded bytes into the buffer without
    /// guaranteeing delivery. Capacity pressure may deliver older bytes;
    /// call [`Self::flush_async`] when delivery must be observable.
    ///
    /// # Errors
    ///
    /// Returns encoding or output errors, or an invalid-input error after the
    /// writer has been finished. Main-stage encoding indices are relative to
    /// this supplied string suffix and use Rust `char` positions.
    ///
    /// # Cancellation safety
    ///
    /// This operation returns the UTF-8 byte length of the committed character
    /// prefix. Resume with `&text[returned..]`.
    pub async fn write_str_async(&mut self, text: &str) -> io::Result<usize> {
        self.ensure_open()?;
        if text.is_empty() {
            return Ok(0);
        }
        let (bytes, _) = self.write_str_step_async(text, 0).await?;
        Ok(bytes)
    }

    /// Encodes one bounded string step and returns its committed byte and
    /// character counts.
    async fn write_str_step_async(&mut self, text: &str, base: usize) -> io::Result<(usize, usize)> {
        if text.is_empty() {
            return Ok((0, 0));
        }
        let mut chars = ['\0'; DEFAULT_CHAR_CHUNK_CAPACITY];
        let mut byte_ends = [0; DEFAULT_CHAR_CHUNK_CAPACITY];
        let mut char_count = 0;
        for (byte_index, ch) in text.char_indices() {
            chars[char_count] = ch;
            byte_ends[char_count] = byte_index + ch.len_utf8();
            char_count += 1;
            if char_count == DEFAULT_CHAR_CHUNK_CAPACITY {
                break;
            }
        }
        let consumed = self.encode_chars_async(&chars[..char_count], base).await?;
        Ok((byte_ends[consumed - 1], consumed))
    }

    /// Asynchronously writes an entire UTF-8 Rust string through the selected
    /// charset.
    ///
    /// # Parameters
    ///
    /// - `text`: Unicode text to encode and write.
    ///
    /// # Errors
    ///
    /// Returns encoding or output errors. A UTF-8 prefix may already be
    /// committed when an error is returned. Returns an invalid-input error if
    /// the writer has been finished, including when `text` is empty.
    /// Main-stage encoding indices are relative to this complete call and use
    /// Rust `char` positions; they do not report a safely retryable cursor.
    ///
    /// # Cancellation safety
    ///
    /// This convenience loop is not cancellation-safe. After cancellation,
    /// its source position cannot be recovered reliably; use the single-step
    /// API for cancellation-sensitive code.
    pub async fn write_str_fully_async(&mut self, text: &str) -> io::Result<()> {
        self.ensure_open()?;
        let mut byte_offset = 0;
        let mut char_base = 0;
        while byte_offset < text.len() {
            let (bytes, chars) = self.write_str_step_async(&text[byte_offset..], char_base).await?;
            byte_offset += bytes;
            char_base += chars;
        }
        Ok(())
    }

    /// Asynchronously writes text followed by the configured line ending.
    ///
    /// # Parameters
    ///
    /// - `line`: Line contents without the added line ending.
    ///
    /// # Errors
    ///
    /// Returns encoding or output errors. Main-stage encoding indices are
    /// relative to the body or the appended line ending, which are encoded as
    /// separate calls and each use Rust `char` positions from zero.
    ///
    /// # Cancellation safety
    ///
    /// This convenience operation is not cancellation-safe. Use the single
    /// step APIs for cancellation-sensitive code.
    pub async fn write_line_fully_async(&mut self, line: &str) -> io::Result<()> {
        self.write_str_fully_async(line).await?;
        self.write_str_fully_async(self.line_ending.as_str()).await
    }

    /// Asynchronously drains encoded bytes and flushes the wrapped output.
    ///
    /// This operation does not finish the charset encoder.
    ///
    /// # Errors
    ///
    /// Returns output write or flush errors.
    ///
    /// # Cancellation safety
    ///
    /// Cancelling this future can commit a pending-byte prefix. Remaining
    /// bytes stay in this writer and `flush_async` can be called again.
    pub async fn flush_async(&mut self) -> io::Result<()> {
        self.output.flush_async().await
    }

    /// Asynchronously finishes the encoder and flushes the byte output.
    ///
    /// A failed output write retains unwritten encoded bytes. Calling this
    /// method again retries delivery without running encoder finalization a
    /// second time.
    ///
    /// # Errors
    ///
    /// Returns encoder finalization or output errors. After encoder
    /// finalization succeeds, later text-write methods return
    /// [`io::ErrorKind::InvalidInput`] even when delivery still needs retrying.
    ///
    /// # Cancellation safety
    ///
    /// Cancelling this future can finalize the encoder and commit a pending
    /// byte prefix. Retry `finish_async` on the same writer; do not write more
    /// text unless finalization has not yet succeeded.
    pub async fn finish_async(&mut self) -> io::Result<()> {
        if !self.finished {
            self.ensure_started_async().await?;
            let mut map_error = encode_error_to_io;
            self.output.finish_async(&mut self.encoder, &mut map_error).await?;
            self.finished = true;
        }
        self.output.flush_async().await
    }

    /// Starts the encoder lifecycle, retaining reset output until capacity or
    /// an explicit delivery call.
    async fn ensure_started_async(&mut self) -> io::Result<()> {
        if !self.started {
            let mut map_error = encode_error_to_io;
            self.output.reset_async(&mut self.encoder, &mut map_error).await?;
            self.started = true;
        }
        Ok(())
    }

    /// Encodes one character-slice progress step using the supplied call base.
    async fn encode_chars_async(&mut self, chars: &[char], base: usize) -> io::Result<usize> {
        self.ensure_started_async().await?;
        let mut map_error = |error| encode_error_to_io(offset_main_encode_error(error, base));
        let progress = self
            .output
            .transcode_async(&mut self.encoder, &mut map_error, chars, 0, chars.len())
            .await?;
        Ok(progress.read())
    }
}

/// Offsets input coordinates for charset errors in the main transcode stage.
///
/// Only input-related failures are shifted; lifecycle, output, and framework
/// failures retain their original error and coordinate semantics.
fn offset_main_encode_error(
    error: TranscodeEncodeError<CharsetEncodeError, char>,
    base: usize,
) -> TranscodeEncodeError<CharsetEncodeError, char> {
    match error {
        TranscodeEncodeError::Unencodable { input_index, value } => TranscodeEncodeError::Unencodable {
            input_index: base.saturating_add(input_index),
            value,
        },
        TranscodeEncodeError::Domain(TranscodeDomainError::Main {
            source,
            input_index,
            input_consumed,
        }) => {
            let source = match source.kind() {
                CharsetEncodeErrorKind::InvalidCodePoint { .. }
                | CharsetEncodeErrorKind::UnmappableCharacter { .. }
                | CharsetEncodeErrorKind::UnencodableValue => source.offset_by(base),
                _ => source,
            };
            TranscodeEncodeError::Domain(TranscodeDomainError::Main {
                source,
                input_index: base.saturating_add(input_index),
                input_consumed,
            })
        }
        other => other,
    }
}
