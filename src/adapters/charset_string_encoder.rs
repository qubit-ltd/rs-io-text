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
    CapacityError,
    TranscodeError,
    TranscodeStatus,
    Transcoder,
};
use qubit_codec_text::{
    CharsetCodec,
    CharsetEncodeError,
    CharsetEncodePolicy,
    CharsetEncoder,
    UnmappableAction,
};
use qubit_io::try_reserve_vec;

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

    /// Clears coverage-only reserve failure hooks.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_reset_reserve_hooks() {
        COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(usize::MAX));
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
    /// Returns [`TranscodeError`] when collecting input characters, sizing the
    /// output, reset, encoding, or finish fails.
    pub fn encode_str(
        &mut self,
        input: &str,
    ) -> Result<Vec<C::Unit>, TranscodeError<CharsetEncodeError>>
    where
        C::Unit: Default,
    {
        let chars = collect_chars(input)?;
        let capacity = self
            .required_encode_output_len(chars.len())
            .map_err(TranscodeError::from)?;
        let mut output = Vec::new();
        let reserve_failed = try_reserve_vec(&mut output, capacity).is_err();
        #[cfg(coverage)]
        let reserve_failed = reserve_failed || coverage_should_fail_reserve();
        if reserve_failed {
            return Err(TranscodeError::output_length_overflow());
        }
        output.resize_with(capacity, C::Unit::default);
        let written = self.encode_chars_into(&chars, &mut output, 0).map_err(
            |error| {
                if matches!(error, TranscodeError::InsufficientOutput { .. }) {
                    TranscodeError::output_length_overflow()
                } else {
                    error
                }
            },
        )?;
        output.truncate(written);
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
    /// Returns [`TranscodeError::InvalidOutputIndex`] when `output_index` is
    /// outside `output`, [`TranscodeError::InsufficientOutput`] when the slice
    /// cannot hold the complete encoded stream, or [`TranscodeError::Domain`]
    /// when reset, encoding, or finish fails.
    pub fn encode_str_into(
        &mut self,
        input: &str,
        output: &mut [C::Unit],
        output_index: usize,
    ) -> Result<usize, TranscodeError<CharsetEncodeError>> {
        let chars = collect_chars(input)?;
        self.encode_chars_into(&chars, output, output_index)
    }

    /// Encodes a complete character slice into an existing output slice.
    ///
    /// # Parameters
    ///
    /// - `input`: Complete character slice.
    /// - `output`: Target output units.
    /// - `output_index`: Absolute output index where encoding starts.
    ///
    /// # Returns
    ///
    /// Returns the number of units written from `output_index`.
    ///
    /// # Errors
    ///
    /// Returns transcode framework or charset errors from reset, transcode, or
    /// finish.
    fn encode_chars_into(
        &mut self,
        input: &[char],
        output: &mut [C::Unit],
        output_index: usize,
    ) -> Result<usize, TranscodeError<CharsetEncodeError>> {
        TranscodeError::ensure_output_index(output.len(), output_index)?;
        let required = self
            .required_encode_output_len(input.len())
            .map_err(TranscodeError::from)?;
        TranscodeError::ensure_output_capacity(
            output.len(),
            output_index,
            required,
        )?;

        let mut output_cursor = output_index;
        output_cursor += self.encoder.reset(output, output_cursor)?;
        let progress =
            self.encoder.transcode(input, 0, output, output_cursor)?;
        output_cursor += progress.written();
        if let TranscodeStatus::NeedOutput {
            output_index,
            required,
            available,
        } = progress.status()
        {
            return Err(TranscodeError::insufficient_output(
                output_index,
                required.get(),
                available,
            ));
        }
        output_cursor += self.encoder.finish(output, output_cursor)?;
        Ok(output_cursor - output_index)
    }

    /// Returns the maximum encoded output units for a complete string.
    ///
    /// # Parameters
    ///
    /// - `input_len`: Number of input characters.
    ///
    /// # Returns
    ///
    /// Returns the reset, stream, and finish output upper bound.
    ///
    /// # Errors
    ///
    /// Returns [`CapacityError`] when any component bound overflows.
    fn required_encode_output_len(
        &self,
        input_len: usize,
    ) -> Result<usize, CapacityError> {
        self.encoder.max_total_output_len(input_len)
    }
}

#[cfg(coverage)]
thread_local! {
    static COVERAGE_RESERVE_FAIL_AFTER: Cell<usize> = const { Cell::new(usize::MAX) };
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

/// Collects a UTF-8 string into the character slice representation expected by
/// the transcode layer.
///
/// # Parameters
///
/// - `input`: UTF-8 source string.
///
/// # Returns
///
/// Returns all Unicode scalar values from `input`.
///
/// # Errors
///
/// Returns [`TranscodeError::OutputLengthOverflow`] when the input character
/// buffer cannot be reserved.
fn collect_chars(
    input: &str,
) -> Result<Vec<char>, TranscodeError<CharsetEncodeError>> {
    let char_count = input.chars().count();
    let mut chars = Vec::new();
    let reserve_failed = try_reserve_vec(&mut chars, char_count).is_err();
    #[cfg(coverage)]
    let reserve_failed = reserve_failed || coverage_should_fail_reserve();
    if reserve_failed {
        return Err(TranscodeError::output_length_overflow());
    }
    chars.extend(input.chars());
    Ok(chars)
}
