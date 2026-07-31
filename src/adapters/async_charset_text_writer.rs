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
    AsyncTranscodeEncodeOutput,
    Transcoder,
};
use qubit_codec_text::{
    CharsetCodec,
    CharsetEncoder,
};
use qubit_io::AsyncOutput;

use crate::{
    CodingErrorPolicy,
    LineEnding,
    adapters::charset_text_writer::create_encoder,
    io_error::encode_error_to_io,
};

/// Default encoded-byte capacity used by asynchronous charset writers.
const DEFAULT_BUFFER_CAPACITY: usize = 8 * 1024;

/// Number of string characters converted in one bounded chunk.
const DEFAULT_CHAR_CHUNK_CAPACITY: usize = 256;

/// Asynchronously encodes Unicode text into a charset byte output.
///
/// Encoded bytes live in this writer until the wrapped output accepts them.
/// Each `write_*_async` operation returns after one encoder step, before any
/// later output poll, and reports exactly how much source it committed. Use
/// the explicit `*_fully_async` methods when a complete source range is
/// required; those convenience loops can commit a prefix before cancellation.
///
/// # Examples
///
/// ```no_run
/// use qubit_codec_text::Utf8Codec;
/// use qubit_io::AsyncOutput;
/// use qubit_io_text::{AsyncCharsetTextWriter, CodingErrorPolicy};
///
/// async fn write_all<O>(output: O) -> std::io::Result<O>
/// where
///     O: AsyncOutput<Item = u8> + Unpin,
/// {
///     let mut writer = AsyncCharsetTextWriter::new(
///         output,
///         Utf8Codec,
///         CodingErrorPolicy::Strict,
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
    O: AsyncOutput<Item = u8>,
    C: CharsetCodec<Unit = u8>,
{
    output: AsyncTranscodeEncodeOutput<O>,
    encoder: CharsetEncoder<C>,
    line_ending: LineEnding,
    started: bool,
    finished: bool,
}

impl<O, C> AsyncCharsetTextWriter<O, C>
where
    O: AsyncOutput<Item = u8>,
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
    pub fn new(output: O, codec: C, policy: CodingErrorPolicy) -> Self {
        Self::new_with_buffer_capacity(
            output,
            codec,
            policy,
            DEFAULT_BUFFER_CAPACITY,
        )
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
    pub fn new_with_buffer_capacity(
        output: O,
        codec: C,
        policy: CodingErrorPolicy,
        buffer_capacity: usize,
    ) -> Self {
        let encoder = create_encoder(codec, policy);
        let one_character = encoder.max_transcode_output_len(1).unwrap_or(1);
        let capacity = buffer_capacity.max(one_character).max(1);
        Self {
            output: AsyncTranscodeEncodeOutput::with_capacity(output, capacity),
            encoder,
            line_ending: LineEnding::Lf,
            started: false,
            finished: false,
        }
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
    pub const fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Returns a shared reference to the asynchronous byte output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output. Encoded bytes may still be pending in this
    /// writer.
    #[must_use]
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
    pub fn output_mut(&mut self) -> &mut O {
        self.output.inner_mut()
    }

    /// Returns the configured line ending.
    ///
    /// # Returns
    ///
    /// Returns the line ending used by future line writes.
    #[must_use]
    pub const fn configured_line_ending(&self) -> LineEnding {
        self.line_ending
    }

    /// Returns the wrapped output and every encoded byte still pending.
    ///
    /// This method does not call [`Self::finish_async`] or
    /// [`AsyncOutput::flush_async`] on the wrapped output. Call
    /// [`Self::finish_async`] first for normal completion; after a successful
    /// finish, the returned byte vector is empty. Calling this method first
    /// explicitly abandons encoder lifecycle output that has not been emitted
    /// while transferring already encoded bytes to the caller.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output and pending bytes in logical write order.
    #[must_use = "the returned output and pending bytes must be handled"]
    pub fn into_parts(self) -> (O, Vec<u8>) {
        let (output, pending) = self.output.into_parts();
        (output, pending.readable().to_vec())
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

impl<O, C> AsyncCharsetTextWriter<O, C>
where
    O: AsyncOutput<Item = u8> + Unpin,
    C: CharsetCodec<Unit = u8>,
{
    /// Starts the encoder lifecycle and sends any stream prefix.
    async fn ensure_started_async(&mut self) -> io::Result<()> {
        if !self.started {
            let mut map_error = encode_error_to_io;
            self.output
                .reset_async(&mut self.encoder, &mut map_error)
                .await?;
            self.started = true;
        }
        self.output.drain_async().await
    }

    /// Encodes one character-slice progress step.
    async fn encode_chars_async(
        &mut self,
        chars: &[char],
    ) -> io::Result<usize> {
        self.ensure_started_async().await?;
        let mut map_error = encode_error_to_io;
        let progress = self
            .output
            .transcode_async(
                &mut self.encoder,
                &mut map_error,
                chars,
                0,
                chars.len(),
            )
            .await?;
        Ok(progress.read())
    }

    /// Asynchronously writes one character.
    ///
    /// # Parameters
    ///
    /// - `ch`: Character to encode and write.
    ///
    /// # Errors
    ///
    /// Returns encoding or output errors, or an invalid-input error after the
    /// writer has been finished.
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
    /// # Errors
    ///
    /// Returns encoding or output errors, or an invalid-input error after the
    /// writer has been finished.
    ///
    /// # Cancellation safety
    ///
    /// This operation returns after one encoder step. Advance the caller's
    /// source cursor by the returned count before calling it again.
    pub async fn write_chars_async(
        &mut self,
        chars: &[char],
    ) -> io::Result<usize> {
        self.ensure_open()?;
        if chars.is_empty() {
            return Ok(0);
        }
        self.encode_chars_async(chars).await
    }

    /// Asynchronously writes an entire character slice.
    ///
    /// # Cancellation safety
    ///
    /// This convenience loop is not cancellation-safe. After cancellation,
    /// its source position cannot be recovered reliably; use the single-step
    /// API for cancellation-sensitive code.
    pub async fn write_chars_fully_async(
        &mut self,
        chars: &[char],
    ) -> io::Result<()> {
        let mut index = 0;
        while index < chars.len() {
            index += self.write_chars_async(&chars[index..]).await?;
        }
        Ok(())
    }

    /// Asynchronously writes a UTF-8 Rust string through the selected charset.
    ///
    /// # Parameters
    ///
    /// - `text`: Unicode text to encode and write.
    ///
    /// # Errors
    ///
    /// Returns encoding or output errors, or an invalid-input error after the
    /// writer has been finished.
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
        let consumed = self.write_chars_async(&chars[..char_count]).await?;
        Ok(byte_ends[consumed - 1])
    }

    /// Asynchronously writes an entire UTF-8 Rust string through the selected
    /// charset.
    ///
    /// # Cancellation safety
    ///
    /// This convenience loop is not cancellation-safe. After cancellation,
    /// its source position cannot be recovered reliably; use the single-step
    /// API for cancellation-sensitive code.
    pub async fn write_str_fully_async(
        &mut self,
        text: &str,
    ) -> io::Result<()> {
        let mut offset = 0;
        while offset < text.len() {
            offset += self.write_str_async(&text[offset..]).await?;
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
    /// Returns encoding or output errors.
    ///
    /// # Cancellation safety
    ///
    /// This convenience operation is not cancellation-safe. Use the single
    /// step APIs for cancellation-sensitive code.
    pub async fn write_line_fully_async(
        &mut self,
        line: &str,
    ) -> io::Result<()> {
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
            self.output
                .finish_async(&mut self.encoder, &mut map_error)
                .await?;
            self.finished = true;
        }
        self.output.flush_async().await
    }
}
