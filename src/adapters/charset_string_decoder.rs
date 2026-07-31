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

use qubit_codec::{
    TranscodeDecodeError,
    TranscodeStatus,
    Transcoder,
};
use qubit_codec_text::{
    Charset,
    CharsetCodec,
    CharsetDecodeError,
    CharsetDecodeErrorKind,
    CharsetDecodePolicy,
    CharsetDecoder,
    MalformedAction,
};
use qubit_io::{
    try_reserve_string,
    try_reserve_vec,
};

const CHAR_CHUNK_CAPACITY: usize = 256;

/// Convenience decoder for complete inputs that should become a [`String`].
///
/// `CharsetStringDecoder` owns a [`CharsetDecoder`] and adapts the `char`
/// output expected by the transcode layer into UTF-8 `String` storage.
/// Lower-level streaming code should use [`CharsetDecoder`] directly; this
/// type is for closed, in-memory string conversions.
///
/// # Type Parameters
///
/// - `C`: Charset codec used to decode source units into Unicode scalar values.
///
/// # Examples
///
/// ```
/// use qubit_codec_text::Utf8Codec;
/// use qubit_io_text::CharsetStringDecoder;
///
/// let mut decoder = CharsetStringDecoder::new(Utf8Codec);
/// let decoded = decoder.decode_to_string("中文".as_bytes())?;
/// assert_eq!("中文", decoded);
/// # Ok::<(), qubit_codec_text::CharsetDecodeError>(())
/// ```
pub struct CharsetStringDecoder<C>
where
    C: CharsetCodec,
{
    decoder: CharsetDecoder<C>,
    charset: Charset,
}

impl<C> CharsetStringDecoder<C>
where
    C: CharsetCodec,
{
    /// Creates a string decoder with the default replacement policy.
    ///
    /// # Parameters
    ///
    /// - `codec`: Charset codec used to decode input units.
    ///
    /// # Returns
    ///
    /// Returns a decoder using [`MalformedAction::Replace`].
    #[must_use]
    #[inline]
    pub fn new(codec: C) -> Self {
        Self::with_policy(codec, CharsetDecodePolicy::default())
    }

    /// Creates a string decoder with an explicit malformed-input policy.
    ///
    /// # Parameters
    ///
    /// - `codec`: Charset codec used to decode input units.
    /// - `policy`: Malformed-input policy used by the decoder.
    ///
    /// # Returns
    ///
    /// Returns a string decoder configured with `policy`.
    #[must_use]
    pub fn with_policy(codec: C, policy: CharsetDecodePolicy) -> Self {
        let charset = codec.charset();
        Self {
            decoder: CharsetDecoder::with_policy(codec, policy),
            charset,
        }
    }

    /// Returns the configured malformed-input action.
    #[must_use]
    #[inline]
    pub const fn malformed_action(&self) -> MalformedAction {
        self.decoder.malformed_action()
    }

    /// Returns the configured replacement character.
    #[must_use]
    #[inline]
    pub const fn replacement(&self) -> char {
        self.decoder.replacement()
    }

    /// Returns the wrapped charset decoder.
    #[must_use]
    #[inline(always)]
    pub const fn decoder(&self) -> &CharsetDecoder<C> {
        &self.decoder
    }

    /// Returns a mutable reference to the wrapped charset decoder.
    #[inline(always)]
    pub fn decoder_mut(&mut self) -> &mut CharsetDecoder<C> {
        &mut self.decoder
    }

    /// Consumes this string decoder and returns the wrapped charset decoder.
    #[must_use]
    #[inline]
    pub fn into_decoder(self) -> CharsetDecoder<C> {
        self.decoder
    }

    /// Makes the next reserve operation fail in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_fail_next_reserve() {
        COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(0));
    }

    /// Makes a later reserve operation fail in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_fail_reserve_after(successful_attempts: usize) {
        COVERAGE_RESERVE_FAIL_AFTER
            .with(|state| state.set(successful_attempts));
    }

    /// Shrinks the next character buffer capacity in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_shrink_next_char_capacity_by(amount: usize) {
        COVERAGE_CHAR_CAPACITY_SHRINK_BY.with(|state| state.set(amount));
    }

    /// Clears coverage-only reserve failure hooks.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_reset_reserve_hooks() {
        COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(usize::MAX));
        COVERAGE_CHAR_CAPACITY_SHRINK_BY.with(|state| state.set(0));
    }

    /// Maps a transcode decode error in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_map_decode_error(
        charset: Charset,
        error: TranscodeDecodeError<CharsetDecodeError>,
    ) -> CharsetDecodeError {
        map_decode_error(charset, error)
    }

    /// Maps a finish-buffer decode error in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_map_finish_decode_error(
        charset: Charset,
        error: TranscodeDecodeError<CharsetDecodeError>,
        output_offset: usize,
    ) -> CharsetDecodeError {
        map_finish_decode_error(charset, error, output_offset)
    }

    /// Decodes a complete input slice into an owned [`String`].
    ///
    /// # Parameters
    ///
    /// - `input`: Complete source units.
    ///
    /// # Returns
    ///
    /// Returns decoded UTF-8 text.
    ///
    /// # Errors
    ///
    /// Returns [`CharsetDecodeError`] when decoding fails, output sizing
    /// overflows, or the complete input ends with an incomplete sequence.
    pub fn decode_to_string(
        &mut self,
        input: &[C::Unit],
    ) -> Result<String, CharsetDecodeError> {
        let mut output = String::new();
        self.decode_to_string_into(input, 0, &mut output)?;
        Ok(output)
    }

    /// Decodes a complete input slice and appends the decoded text to
    /// `output`.
    ///
    /// This method treats `input[input_index..]` as a closed stream. If the
    /// underlying decoder requests more input, the tail is reported as
    /// [`CharsetDecodeErrorKind::IncompleteSequence`].
    ///
    /// # Parameters
    ///
    /// - `input`: Complete input slice visible to the decoder.
    /// - `input_index`: Absolute index where decoding starts.
    /// - `output`: String receiving decoded text.
    ///
    /// # Errors
    ///
    /// Returns [`CharsetDecodeError`] when `input_index` is outside `input`,
    /// decoding fails, or output sizing overflows.
    pub fn decode_to_string_into(
        &mut self,
        input: &[C::Unit],
        input_index: usize,
        output: &mut String,
    ) -> Result<(), CharsetDecodeError> {
        if input_index > input.len() {
            return Err(CharsetDecodeError::new(
                self.charset,
                CharsetDecodeErrorKind::InvalidInputIndex {
                    input_len: input.len(),
                },
                input_index,
            ));
        }
        let output_len = output.len();
        let result = self.decode_bounded(input, input_index, output);
        if result.is_err() {
            output.truncate(output_len);
        }
        result
    }

    /// Drives one closed decode lifecycle through a bounded character buffer.
    ///
    /// # Parameters
    ///
    /// - `input`: Complete source-unit slice visible to the decoder.
    /// - `input_index`: Absolute unit index where decoding starts.
    /// - `output`: String receiving decoded UTF-8 text.
    ///
    /// # Errors
    ///
    /// Returns [`CharsetDecodeError`] for malformed or incomplete input,
    /// invalid transcode progress, output-length overflow, or allocation
    /// failure.
    fn decode_bounded(
        &mut self,
        input: &[C::Unit],
        input_index: usize,
        output: &mut String,
    ) -> Result<(), CharsetDecodeError> {
        let reset_capacity = match self.decoder.max_reset_output_len() {
            Ok(capacity) => capacity,
            Err(_) => return Err(output_length_overflow(self.charset)),
        };
        let char_capacity = reset_capacity.max(CHAR_CHUNK_CAPACITY);
        #[cfg(coverage)]
        let char_capacity =
            char_capacity.saturating_sub(coverage_take_char_capacity_shrink());
        let mut chars = Vec::new();
        ensure_char_capacity(&mut chars, char_capacity, self.charset)?;
        let reset_written = match self.decoder.reset(&mut chars, 0) {
            Ok(written) => written,
            Err(error) => return Err(map_decode_error(self.charset, error)),
        };
        append_chars(output, &chars[..reset_written], self.charset)?;
        let mut output_char_count = reset_written;

        let mut input_cursor = input_index;
        loop {
            let progress = match self.decoder.transcode(
                input,
                input_cursor,
                &mut chars,
                0,
            ) {
                Ok(progress) => progress,
                Err(error) => {
                    return Err(map_decode_error(self.charset, error));
                }
            };
            append_chars(output, &chars[..progress.written()], self.charset)?;
            let Some(next_output_char_count) =
                output_char_count.checked_add(progress.written())
            else {
                return Err(output_length_overflow(self.charset));
            };
            output_char_count = next_output_char_count;
            input_cursor += progress.read();
            match progress.status() {
                TranscodeStatus::Complete => break,
                TranscodeStatus::NeedInput {
                    input_index,
                    required,
                    available,
                } => {
                    let kind = CharsetDecodeErrorKind::IncompleteSequence {
                        required: required.get(),
                        available,
                    };
                    return Err(CharsetDecodeError::new(
                        self.charset,
                        kind,
                        input_index,
                    ));
                }
                TranscodeStatus::NeedOutput { required, .. } => {
                    if progress.read() == 0 && progress.written() == 0 {
                        let required = required.get().max(CHAR_CHUNK_CAPACITY);
                        ensure_char_capacity(
                            &mut chars,
                            required,
                            self.charset,
                        )?;
                    }
                }
            }
        }

        let finish_capacity = match self.decoder.max_finish_output_len() {
            Ok(capacity) => capacity,
            Err(_) => return Err(output_length_overflow(self.charset)),
        };
        ensure_char_capacity(&mut chars, finish_capacity, self.charset)?;
        let finish_written = match self.decoder.finish(&mut chars, 0) {
            Ok(written) => written,
            Err(error) => {
                return Err(map_finish_decode_error(
                    self.charset,
                    error,
                    output_char_count,
                ));
            }
        };
        append_chars(output, &chars[..finish_written], self.charset)
    }
}

/// Ensures that the reusable decoded-character buffer exposes `required` slots.
///
/// # Parameters
///
/// - `chars`: Reusable character buffer to grow and initialize.
/// - `required`: Minimum number of character slots required.
/// - `charset`: Charset attached to any reported error.
///
/// # Errors
///
/// Returns [`CharsetDecodeErrorKind::OutputLengthOverflow`] when the allocation
/// cannot be reserved.
fn ensure_char_capacity(
    chars: &mut Vec<char>,
    required: usize,
    charset: Charset,
) -> Result<(), CharsetDecodeError> {
    if required <= chars.len() {
        return Ok(());
    }
    let additional = required - chars.len();
    let reserve_failed = try_reserve_vec(chars, additional).is_err();
    #[cfg(coverage)]
    let reserve_failed = reserve_failed || coverage_should_fail_reserve();
    if reserve_failed {
        return Err(output_length_overflow(charset));
    }
    chars.resize(required, '\0');
    Ok(())
}

/// Appends one decoded character window to a string.
///
/// # Parameters
///
/// - `output`: String receiving the decoded characters.
/// - `chars`: Decoded character window to append.
/// - `charset`: Charset attached to any reported error.
///
/// # Errors
///
/// Returns [`CharsetDecodeErrorKind::OutputLengthOverflow`] when the required
/// UTF-8 storage cannot be reserved.
fn append_chars(
    output: &mut String,
    chars: &[char],
    charset: Charset,
) -> Result<(), CharsetDecodeError> {
    let byte_capacity = required_string_capacity(chars);
    let reserve_failed = try_reserve_string(output, byte_capacity).is_err();
    #[cfg(coverage)]
    let reserve_failed = reserve_failed || coverage_should_fail_reserve();
    if reserve_failed {
        return Err(output_length_overflow(charset));
    }
    output.extend(chars.iter());
    Ok(())
}

/// Converts a transcode-layer decode error into the charset error model.
///
/// # Parameters
///
/// - `charset`: Charset attached to framework-level failures.
/// - `error`: Error returned by the transcode layer.
///
/// # Returns
///
/// Returns the corresponding charset decoding error.
fn map_decode_error(
    charset: Charset,
    error: TranscodeDecodeError<CharsetDecodeError>,
) -> CharsetDecodeError {
    CharsetDecodeError::from_transcode_error(charset, error)
}

/// Maps a bounded finish-buffer index to the complete decoded output index.
///
/// # Parameters
///
/// - `charset`: Charset attached to framework-level failures.
/// - `error`: Error whose output index is relative to the finish buffer.
/// - `output_offset`: Character count already appended before finishing.
///
/// # Returns
///
/// Returns an error whose output index is relative to the complete output.
fn map_finish_decode_error(
    charset: Charset,
    error: TranscodeDecodeError<CharsetDecodeError>,
    output_offset: usize,
) -> CharsetDecodeError {
    let error = map_decode_error(charset, error);
    if matches!(error.kind(), CharsetDecodeErrorKind::OutputLengthOverflow) {
        return error;
    }
    CharsetDecodeError::new(
        charset,
        error.kind(),
        output_offset.saturating_add(error.index()),
    )
}

#[cfg(coverage)]
thread_local! {
    static COVERAGE_RESERVE_FAIL_AFTER: Cell<usize> = const { Cell::new(usize::MAX) };
    static COVERAGE_CHAR_CAPACITY_SHRINK_BY: Cell<usize> = const { Cell::new(0) };
}

/// Returns and clears the synthetic char capacity shrink request.
#[cfg(coverage)]
fn coverage_take_char_capacity_shrink() -> usize {
    COVERAGE_CHAR_CAPACITY_SHRINK_BY.with(|state| {
        let amount = state.get();
        state.set(0);
        amount
    })
}

/// Reports whether a synthetic reserve failure was requested.
#[cfg(coverage)]
fn coverage_should_fail_reserve() -> bool {
    COVERAGE_RESERVE_FAIL_AFTER.with(|state| {
        let remaining = state.get();
        if remaining == usize::MAX {
            return false;
        }
        if remaining == 0 {
            state.set(usize::MAX);
            return true;
        }
        state.set(remaining - 1);
        false
    })
}

/// Creates an output-length overflow error for `charset`.
///
/// # Parameters
///
/// - `charset`: Charset whose output length could not be represented.
///
/// # Returns
///
/// Returns an output-length-overflow error at the sentinel maximum index.
#[inline]
fn output_length_overflow(charset: Charset) -> CharsetDecodeError {
    CharsetDecodeError::new(
        charset,
        CharsetDecodeErrorKind::OutputLengthOverflow,
        usize::MAX,
    )
}

/// Returns the UTF-8 byte capacity required for a decoded character slice.
///
/// # Parameters
///
/// - `chars`: Decoded characters that will be appended to a string.
///
/// # Returns
///
/// Returns the exact number of additional UTF-8 bytes required.
fn required_string_capacity(chars: &[char]) -> usize {
    let mut capacity = 0;
    for ch in chars {
        capacity += ch.len_utf8();
    }
    capacity
}
