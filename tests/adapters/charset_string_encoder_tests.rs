// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use core::num::NonZeroUsize;

use qubit_codec::Codec;
use qubit_codec_text::Utf8Codec;
use qubit_codec_text::{
    AsciiCodec, Charset, CharsetCodec, CharsetDecodeError, CharsetDecodeErrorKind,
    CharsetEncodeError, CharsetEncodeErrorKind, CharsetEncodePolicy, CharsetEncodeResult,
    UnmappableAction,
};
use qubit_io_text::CharsetStringEncoder;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NonDefaultUnit(u8);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NonDefaultUnitCodec;

impl CharsetCodec for NonDefaultUnitCodec {
    fn charset(&self) -> Charset {
        Charset::ASCII
    }
}

impl Codec for NonDefaultUnitCodec {
    type Value = char;
    type Unit = NonDefaultUnit;
    type DecodeError = CharsetDecodeError;
    type EncodeError = CharsetEncodeError;

    const MIN_UNITS_PER_VALUE: usize = 1;

    const MAX_ENCODE_UNITS_PER_VALUE: usize = 1;

    const MAX_DECODE_UNITS_PER_VALUE: usize = 1;

    fn can_encode_value(&self, value: &char) -> bool {
        value.is_ascii()
    }

    unsafe fn decode(
        &mut self,
        _input: &[NonDefaultUnit],
        input_index: usize,
    ) -> Result<(char, NonZeroUsize), qubit_codec::DecodeFailure<Self::DecodeError>> {
        let kind = CharsetDecodeErrorKind::malformed_unknown();
        Err(CharsetDecodeError::new(Charset::ASCII, kind, input_index).into_codec_failure())
    }

    unsafe fn encode(
        &mut self,
        value: &char,
        output: &mut [NonDefaultUnit],
        output_index: usize,
    ) -> CharsetEncodeResult<usize> {
        output[output_index] = NonDefaultUnit(*value as u8);
        Ok(1)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct UnderreportedEncodeLenCodec;

#[derive(Clone, Copy, Debug, Default)]
struct HugeEncodeBoundsCodec;

#[derive(Clone, Copy, Debug, Default)]
struct EncodeResetErrorCodec;

#[derive(Clone, Copy, Debug, Default)]
struct EncodeFlushErrorCodec;

impl CharsetCodec for HugeEncodeBoundsCodec {
    fn charset(&self) -> Charset {
        Charset::ASCII
    }
}

impl Codec for HugeEncodeBoundsCodec {
    type Value = char;
    type Unit = u8;
    type DecodeError = CharsetDecodeError;
    type EncodeError = CharsetEncodeError;

    const MIN_UNITS_PER_VALUE: usize = 1;

    const MAX_ENCODE_UNITS_PER_VALUE: usize = usize::MAX;

    const MAX_DECODE_UNITS_PER_VALUE: usize = usize::MAX;

    fn can_encode_value(&self, value: &char) -> bool {
        value.is_ascii()
    }

    unsafe fn decode(
        &mut self,
        _input: &[u8],
        input_index: usize,
    ) -> Result<(char, NonZeroUsize), qubit_codec::DecodeFailure<Self::DecodeError>> {
        let kind = CharsetDecodeErrorKind::malformed_unknown();
        Err(CharsetDecodeError::new(Charset::ASCII, kind, input_index).into_codec_failure())
    }

    unsafe fn encode(
        &mut self,
        value: &char,
        output: &mut [u8],
        output_index: usize,
    ) -> CharsetEncodeResult<usize> {
        output[output_index] = *value as u8;
        Ok(1)
    }
}

impl CharsetCodec for UnderreportedEncodeLenCodec {
    fn charset(&self) -> Charset {
        Charset::ASCII
    }
}

impl CharsetCodec for EncodeResetErrorCodec {
    fn charset(&self) -> Charset {
        Charset::ASCII
    }
}

impl CharsetCodec for EncodeFlushErrorCodec {
    fn charset(&self) -> Charset {
        Charset::ASCII
    }
}

impl Codec for UnderreportedEncodeLenCodec {
    type Value = char;
    type Unit = u8;
    type DecodeError = CharsetDecodeError;
    type EncodeError = CharsetEncodeError;

    const MIN_UNITS_PER_VALUE: usize = 1;

    const MAX_ENCODE_UNITS_PER_VALUE: usize = 1;

    const MAX_DECODE_UNITS_PER_VALUE: usize = 1;

    fn can_encode_value(&self, value: &char) -> bool {
        value.is_ascii()
    }

    fn encode_len(&self, _value: &char) -> usize {
        2
    }

    unsafe fn decode(
        &mut self,
        _input: &[u8],
        input_index: usize,
    ) -> Result<(char, NonZeroUsize), qubit_codec::DecodeFailure<Self::DecodeError>> {
        let kind = CharsetDecodeErrorKind::malformed_unknown();
        Err(CharsetDecodeError::new(Charset::ASCII, kind, input_index).into_codec_failure())
    }

    unsafe fn encode(
        &mut self,
        value: &char,
        output: &mut [u8],
        output_index: usize,
    ) -> CharsetEncodeResult<usize> {
        output[output_index] = *value as u8;
        output[output_index + 1] = *value as u8;
        Ok(2)
    }
}

impl Codec for EncodeResetErrorCodec {
    type Value = char;
    type Unit = u8;
    type DecodeError = CharsetDecodeError;
    type EncodeError = CharsetEncodeError;

    const MIN_UNITS_PER_VALUE: usize = 1;

    const MAX_ENCODE_UNITS_PER_VALUE: usize = 1;

    const MAX_DECODE_UNITS_PER_VALUE: usize = 1;

    const MAX_ENCODE_RESET_UNITS: usize = 1;

    fn can_encode_value(&self, value: &char) -> bool {
        value.is_ascii()
    }

    unsafe fn decode(
        &mut self,
        _input: &[u8],
        input_index: usize,
    ) -> Result<(char, NonZeroUsize), qubit_codec::DecodeFailure<Self::DecodeError>> {
        let kind = CharsetDecodeErrorKind::malformed_unknown();
        Err(CharsetDecodeError::new(Charset::ASCII, kind, input_index).into_codec_failure())
    }

    unsafe fn encode_reset(
        &mut self,
        _output: &mut [u8],
        output_index: usize,
    ) -> CharsetEncodeResult<usize> {
        let kind = CharsetEncodeErrorKind::UnmappableCharacter {
            value: output_index as u32,
        };
        Err(CharsetEncodeError::new(Charset::ASCII, kind, output_index))
    }

    unsafe fn encode(
        &mut self,
        value: &char,
        output: &mut [u8],
        output_index: usize,
    ) -> CharsetEncodeResult<usize> {
        output[output_index] = *value as u8;
        Ok(1)
    }
}

impl Codec for EncodeFlushErrorCodec {
    type Value = char;
    type Unit = u8;
    type DecodeError = CharsetDecodeError;
    type EncodeError = CharsetEncodeError;

    const MIN_UNITS_PER_VALUE: usize = 1;

    const MAX_ENCODE_UNITS_PER_VALUE: usize = 1;

    const MAX_DECODE_UNITS_PER_VALUE: usize = 1;

    const MAX_ENCODE_FINISH_UNITS: usize = 1;

    fn can_encode_value(&self, value: &char) -> bool {
        value.is_ascii()
    }

    unsafe fn decode(
        &mut self,
        _input: &[u8],
        input_index: usize,
    ) -> Result<(char, NonZeroUsize), qubit_codec::DecodeFailure<Self::DecodeError>> {
        let kind = CharsetDecodeErrorKind::malformed_unknown();
        Err(CharsetDecodeError::new(Charset::ASCII, kind, input_index).into_codec_failure())
    }

    unsafe fn encode(
        &mut self,
        value: &char,
        output: &mut [u8],
        output_index: usize,
    ) -> CharsetEncodeResult<usize> {
        output[output_index] = *value as u8;
        Ok(1)
    }

    unsafe fn encode_finish(
        &mut self,
        _output: &mut [u8],
        output_index: usize,
    ) -> CharsetEncodeResult<usize> {
        let kind = CharsetEncodeErrorKind::UnmappableCharacter {
            value: output_index as u32,
        };
        Err(CharsetEncodeError::new(Charset::ASCII, kind, output_index))
    }
}

#[test]
fn test_charset_string_encoder_encode_str_encodes_complete_string() {
    let mut encoder = CharsetStringEncoder::new(Utf8Codec);

    let output = encoder
        .encode_str("A中")
        .expect("UTF-8 encoder should encode complete string");

    assert_eq!("A中".as_bytes(), output.as_slice());
}

#[test]
fn test_charset_string_encoder_exposes_configuration_and_wrapped_encoder() {
    let mut encoder = CharsetStringEncoder::with_policy(
        AsciiCodec,
        CharsetEncodePolicy::ignore_with_replacement('!'),
    )
    .expect("ASCII replacement should be encodable");

    assert_eq!(UnmappableAction::Ignore, encoder.unmappable_action());
    assert_eq!('!', encoder.replacement());
    assert_eq!(
        UnmappableAction::Ignore,
        encoder.encoder().unmappable_action()
    );
    assert_eq!('!', encoder.encoder_mut().replacement());

    let inner = encoder.into_encoder();
    assert_eq!(UnmappableAction::Ignore, inner.unmappable_action());
    assert_eq!('!', inner.replacement());
}

#[test]
fn test_charset_string_encoder_with_policy_rejects_unencodable_replacement() {
    let result = CharsetStringEncoder::with_policy(AsciiCodec, CharsetEncodePolicy::replace('中'));
    let Err(error) = result else {
        panic!("unencodable replacement should be rejected");
    };

    assert_eq!(
        CharsetEncodeErrorKind::UnmappableCharacter {
            value: '中' as u32
        },
        error.kind()
    );
    assert_eq!(0, error.index());
}

#[test]
fn test_charset_string_encoder_encode_str_into_writes_at_output_index() {
    let mut encoder = CharsetStringEncoder::new(Utf8Codec);
    let mut output = [0xff_u8; 10];

    let written = encoder
        .encode_str_into("A中", &mut output, 1)
        .expect("UTF-8 encoder should write into caller slice");

    assert_eq!("A中".len(), written);
    assert_eq!(0xff, output[0]);
    assert_eq!("A中".as_bytes(), &output[1..1 + written]);
    assert_eq!(0xff, output[1 + written]);
}

#[test]
fn test_charset_string_encoder_encode_str_into_accepts_actual_utf8_capacity() {
    let mut encoder = CharsetStringEncoder::new(Utf8Codec);
    let mut output = [0_u8; 3];

    let written = encoder
        .encode_str_into("ABC", &mut output, 0)
        .expect("the actual UTF-8 output fits the caller slice");

    assert_eq!(3, written);
    assert_eq!(b"ABC", &output);
}

#[test]
fn test_charset_string_encoder_encode_str_crosses_character_windows() {
    let mut encoder = CharsetStringEncoder::new(Utf8Codec);
    let input = "A中🙂".repeat(100);

    let output = encoder
        .encode_str(&input)
        .expect("UTF-8 encoder should cross bounded character windows");

    assert_eq!(input.as_bytes(), output.as_slice());
}

#[test]
fn test_charset_string_encoder_encode_str_into_supports_non_default_units() {
    let mut encoder =
        CharsetStringEncoder::with_policy(NonDefaultUnitCodec, CharsetEncodePolicy::report())
            .expect("report policy does not require replacement units");
    let mut output = [NonDefaultUnit(0), NonDefaultUnit(0)];

    let written = encoder
        .encode_str_into("A", &mut output, 1)
        .expect("ASCII character should encode into non-default unit slice");

    assert_eq!(1, written);
    assert_eq!(NonDefaultUnit(b'A'), output[1]);
}

#[test]
fn test_charset_string_encoder_encode_str_into_reports_invalid_output_index() {
    let mut encoder = CharsetStringEncoder::new(Utf8Codec);
    let mut output = [0_u8; 1];

    let error = encoder
        .encode_str_into("A", &mut output, 2)
        .expect_err("output index outside slice should be rejected");

    assert_eq!(
        CharsetEncodeErrorKind::InvalidOutputIndex { output_len: 1 },
        error.kind(),
    );
    assert_eq!(2, error.index());
}

#[test]
fn test_charset_string_encoder_encode_str_into_reports_insufficient_output() {
    let mut encoder = CharsetStringEncoder::new(Utf8Codec);
    let mut output = [0_u8; 3];

    let error = encoder
        .encode_str_into("A中", &mut output, 0)
        .expect_err("caller slice is too small for complete output");

    assert_eq!(
        CharsetEncodeErrorKind::BufferTooSmall {
            required: "中".len(),
            available: output.len() - 1,
        },
        error.kind(),
    );
    assert_eq!(1, error.index());
    assert_eq!(b'A', output[0]);
}

#[test]
fn test_charset_string_encoder_encode_str_into_reports_unmappable_input() {
    let mut encoder = CharsetStringEncoder::with_policy(AsciiCodec, CharsetEncodePolicy::report())
        .expect("report policy should be constructible");
    let mut output = [0_u8; 1];

    let error = encoder
        .encode_str_into("é", &mut output, 0)
        .expect_err("strict ASCII should reject non-ASCII input");

    assert_eq!(
        CharsetEncodeErrorKind::UnmappableCharacter { value: 'é' as u32 },
        error.kind(),
    );
    assert_eq!(0, error.index());
}

#[test]
#[should_panic(expected = "Codec::encode_len exceeded Codec::MAX_ENCODE_UNITS_PER_VALUE")]
fn test_charset_string_encoder_encode_str_panics_when_encode_len_exceeds_codec_maximum() {
    let mut encoder = CharsetStringEncoder::new(UnderreportedEncodeLenCodec);

    let _ = encoder.encode_str("A");
}

#[test]
fn test_charset_string_encoder_encode_str_reports_output_reserve_overflow() {
    let mut encoder = CharsetStringEncoder::new(HugeEncodeBoundsCodec);

    let error = encoder
        .encode_str("A")
        .expect_err("huge output bound should fail output reservation");

    assert_eq!(CharsetEncodeErrorKind::OutputLengthOverflow, error.kind());
}

#[test]
fn test_charset_string_encoder_encode_str_reports_output_capacity_overflow() {
    let mut encoder = CharsetStringEncoder::new(HugeEncodeBoundsCodec);

    let error = encoder
        .encode_str("AA")
        .expect_err("huge output bound should overflow capacity arithmetic");

    assert_eq!(CharsetEncodeErrorKind::OutputLengthOverflow, error.kind());
}

#[test]
fn test_charset_string_encoder_encode_str_into_reports_output_capacity_overflow() {
    let mut encoder = CharsetStringEncoder::new(HugeEncodeBoundsCodec);
    let mut output = [0_u8; 1];

    let error = encoder
        .encode_str_into("AA", &mut output, 0)
        .expect_err("huge output bound should overflow capacity arithmetic");

    assert_eq!(CharsetEncodeErrorKind::OutputLengthOverflow, error.kind());
}

#[test]
fn test_charset_string_encoder_encode_str_propagates_reset_errors() {
    let mut encoder = CharsetStringEncoder::new(EncodeResetErrorCodec);

    let error = encoder
        .encode_str("A")
        .expect_err("encode reset errors should be propagated");

    assert_eq!(
        CharsetEncodeErrorKind::UnmappableCharacter { value: 0 },
        error.kind(),
    );
    assert_eq!(0, error.index());
}

#[test]
fn test_charset_string_encoder_encode_str_into_propagates_reset_errors() {
    let mut encoder = CharsetStringEncoder::new(EncodeResetErrorCodec);
    let mut output = [0_u8; 1];

    let error = encoder
        .encode_str_into("", &mut output, 0)
        .expect_err("caller-buffer reset errors should be propagated");

    assert_eq!(
        CharsetEncodeErrorKind::UnmappableCharacter { value: 0 },
        error.kind(),
    );
}

#[test]
fn test_charset_string_encoder_encode_str_propagates_finish_errors() {
    let mut encoder = CharsetStringEncoder::new(EncodeFlushErrorCodec);

    let error = encoder
        .encode_str("A")
        .expect_err("encode flush errors should be propagated");

    assert_eq!(
        CharsetEncodeErrorKind::UnmappableCharacter { value: 1 },
        error.kind(),
    );
    assert_eq!(1, error.index());
}

#[test]
fn test_charset_string_encoder_encode_str_into_propagates_finish_errors() {
    let mut encoder = CharsetStringEncoder::new(EncodeFlushErrorCodec);
    let mut output = [0_u8; 2];

    let error = encoder
        .encode_str_into("A", &mut output, 0)
        .expect_err("caller-buffer finish errors should be propagated");

    assert_eq!(
        CharsetEncodeErrorKind::UnmappableCharacter { value: 1 },
        error.kind(),
    );
}

#[test]
fn test_charset_string_encoder_encode_str_reports_global_error_index() {
    let mut encoder = CharsetStringEncoder::with_policy(AsciiCodec, CharsetEncodePolicy::report())
        .expect("report policy should be constructible");
    let input = format!("{}é", "A".repeat(300));

    let error = encoder
        .encode_str(&input)
        .expect_err("encode error should report global character index");

    assert_eq!(
        CharsetEncodeErrorKind::UnmappableCharacter { value: 'é' as u32 },
        error.kind(),
    );
    assert_eq!(300, error.index());
}

#[test]
fn test_charset_string_encoder_encode_str_applies_default_policy() {
    let mut encoder = CharsetStringEncoder::new(AsciiCodec);

    let output = encoder
        .encode_str("A中")
        .expect("ASCII encoder should replace unmappable character");

    assert_eq!(b"A?", output.as_slice());
}

#[cfg(coverage)]
mod coverage_tests {
    use super::{EncodeFlushErrorCodec, EncodeResetErrorCodec};

    use qubit_codec::{TranscodeDomainError, TranscodeEncodeError};
    use qubit_codec_text::Charset;
    use qubit_codec_text::CharsetEncodeErrorKind;
    use qubit_codec_text::Utf8Codec;
    use qubit_io_text::CharsetStringEncoder;

    fn reset_coverage_hooks() {
        CharsetStringEncoder::<Utf8Codec>::coverage_reset_reserve_hooks();
    }

    #[test]
    fn test_charset_string_encoder_reports_owned_output_reserve_failure() {
        reset_coverage_hooks();
        let mut encoder = CharsetStringEncoder::new(Utf8Codec);

        CharsetStringEncoder::<Utf8Codec>::coverage_fail_next_reserve();
        let error = encoder
            .encode_str("A")
            .expect_err("owned output reserve failure should be reported");

        assert_eq!(CharsetEncodeErrorKind::OutputLengthOverflow, error.kind());
        reset_coverage_hooks();
    }

    #[test]
    fn test_charset_string_encoder_uses_one_reserve_for_small_output() {
        reset_coverage_hooks();
        let mut encoder = CharsetStringEncoder::new(Utf8Codec);

        CharsetStringEncoder::<Utf8Codec>::coverage_fail_reserve_after(1);
        let output = encoder
            .encode_str("A")
            .expect("no second reserve should occur for a small output");

        assert_eq!(b"A", output.as_slice());
        reset_coverage_hooks();
    }

    #[test]
    fn test_charset_string_encoder_reports_finish_output_reserve_failure() {
        reset_coverage_hooks();
        let mut encoder = CharsetStringEncoder::new(EncodeFlushErrorCodec);

        CharsetStringEncoder::<Utf8Codec>::coverage_fail_reserve_after(1);
        let error = encoder
            .encode_str("A")
            .expect_err("finish output reserve failure should be reported");

        assert_eq!(CharsetEncodeErrorKind::OutputLengthOverflow, error.kind());
        reset_coverage_hooks();
    }

    #[test]
    fn test_charset_string_encoder_reports_reset_output_reserve_failure() {
        reset_coverage_hooks();
        let mut encoder = CharsetStringEncoder::new(EncodeResetErrorCodec);

        CharsetStringEncoder::<Utf8Codec>::coverage_fail_next_reserve();
        let error = encoder
            .encode_str("")
            .expect_err("reset output reserve failure should be reported");

        assert_eq!(CharsetEncodeErrorKind::OutputLengthOverflow, error.kind());
        reset_coverage_hooks();
    }

    #[test]
    fn test_charset_string_encoder_reports_lifecycle_capacity_failures() {
        reset_coverage_hooks();
        let mut encoder = CharsetStringEncoder::new(Utf8Codec);

        CharsetStringEncoder::<Utf8Codec>::coverage_fail_next_capacity_bound();
        let error = encoder
            .encode_str("")
            .expect_err("reset capacity failure should be reported");
        assert_eq!(CharsetEncodeErrorKind::OutputLengthOverflow, error.kind());

        let mut encoder = CharsetStringEncoder::new(Utf8Codec);
        CharsetStringEncoder::<Utf8Codec>::coverage_fail_capacity_bound_after(1);
        let error = encoder
            .encode_str("")
            .expect_err("finish capacity failure should be reported");
        assert_eq!(CharsetEncodeErrorKind::OutputLengthOverflow, error.kind());

        let mut encoder = CharsetStringEncoder::new(Utf8Codec);
        let mut output = [];
        CharsetStringEncoder::<Utf8Codec>::coverage_fail_next_capacity_bound();
        let error = encoder
            .encode_str_into("", &mut output, 0)
            .expect_err("caller-buffer finish capacity failure should be reported");
        assert_eq!(CharsetEncodeErrorKind::OutputLengthOverflow, error.kind());
        reset_coverage_hooks();
    }

    #[test]
    fn test_charset_string_encoder_encode_str_into_does_not_reserve_chars() {
        reset_coverage_hooks();
        let mut encoder = CharsetStringEncoder::new(Utf8Codec);
        let mut output = [0_u8; 1];

        CharsetStringEncoder::<Utf8Codec>::coverage_fail_next_reserve();
        let written = encoder
            .encode_str_into("A", &mut output, 0)
            .expect("caller-buffer encoding should not reserve characters");

        assert_eq!(1, written);
        assert_eq!(b'A', output[0]);
        reset_coverage_hooks();
    }

    #[test]
    fn test_charset_string_encoder_maps_transcode_failure() {
        let error = CharsetStringEncoder::<Utf8Codec>::coverage_map_encode_error(
            Charset::UTF_8,
            qubit_codec::TranscodeEncodeError::Failure(
                qubit_codec::TranscodeFailure::insufficient_output(3, 2, 1),
            ),
        );

        assert_eq!(
            CharsetEncodeErrorKind::BufferTooSmall {
                required: 2,
                available: 1,
            },
            error.kind(),
        );
        assert_eq!(3, error.index());
    }

    #[test]
    fn test_charset_string_encoder_maps_unencodable_transcode_error() {
        let error = CharsetStringEncoder::<Utf8Codec>::coverage_map_encode_error(
            Charset::UTF_8,
            qubit_codec::TranscodeEncodeError::Unencodable {
                input_index: 3,
                value: Some('中'),
            },
        );

        assert_eq!(
            CharsetEncodeErrorKind::UnmappableCharacter {
                value: '中' as u32
            },
            error.kind(),
        );
        assert_eq!(3, error.index());
    }

    #[test]
    fn test_charset_string_encoder_maps_owned_buffer_capacity_errors() {
        let capacity_error = qubit_codec_text::CharsetEncodeError::new(
            Charset::UTF_8,
            CharsetEncodeErrorKind::BufferTooSmall {
                required: 2,
                available: 1,
            },
            7,
        );
        let error = CharsetStringEncoder::<Utf8Codec>::coverage_map_owned_encode_error(
            Charset::UTF_8,
            capacity_error,
        );
        assert_eq!(CharsetEncodeErrorKind::OutputLengthOverflow, error.kind());

        let domain_error = qubit_codec_text::CharsetEncodeError::new(
            Charset::UTF_8,
            CharsetEncodeErrorKind::UnmappableCharacter { value: 0x100 },
            3,
        );
        let error = CharsetStringEncoder::<Utf8Codec>::coverage_map_owned_encode_error(
            Charset::UTF_8,
            domain_error,
        );
        assert_eq!(domain_error, error);
    }

    #[test]
    fn test_charset_string_encoder_maps_chunk_local_errors() {
        let error = CharsetStringEncoder::<Utf8Codec>::coverage_map_chunk_encode_error(
            Charset::UTF_8,
            TranscodeEncodeError::Unencodable {
                input_index: 2,
                value: Some('中'),
            },
            5,
        );
        assert_eq!(7, error.index());

        let capacity_error = qubit_codec_text::CharsetEncodeError::new(
            Charset::UTF_8,
            CharsetEncodeErrorKind::BufferTooSmall {
                required: 2,
                available: 1,
            },
            3,
        );
        let error = CharsetStringEncoder::<Utf8Codec>::coverage_map_chunk_encode_error(
            Charset::UTF_8,
            TranscodeEncodeError::Domain(TranscodeDomainError::main(capacity_error, 0)),
            5,
        );
        assert_eq!(capacity_error, error);
    }

    #[test]
    fn test_charset_string_encoder_reports_owned_length_addition_overflow() {
        let error = CharsetStringEncoder::<Utf8Codec>::coverage_ensure_owned_capacity(
            Charset::UTF_8,
            usize::MAX,
            1,
        )
        .expect_err("owned output length addition should overflow");

        assert_eq!(CharsetEncodeErrorKind::OutputLengthOverflow, error.kind());
    }
}
