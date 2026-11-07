// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;

use qubit_codec::TranscodeDomainError;
use qubit_codec::TranscodeEncodeError;
use qubit_codec::Transcoder;
use qubit_codec_text::CharsetCodec;
use qubit_codec_text::CharsetEncodeError;
use qubit_codec_text::CharsetEncodeErrorKind;
use qubit_io::AsyncByteOutput;

use super::AsyncCharsetTextWriter;
use super::DEFAULT_CHAR_CHUNK_CAPACITY;
use crate::io_error::encode_error_to_io;

impl<O, C> AsyncCharsetTextWriter<O, C>
where
    O: AsyncByteOutput + Unpin,
    C: CharsetCodec<Unit = u8>,
{
    /// Encodes one bounded string step and returns its committed byte and
    /// character counts.
    ///
    /// # Parameters
    ///
    /// - `text`: Remaining UTF-8 source text to encode.
    /// - `base`: Character offset of this text within the complete write call.
    ///
    /// # Returns
    ///
    /// Returns the committed UTF-8 byte count and character count, or
    /// `(0, 0)` for empty input.
    ///
    /// # Errors
    ///
    /// Returns encoder or output errors.
    pub(super) async fn write_str_step_async(&mut self, text: &str, base: usize) -> io::Result<(usize, usize)> {
        if text.is_empty() {
            return Ok((0, 0));
        }
        let mut chars = ['\0'; DEFAULT_CHAR_CHUNK_CAPACITY];
        let mut byte_ends = [0; DEFAULT_CHAR_CHUNK_CAPACITY];
        let char_limit = self.string_step_char_limit();
        let mut char_count = 0;
        for (byte_index, ch) in text.char_indices() {
            chars[char_count] = ch;
            byte_ends[char_count] = byte_index + ch.len_utf8();
            char_count += 1;
            if char_count == char_limit {
                break;
            }
        }
        let consumed = self.encode_chars_async(&chars[..char_count], base).await?;
        Ok((byte_ends[consumed - 1], consumed))
    }

    /// Encodes one character-slice progress step using the supplied call base.
    ///
    /// # Parameters
    ///
    /// - `chars`: Character slice to encode in this progress step.
    /// - `base`: Character offset of this slice within the complete write call.
    ///
    /// # Returns
    ///
    /// Returns the number of source characters committed.
    ///
    /// # Errors
    ///
    /// Returns encoder or output errors.
    pub(super) async fn encode_chars_async(&mut self, chars: &[char], base: usize) -> io::Result<usize> {
        self.ensure_started_async().await?;
        let mut map_error = |error| encode_error_to_io(offset_main_encode_error(error, base));
        let progress = self
            .output
            .transcode_async(&mut self.encoder, &mut map_error, chars, 0, chars.len())
            .await?;
        Ok(progress.read())
    }

    /// Starts the encoder lifecycle, retaining reset output until capacity or
    /// an explicit delivery call.
    ///
    /// # Errors
    ///
    /// Returns encoder or output errors while resetting the encoder.
    pub(super) async fn ensure_started_async(&mut self) -> io::Result<()> {
        if !self.started {
            let mut map_error = encode_error_to_io;
            self.output.reset_async(&mut self.encoder, &mut map_error).await?;
            self.started = true;
        }
        Ok(())
    }

    /// Returns an error when encoding has already been finished.
    ///
    /// # Errors
    ///
    /// Returns an invalid-input error when encoder finalization has succeeded.
    pub(super) fn ensure_open(&self) -> io::Result<()> {
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
    O: AsyncByteOutput + Unpin,
    C: CharsetCodec<Unit = u8>,
{
    /// Limits string pre-conversion to the configured output capacity.
    ///
    /// This budget bounds the number of source characters converted before an
    /// encode step; the output driver still decides how many it can commit.
    ///
    /// # Returns
    ///
    /// Returns a character limit between one and the fixed chunk capacity.
    fn string_step_char_limit(&self) -> usize {
        let per_char = self.encoder.max_transcode_output_len(1).unwrap_or(1).max(1);
        (self.output.capacity() / per_char).clamp(1, DEFAULT_CHAR_CHUNK_CAPACITY)
    }
}

/// Offsets input coordinates for charset errors in the main transcode stage.
///
/// Only input-related failures are shifted; lifecycle, output, and framework
/// failures retain their original error and coordinate semantics.
///
/// # Parameters
///
/// - `error`: Transcode error produced while encoding the current input slice.
/// - `base`: Character offset of that slice within the complete write call.
///
/// # Returns
///
/// Returns the error with main-stage input coordinates shifted by `base`.
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
