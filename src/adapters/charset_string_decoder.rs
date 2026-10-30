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
#[cfg(coverage)]
use std::num::NonZeroUsize;

use qubit_codec::CapacityError;
use qubit_codec::TranscodeDecodeError;
use qubit_codec::TranscodeStatus;
use qubit_codec::Transcoder;
use qubit_codec_text::Charset;
use qubit_codec_text::CharsetCodec;
use qubit_codec_text::CharsetDecodeError;
use qubit_codec_text::CharsetDecodeErrorKind;
use qubit_codec_text::CharsetDecodePolicy;
use qubit_codec_text::CharsetDecoder;
use qubit_codec_text::MalformedAction;
use qubit_utils::try_reserve_string;
use qubit_utils::try_reserve_vec;

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
    /// Owns the charset codec and decode policy used for each closed input
    /// conversion.
    decoder: CharsetDecoder<C>,
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
        Self {
            decoder: CharsetDecoder::with_policy(codec, policy),
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
    #[must_use]
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
        COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(successful_attempts));
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
        COVERAGE_FORCE_STATUS.with(|state| state.set(0));
        COVERAGE_FORCE_OUTPUT_COUNT_OVERFLOW.with(|state| state.set(false));
        COVERAGE_FORCE_RESET_CAPACITY_ERROR.with(|state| state.set(false));
        COVERAGE_FORCE_FINISH_CAPACITY_ERROR.with(|state| state.set(false));
    }

    /// Forces the next transcode status in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_force_next_status(mode: u8) {
        COVERAGE_FORCE_STATUS.with(|state| state.set(mode));
    }

    /// Forces the output character count overflow branch in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_force_output_count_overflow() {
        COVERAGE_FORCE_OUTPUT_COUNT_OVERFLOW.with(|state| state.set(true));
    }

    /// Forces reset capacity planning to fail in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_force_reset_capacity_error() {
        COVERAGE_FORCE_RESET_CAPACITY_ERROR.with(|state| state.set(true));
    }

    /// Forces finish capacity planning to fail in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_force_finish_capacity_error() {
        COVERAGE_FORCE_FINISH_CAPACITY_ERROR.with(|state| state.set(true));
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
    /// overflows, or the configured decode policy rejects malformed or
    /// incomplete input.
    pub fn decode_to_string(&mut self, input: &[C::Unit]) -> Result<String, CharsetDecodeError> {
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
    /// On success, decoded text is appended to `output`. If this method returns
    /// `Err`, the destination string retains its original contents. This
    /// guarantee applies to the destination only; it does not roll back
    /// decoder state or cover panics. Streaming text-reader operations have
    /// their own prefix-retention contracts.
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
                self.decoder.charset(),
                CharsetDecodeErrorKind::InvalidInputIndex { input_len: input.len() },
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
        let charset = self.decoder.charset();
        let reset_capacity = match coverage_capacity_result(self.decoder.max_reset_output_len(), true) {
            Ok(capacity) => capacity,
            Err(_) => return Err(output_length_overflow(charset)),
        };
        let char_capacity = reset_capacity.max(CHAR_CHUNK_CAPACITY);
        #[cfg(coverage)]
        let char_capacity = char_capacity.saturating_sub(coverage_take_char_capacity_shrink());
        let mut chars = Vec::new();
        ensure_char_capacity(&mut chars, char_capacity, charset)?;
        let reset_written = match self.decoder.reset(&mut chars, 0) {
            Ok(written) => written,
            Err(error) => return Err(map_decode_error(charset, error)),
        };
        append_chars(output, &chars[..reset_written], charset)?;
        let mut output_char_count = reset_written;

        let mut input_cursor = input_index;
        loop {
            let progress = match self.decoder.transcode_eof(input, input_cursor, &mut chars, 0) {
                Ok(progress) => progress,
                Err(error) => {
                    return Err(map_decode_error(charset, error));
                }
            };
            append_chars(output, &chars[..progress.written()], charset)?;
            let checked_output_char_count = coverage_output_count(output_char_count);
            let Some(next_output_char_count) = checked_output_char_count.checked_add(progress.written()) else {
                return Err(output_length_overflow(charset));
            };
            output_char_count = next_output_char_count;
            input_cursor += progress.read();
            match coverage_status(progress.status()) {
                TranscodeStatus::Complete => break,
                TranscodeStatus::NeedInput { required } => {
                    let available = input.len().saturating_sub(input_cursor);
                    let kind = CharsetDecodeErrorKind::IncompleteSequence {
                        required: required.get(),
                        available,
                    };
                    return Err(CharsetDecodeError::new(charset, kind, input_cursor));
                }
                TranscodeStatus::NeedOutput { required, .. } => {
                    if progress.read() == 0 && progress.written() == 0 {
                        let required = required.get().max(CHAR_CHUNK_CAPACITY);
                        ensure_char_capacity(&mut chars, required, charset)?;
                    }
                }
            }
        }

        let finish_capacity = match coverage_capacity_result(self.decoder.max_finish_output_len(), false) {
            Ok(capacity) => capacity,
            Err(_) => return Err(output_length_overflow(charset)),
        };
        ensure_char_capacity(&mut chars, finish_capacity, charset)?;
        let finish_written = match self.decoder.finish(&mut chars, 0) {
            Ok(written) => written,
            Err(error) => {
                return Err(map_finish_decode_error(charset, error, output_char_count));
            }
        };
        append_chars(output, &chars[..finish_written], charset)
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
fn ensure_char_capacity(chars: &mut Vec<char>, required: usize, charset: Charset) -> Result<(), CharsetDecodeError> {
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
fn append_chars(output: &mut String, chars: &[char], charset: Charset) -> Result<(), CharsetDecodeError> {
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
fn map_decode_error(charset: Charset, error: TranscodeDecodeError<CharsetDecodeError>) -> CharsetDecodeError {
    CharsetDecodeError::from_transcode_error(charset, error)
}

/// Preserves finish domain errors and globalizes framework output indices.
///
/// # Parameters
///
/// - `charset`: Charset attached to framework-level failures.
/// - `error`: Domain error or framework failure reported while finishing.
/// - `output_offset`: Character count already appended before finishing.
///
/// # Returns
///
/// Returns domain errors unchanged. Framework output indices are relative to
/// the complete output; other framework error indices remain unchanged.
fn map_finish_decode_error(
    charset: Charset,
    error: TranscodeDecodeError<CharsetDecodeError>,
    output_offset: usize,
) -> CharsetDecodeError {
    let failure = match error {
        TranscodeDecodeError::Failure(failure) => failure,
        TranscodeDecodeError::Domain(error) => return error.into_source(),
    };
    let error = map_decode_error(charset, TranscodeDecodeError::Failure(failure));
    if matches!(
        error.kind(),
        CharsetDecodeErrorKind::InvalidOutputIndex { .. } | CharsetDecodeErrorKind::BufferTooSmall { .. }
    ) {
        CharsetDecodeError::new(charset, error.kind(), output_offset.saturating_add(error.index()))
    } else {
        error
    }
}

#[cfg(coverage)]
thread_local! {
    static COVERAGE_RESERVE_FAIL_AFTER: Cell<usize> = const { Cell::new(usize::MAX) };
    static COVERAGE_CHAR_CAPACITY_SHRINK_BY: Cell<usize> = const { Cell::new(0) };
    static COVERAGE_FORCE_STATUS: Cell<u8> = const { Cell::new(0) };
    static COVERAGE_FORCE_OUTPUT_COUNT_OVERFLOW: Cell<bool> = const { Cell::new(false) };
    static COVERAGE_FORCE_RESET_CAPACITY_ERROR: Cell<bool> = const { Cell::new(false) };
    static COVERAGE_FORCE_FINISH_CAPACITY_ERROR: Cell<bool> = const { Cell::new(false) };
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

#[cfg(coverage)]
fn coverage_status(status: TranscodeStatus) -> TranscodeStatus {
    match COVERAGE_FORCE_STATUS.with(|state| state.replace(0)) {
        1 => TranscodeStatus::NeedInput {
            required: NonZeroUsize::MIN,
        },
        2 => TranscodeStatus::NeedOutput {
            required: NonZeroUsize::MIN,
        },
        _ => status,
    }
}

#[cfg(not(coverage))]
fn coverage_status(status: TranscodeStatus) -> TranscodeStatus {
    status
}

#[cfg(coverage)]
fn coverage_output_count(count: usize) -> usize {
    if COVERAGE_FORCE_OUTPUT_COUNT_OVERFLOW.with(|state| state.replace(false)) {
        usize::MAX
    } else {
        count
    }
}

#[cfg(not(coverage))]
fn coverage_output_count(count: usize) -> usize {
    count
}

#[cfg(coverage)]
fn coverage_capacity_result(result: Result<usize, CapacityError>, reset: bool) -> Result<usize, CapacityError> {
    let force = if reset {
        COVERAGE_FORCE_RESET_CAPACITY_ERROR.with(|state| state.replace(false))
    } else {
        COVERAGE_FORCE_FINISH_CAPACITY_ERROR.with(|state| state.replace(false))
    };
    if force {
        Err(CapacityError::OutputLengthOverflow)
    } else {
        result
    }
}

#[cfg(not(coverage))]
fn coverage_capacity_result(result: Result<usize, CapacityError>, _reset: bool) -> Result<usize, CapacityError> {
    result
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
    CharsetDecodeError::new(charset, CharsetDecodeErrorKind::OutputLengthOverflow, usize::MAX)
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

#[cfg(test)]
mod tests {
    use qubit_codec::TranscodeDecodeError;
    use qubit_codec::TranscodeDomainError;
    use qubit_codec::TranscodeFailure;
    use qubit_codec_text::Charset;
    use qubit_codec_text::CharsetDecodeError;
    use qubit_codec_text::CharsetDecodeErrorKind;

    use super::map_decode_error;
    use super::map_finish_decode_error;

    #[test]
    fn test_charset_string_decoder_maps_framework_and_finish_errors() {
        let error = map_decode_error(
            Charset::UTF_8,
            TranscodeDecodeError::Failure(TranscodeFailure::insufficient_output(3, 2, 1)),
        );
        assert_eq!(
            CharsetDecodeErrorKind::BufferTooSmall {
                required: 2,
                available: 1,
            },
            error.kind(),
        );
        assert_eq!(3, error.index());

        let invalid_output = map_finish_decode_error(
            Charset::UTF_8,
            TranscodeDecodeError::Failure(TranscodeFailure::invalid_output_index(3, 2)),
            7,
        );
        assert_eq!(
            CharsetDecodeErrorKind::InvalidOutputIndex { output_len: 2 },
            invalid_output.kind()
        );
        assert_eq!(10, invalid_output.index());

        let insufficient_output = map_finish_decode_error(
            Charset::UTF_8,
            TranscodeDecodeError::Failure(TranscodeFailure::insufficient_output(3, 2, 1)),
            7,
        );
        assert_eq!(
            CharsetDecodeErrorKind::BufferTooSmall {
                required: 2,
                available: 1
            },
            insufficient_output.kind()
        );
        assert_eq!(10, insufficient_output.index());

        let invalid_input = map_finish_decode_error(
            Charset::UTF_8,
            TranscodeDecodeError::Failure(TranscodeFailure::invalid_input_index(3, 2)),
            7,
        );
        assert_eq!(
            CharsetDecodeErrorKind::InvalidInputIndex { input_len: 2 },
            invalid_input.kind()
        );
        assert_eq!(3, invalid_input.index());

        let unexpected = map_finish_decode_error(
            Charset::UTF_8,
            TranscodeDecodeError::Failure(TranscodeFailure::TranscodeBeforeReset),
            7,
        );
        assert_eq!(CharsetDecodeErrorKind::TranscodeBeforeReset, unexpected.kind());
        assert_eq!(usize::MAX, unexpected.index());

        let unknown = map_finish_decode_error(
            Charset::UTF_8,
            TranscodeDecodeError::Failure(TranscodeFailure::trailing_input(1, 1)),
            7,
        );
        assert_eq!(CharsetDecodeErrorKind::UnexpectedTranscodeFailure, unknown.kind());
        assert_eq!(usize::MAX, unknown.index());

        let overflow = map_finish_decode_error(
            Charset::UTF_8,
            TranscodeDecodeError::Failure(TranscodeFailure::output_length_overflow()),
            7,
        );
        assert_eq!(CharsetDecodeErrorKind::OutputLengthOverflow, overflow.kind());
        assert_eq!(usize::MAX, overflow.index());

        let domain_error = CharsetDecodeError::new(Charset::ASCII, CharsetDecodeErrorKind::malformed(0xff), 7)
            .with_consumed(core::num::NonZeroUsize::new(2).expect("two is non-zero"));
        let error = map_finish_decode_error(
            Charset::UTF_8,
            TranscodeDecodeError::Domain(TranscodeDomainError::finish(domain_error)),
            300,
        );
        assert_eq!(domain_error, error);
        assert_eq!(Some(0xff), error.kind().value());

        let overflow =
            CharsetDecodeError::new(Charset::UTF_8, CharsetDecodeErrorKind::OutputLengthOverflow, usize::MAX);
        let error = map_finish_decode_error(
            Charset::UTF_8,
            TranscodeDecodeError::Domain(TranscodeDomainError::finish(overflow)),
            7,
        );
        assert_eq!(overflow, error);
    }
}
