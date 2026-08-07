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

use qubit_codec::CapacityError;
use qubit_codec::TranscodeEncodeError;
use qubit_codec::TranscodeStatus;
use qubit_codec::Transcoder;
use qubit_codec_text::Charset;
use qubit_codec_text::CharsetCodec;
use qubit_codec_text::CharsetEncodeError;
use qubit_codec_text::CharsetEncodeErrorKind;
use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::CharsetEncoder;
use qubit_codec_text::UnmappableAction;
use qubit_utils::try_reserve_vec;

const CHAR_CHUNK_CAPACITY: usize = 256;

/// Convenience encoder for complete Rust strings.
///
/// `CharsetStringEncoder` owns a [`CharsetEncoder`] and adapts `&str` input to
/// the `char` slices expected by the transcode layer. Lower-level streaming
/// code should use [`CharsetEncoder`] directly; this type is for closed,
/// in-memory string conversions.
///
/// # Type Parameters
///
/// - `C`: Charset codec used to encode Unicode scalar values into target units.
///
/// # Examples
///
/// ```
/// use qubit_codec_text::Utf8Codec;
/// use qubit_io_text::CharsetStringEncoder;
///
/// let mut encoder = CharsetStringEncoder::new(Utf8Codec);
/// let encoded = encoder.encode_str("中文")?;
/// assert_eq!("中文".as_bytes(), encoded.as_slice());
/// # Ok::<(), qubit_codec_text::CharsetEncodeError>(())
/// ```
pub struct CharsetStringEncoder<C>
where
    C: CharsetCodec,
{
    encoder: CharsetEncoder<C>,
}

impl<C> CharsetStringEncoder<C>
where
    C: CharsetCodec,
{
    /// Creates a string encoder with the charset default replacement policy.
    ///
    /// # Parameters
    ///
    /// - `codec`: Charset codec used to encode output units.
    ///
    /// # Returns
    ///
    /// Returns a string encoder using [`UnmappableAction::Replace`].
    ///
    /// # Panics
    ///
    /// Panics with the same conditions as [`CharsetEncoder::new`]: replacement
    /// mode requires either the default replacement character or the fallback
    /// `?` to be encodable by `codec`.
    #[must_use]
    #[inline]
    pub fn new(codec: C) -> Self {
        Self {
            encoder: CharsetEncoder::new(codec),
        }
    }

    /// Creates a string encoder with an explicit unmappable-input policy.
    ///
    /// # Parameters
    ///
    /// - `codec`: Charset codec used to encode output units.
    /// - `policy`: Unmappable-input policy used by the encoder.
    ///
    /// # Returns
    ///
    /// Returns a string encoder configured with `policy`.
    ///
    /// # Errors
    ///
    /// Returns [`CharsetEncodeError`] when `policy` uses replacement and the
    /// replacement character cannot be encoded by `codec`.
    pub fn with_policy(
        codec: C,
        policy: CharsetEncodePolicy,
    ) -> Result<Self, CharsetEncodeError> {
        Ok(Self {
            encoder: CharsetEncoder::with_policy(codec, policy)?,
        })
    }

    /// Returns the configured unmappable-character action.
    #[must_use]
    #[inline]
    pub const fn unmappable_action(&self) -> UnmappableAction {
        self.encoder.unmappable_action()
    }

    /// Returns the configured replacement character.
    #[must_use]
    #[inline]
    pub const fn replacement(&self) -> char {
        self.encoder.replacement()
    }

    /// Returns the wrapped charset encoder.
    #[must_use]
    #[inline(always)]
    pub const fn encoder(&self) -> &CharsetEncoder<C> {
        &self.encoder
    }

    /// Returns a mutable reference to the wrapped charset encoder.
    #[inline(always)]
    pub fn encoder_mut(&mut self) -> &mut CharsetEncoder<C> {
        &mut self.encoder
    }

    /// Consumes this string encoder and returns the wrapped charset encoder.
    #[must_use]
    #[inline]
    pub fn into_encoder(self) -> CharsetEncoder<C> {
        self.encoder
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

    /// Makes the next capacity-bound operation fail in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_fail_next_capacity_bound() {
        COVERAGE_CAPACITY_FAIL_AFTER.with(|state| state.set(0));
    }

    /// Makes a later capacity-bound operation fail in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_fail_capacity_bound_after(successful_attempts: usize) {
        COVERAGE_CAPACITY_FAIL_AFTER
            .with(|state| state.set(successful_attempts));
    }

    /// Clears coverage-only reserve failure hooks.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_reset_reserve_hooks() {
        COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(usize::MAX));
        COVERAGE_CAPACITY_FAIL_AFTER.with(|state| state.set(usize::MAX));
    }

    /// Maps a transcode error in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_map_encode_error(
        charset: Charset,
        error: TranscodeEncodeError<CharsetEncodeError, char>,
    ) -> CharsetEncodeError {
        map_encode_error(charset, error)
    }

    /// Maps a chunk-local transcode error in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_map_chunk_encode_error(
        charset: Charset,
        error: TranscodeEncodeError<CharsetEncodeError, char>,
        input_offset: usize,
    ) -> CharsetEncodeError {
        map_chunk_encode_error(charset, error, input_offset)
    }

    /// Maps an owned-buffer encoding error in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_map_owned_encode_error(
        charset: Charset,
        error: CharsetEncodeError,
    ) -> CharsetEncodeError {
        map_owned_encode_error(charset, error)
    }

    /// Exercises owned output length arithmetic in coverage builds.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_ensure_owned_capacity(
        charset: Charset,
        output_index: usize,
        required: usize,
    ) -> Result<(), CharsetEncodeError> {
        ensure_owned_capacity(
            &mut Vec::<u8>::new(),
            output_index,
            required,
            charset,
        )
    }

    /// Encodes a complete string into an owned output buffer.
    ///
    /// # Parameters
    ///
    /// - `input`: UTF-8 string whose Unicode scalar values are encoded.
    ///
    /// # Returns
    ///
    /// Returns an owned buffer containing the encoded units.
    ///
    /// # Errors
    ///
    /// Returns [`CharsetEncodeError`] when sizing or reserving the output, or
    /// when encoder reset, transcoding, or finalization fails.
    pub fn encode_str(
        &mut self,
        input: &str,
    ) -> Result<Vec<C::Unit>, CharsetEncodeError>
    where
        C::Unit: Default,
    {
        let charset = self.encoder.charset();
        let mut output = Vec::new();
        let reset_capacity =
            map_capacity_bound(self.encoder.max_reset_output_len(), charset)?;
        ensure_owned_capacity(&mut output, 0, reset_capacity, charset)?;
        let mut output_cursor = match self.encoder.reset(&mut output, 0) {
            Ok(written) => written,
            Err(error) => return Err(map_encode_error(charset, error)),
        };

        let mut chars = input.chars();
        let mut input_offset = 0;
        let mut chunk = ['\0'; CHAR_CHUNK_CAPACITY];
        loop {
            let chunk_len = fill_char_chunk(&mut chars, &mut chunk);
            if chunk_len == 0 {
                break;
            }
            let required = map_capacity_bound(
                self.encoder.max_transcode_output_len(chunk_len),
                charset,
            )?;
            ensure_owned_capacity(
                &mut output,
                output_cursor,
                required,
                charset,
            )?;
            let progress = match self.encoder.transcode(
                &chunk[..chunk_len],
                0,
                &mut output,
                output_cursor,
            ) {
                Ok(progress) => progress,
                Err(error) => {
                    return Err(map_chunk_encode_error(
                        charset,
                        error,
                        input_offset,
                    ));
                }
            };
            output_cursor += progress.written();
            // The pre-sized output uses the encoder's maximum bound, and the
            // transcode engine validates complete consumption.
            input_offset += progress.read();
        }

        let finish_capacity =
            map_capacity_bound(self.encoder.max_finish_output_len(), charset)?;
        ensure_owned_capacity(
            &mut output,
            output_cursor,
            finish_capacity,
            charset,
        )?;
        output_cursor += match self.encoder.finish(&mut output, output_cursor) {
            Ok(written) => written,
            Err(error) => return Err(map_encode_error(charset, error)),
        };
        output.truncate(output_cursor);
        Ok(output)
    }

    /// Encodes a complete string into an existing output slice.
    ///
    /// The encoded stream starts at `output_index`, and the return value is the
    /// number of target units written from that index.
    ///
    /// # Parameters
    ///
    /// - `input`: UTF-8 string whose Unicode scalar values are encoded.
    /// - `output`: Complete output slice visible to the encoder.
    /// - `output_index`: Absolute index where the encoded stream starts.
    ///
    /// # Returns
    ///
    /// Returns the number of units written to `output`.
    ///
    /// # Errors
    ///
    /// Returns [`CharsetEncodeError`] when `output_index` is outside `output`,
    /// the slice cannot hold the complete encoded stream, output sizing
    /// overflows, or encoding fails. When the output is too small, the prefix
    /// written before the failing character remains in `output`.
    pub fn encode_str_into(
        &mut self,
        input: &str,
        output: &mut [C::Unit],
        output_index: usize,
    ) -> Result<usize, CharsetEncodeError> {
        let charset = self.encoder.charset();
        if output_index > output.len() {
            return Err(CharsetEncodeError::new(
                charset,
                CharsetEncodeErrorKind::InvalidOutputIndex {
                    output_len: output.len(),
                },
                output_index,
            ));
        }
        let mut output_cursor = output_index;
        output_cursor += match self.encoder.reset(output, output_cursor) {
            Ok(written) => written,
            Err(error) => return Err(map_encode_error(charset, error)),
        };

        let mut chars = input.chars();
        let mut input_offset = 0;
        let mut chunk = ['\0'; CHAR_CHUNK_CAPACITY];
        loop {
            let chunk_len = fill_char_chunk(&mut chars, &mut chunk);
            if chunk_len == 0 {
                break;
            }
            map_capacity_bound(
                self.encoder.max_transcode_output_len(chunk_len),
                charset,
            )?;
            let progress = match self.encoder.transcode(
                &chunk[..chunk_len],
                0,
                output,
                output_cursor,
            ) {
                Ok(progress) => progress,
                Err(error) => {
                    return Err(map_chunk_encode_error(
                        charset,
                        error,
                        input_offset,
                    ));
                }
            };
            output_cursor += progress.written();
            if let TranscodeStatus::NeedOutput { required } = progress.status()
            {
                let available = output.len().saturating_sub(output_cursor);
                return Err(CharsetEncodeError::new(
                    charset,
                    CharsetEncodeErrorKind::BufferTooSmall {
                        required: required.get(),
                        available,
                    },
                    output_cursor,
                ));
            }
            input_offset += progress.read();
        }

        map_capacity_bound(self.encoder.max_finish_output_len(), charset)?;
        output_cursor += match self.encoder.finish(output, output_cursor) {
            Ok(written) => written,
            Err(error) => return Err(map_encode_error(charset, error)),
        };
        Ok(output_cursor - output_index)
    }
}

/// Fills one bounded character chunk from a UTF-8 scalar iterator.
///
/// # Parameters
///
/// - `chars`: Iterator supplying the remaining Unicode scalar values.
/// - `chunk`: Fixed-size destination for the next scalar window.
///
/// # Returns
///
/// Returns the number of initialized entries in `chunk`.
fn fill_char_chunk(
    chars: &mut impl Iterator<Item = char>,
    chunk: &mut [char; CHAR_CHUNK_CAPACITY],
) -> usize {
    let mut len = 0;
    while len < chunk.len() {
        let Some(ch) = chars.next() else {
            break;
        };
        chunk[len] = ch;
        len += 1;
    }
    len
}

/// Ensures that an owned output exposes a writable range at `output_index`.
///
/// # Parameters
///
/// - `output`: Owned unit buffer to grow and initialize.
/// - `output_index`: Start of the writable range.
/// - `required`: Number of units that must be writable from `output_index`.
/// - `charset`: Charset attached to any reported error.
///
/// # Errors
///
/// Returns [`CharsetEncodeErrorKind::OutputLengthOverflow`] when length
/// arithmetic overflows or the allocation cannot be reserved.
fn ensure_owned_capacity<T>(
    output: &mut Vec<T>,
    output_index: usize,
    required: usize,
    charset: Charset,
) -> Result<(), CharsetEncodeError>
where
    T: Default,
{
    let Some(required_len) = output_index.checked_add(required) else {
        return Err(output_length_overflow(charset));
    };
    if required_len <= output.len() {
        return Ok(());
    }
    let additional = required_len - output.len();
    let reserve_failed = try_reserve_vec(output, additional).is_err();
    #[cfg(coverage)]
    let reserve_failed = reserve_failed || coverage_should_fail_reserve();
    if reserve_failed {
        return Err(output_length_overflow(charset));
    }
    output.resize_with(required_len, T::default);
    Ok(())
}

/// Maps a transcode output-bound result into the charset error model.
///
/// # Parameters
///
/// - `result`: Capacity bound reported by the transcode layer.
/// - `charset`: Charset attached to a capacity failure.
///
/// # Returns
///
/// Returns the reported bound when successful.
///
/// # Errors
///
/// Returns [`CharsetEncodeErrorKind::OutputLengthOverflow`] when the bound
/// cannot be represented.
fn map_capacity_bound(
    result: Result<usize, CapacityError>,
    charset: Charset,
) -> Result<usize, CharsetEncodeError> {
    #[cfg(coverage)]
    if coverage_should_fail_capacity_bound() {
        return Err(output_length_overflow(charset));
    }
    match result {
        Ok(capacity) => Ok(capacity),
        Err(_) => Err(output_length_overflow(charset)),
    }
}

/// Converts a transcode-layer encode error into the charset error model.
///
/// # Parameters
///
/// - `charset`: Charset attached to framework-level failures.
/// - `error`: Error returned by the transcode layer.
///
/// # Returns
///
/// Returns the corresponding charset encoding error.
fn map_encode_error(
    charset: Charset,
    error: TranscodeEncodeError<CharsetEncodeError, char>,
) -> CharsetEncodeError {
    CharsetEncodeError::from_transcode_error(charset, error)
}

/// Maps a chunk-local unencodable index to the complete string index.
///
/// # Parameters
///
/// - `charset`: Charset attached to framework-level failures.
/// - `error`: Error whose input index is relative to the current chunk.
/// - `input_offset`: Character offset of the current chunk in the string.
///
/// # Returns
///
/// Returns an error whose input index is relative to the complete string.
fn map_chunk_encode_error(
    charset: Charset,
    error: TranscodeEncodeError<CharsetEncodeError, char>,
    input_offset: usize,
) -> CharsetEncodeError {
    let error = match error {
        TranscodeEncodeError::Unencodable { input_index, value } => {
            let input_index = input_offset.saturating_add(input_index);
            return CharsetEncodeError::map_unencodable(
                charset,
                input_index,
                value,
            );
        }
        error => map_encode_error(charset, error),
    };
    match error.kind() {
        CharsetEncodeErrorKind::InvalidOutputIndex { .. }
        | CharsetEncodeErrorKind::BufferTooSmall { .. }
        | CharsetEncodeErrorKind::OutputLengthOverflow => error,
        kind => CharsetEncodeError::new(
            charset,
            kind,
            input_offset.saturating_add(error.index()),
        ),
    }
}

/// Maps an impossible owned-buffer capacity miss to an overflow error.
///
/// # Parameters
///
/// - `charset`: Charset attached to the replacement overflow error.
/// - `error`: Error produced while using a pre-sized owned buffer.
///
/// # Returns
///
/// Returns an output-length overflow for an impossible capacity miss, or the
/// original error for every other error kind.
#[cfg(coverage)]
fn map_owned_encode_error(
    charset: Charset,
    error: CharsetEncodeError,
) -> CharsetEncodeError {
    if matches!(error.kind(), CharsetEncodeErrorKind::BufferTooSmall { .. }) {
        output_length_overflow(charset)
    } else {
        error
    }
}

#[cfg(coverage)]
thread_local! {
    static COVERAGE_RESERVE_FAIL_AFTER: Cell<usize> = const { Cell::new(usize::MAX) };
    static COVERAGE_CAPACITY_FAIL_AFTER: Cell<usize> = const { Cell::new(usize::MAX) };
}

/// Reports whether a synthetic capacity-bound failure was requested.
#[cfg(coverage)]
fn coverage_should_fail_capacity_bound() -> bool {
    COVERAGE_CAPACITY_FAIL_AFTER.with(|state| {
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
fn output_length_overflow(charset: Charset) -> CharsetEncodeError {
    CharsetEncodeError::new(
        charset,
        CharsetEncodeErrorKind::OutputLengthOverflow,
        usize::MAX,
    )
}
