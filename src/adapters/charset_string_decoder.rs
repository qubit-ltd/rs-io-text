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

use qubit_codec::{CapacityError, TranscodeError, TranscodeStatus, Transcoder};
use qubit_codec_text::{
    Charset, CharsetCodec, CharsetDecodeError, CharsetDecodeErrorKind, CharsetDecodePolicy,
    CharsetDecoder, MalformedAction,
};
use qubit_io::{try_reserve_string, try_reserve_vec};

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
        COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(successful_attempts));
    }

    /// Clears coverage-only reserve failure hooks.
    #[cfg(coverage)]
    #[doc(hidden)]
    pub fn coverage_reset_reserve_hooks() {
        COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(usize::MAX));
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
    /// Returns [`TranscodeError`] when decoding fails, output sizing overflows,
    /// or the complete input ends with an incomplete sequence.
    pub fn decode_to_string(
        &mut self,
        input: &[C::Unit],
    ) -> Result<String, TranscodeError<CharsetDecodeError>> {
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
    /// Returns [`TranscodeError::InvalidInputIndex`] when `input_index` is
    /// outside `input`, [`TranscodeError::Domain`] when decoding fails, or
    /// [`TranscodeError::OutputLengthOverflow`] when output sizing overflows.
    pub fn decode_to_string_into(
        &mut self,
        input: &[C::Unit],
        input_index: usize,
        output: &mut String,
    ) -> Result<(), TranscodeError<CharsetDecodeError>> {
        TranscodeError::ensure_input_index(input.len(), input_index)?;
        let input_len = input.len() - input_index;
        let char_capacity = self
            .required_char_output_len(input_len)
            .map_err(TranscodeError::from)?;
        let mut chars = Vec::new();
        let reserve_failed = try_reserve_vec(&mut chars, char_capacity).is_err();
        #[cfg(coverage)]
        let reserve_failed = reserve_failed || coverage_should_fail_reserve();
        if reserve_failed {
            return Err(TranscodeError::output_length_overflow());
        }
        chars.resize(char_capacity, '\0');
        let written = self.decode_units_into(input, input_index, &mut chars)?;
        let byte_capacity = required_string_capacity(&chars[..written]);
        let reserve_failed = try_reserve_string(output, byte_capacity).is_err();
        #[cfg(coverage)]
        let reserve_failed = reserve_failed || coverage_should_fail_reserve();
        if reserve_failed {
            return Err(TranscodeError::output_length_overflow());
        }
        output.extend(chars[..written].iter());
        Ok(())
    }

    /// Returns the maximum decoded character count for a complete input.
    ///
    /// # Parameters
    ///
    /// - `input_len`: Number of input units decoded by the string helper.
    ///
    /// # Returns
    ///
    /// Returns the reset, stream, and finish character upper bound.
    ///
    /// # Errors
    ///
    /// Returns [`CapacityError`] when char-count arithmetic overflows.
    fn required_char_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        self.decoder.max_total_output_len(input_len)
    }

    /// Decodes a complete unit slice into a caller-provided character buffer.
    ///
    /// # Parameters
    ///
    /// - `input`: Complete input slice visible to the decoder.
    /// - `input_index`: Absolute index where decoding starts.
    /// - `output`: Character buffer receiving decoded values.
    ///
    /// # Returns
    ///
    /// Returns the number of decoded characters written to `output`.
    ///
    /// # Errors
    ///
    /// Returns framework or charset errors from reset, transcode, or finish.
    fn decode_units_into(
        &mut self,
        input: &[C::Unit],
        input_index: usize,
        output: &mut [char],
    ) -> Result<usize, TranscodeError<CharsetDecodeError>> {
        let mut output_cursor = self.decoder.reset(output, 0)?;
        let progress = self
            .decoder
            .transcode(input, input_index, output, output_cursor)?;
        output_cursor += progress.written();
        if let TranscodeStatus::NeedInput {
            input_index,
            required,
            available,
        } = progress.status()
        {
            let kind = CharsetDecodeErrorKind::IncompleteSequence {
                required: required.get(),
                available,
            };
            return Err(TranscodeError::Domain(CharsetDecodeError::new(
                self.charset,
                kind,
                input_index,
            )));
        }
        output_cursor += self.decoder.finish(output, output_cursor)?;
        Ok(output_cursor)
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
