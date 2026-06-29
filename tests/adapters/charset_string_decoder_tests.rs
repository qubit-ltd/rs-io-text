use core::num::NonZeroUsize;

use qubit_codec::Codec;
use qubit_codec_text::{
    Charset, CharsetCodec, CharsetDecodeError, CharsetDecodeErrorKind, CharsetDecodePolicy,
    CharsetDecodeResult, CharsetEncodeError, CharsetEncodeErrorKind, CharsetEncodeResult,
    MalformedAction, Utf32U32Codec,
};
use qubit_io_text::{CharsetStringDecoder, Utf8Codec};

#[derive(Clone, Copy, Debug, Default)]
struct InvalidInputErrorCodec;

impl CharsetCodec for InvalidInputErrorCodec {
    fn charset(&self) -> Charset {
        Charset::ASCII
    }
}

impl Codec for InvalidInputErrorCodec {
    type Value = char;
    type Unit = u8;
    type DecodeError = CharsetDecodeError;
    type EncodeError = CharsetEncodeError;

    const MIN_UNITS_PER_VALUE: NonZeroUsize = NonZeroUsize::MIN;

    const MAX_UNITS_PER_VALUE: NonZeroUsize = NonZeroUsize::MIN;

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
        _value: &char,
        _output: &mut [u8],
        output_index: usize,
    ) -> CharsetEncodeResult<NonZeroUsize> {
        let kind = CharsetEncodeErrorKind::UnmappableCharacter {
            value: output_index as u32,
        };
        Err(CharsetEncodeError::new(Charset::ASCII, kind, output_index))
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct HugeDecodeFinishBoundsCodec;

#[derive(Clone, Copy, Debug, Default)]
struct HugeDecodeResetBoundsCodec;

impl CharsetCodec for HugeDecodeResetBoundsCodec {
    fn charset(&self) -> Charset {
        Charset::ASCII
    }
}

impl Codec for HugeDecodeResetBoundsCodec {
    type Value = char;
    type Unit = u8;
    type DecodeError = CharsetDecodeError;
    type EncodeError = CharsetEncodeError;

    const MIN_UNITS_PER_VALUE: NonZeroUsize = NonZeroUsize::MIN;

    const MAX_UNITS_PER_VALUE: NonZeroUsize = NonZeroUsize::MIN;

    const MAX_DECODE_RESET_VALUES: usize = usize::MAX;

    unsafe fn decode(
        &mut self,
        _input: &[u8],
        _input_index: usize,
    ) -> Result<(char, NonZeroUsize), qubit_codec::DecodeFailure<Self::DecodeError>> {
        Ok(('A', NonZeroUsize::MIN))
    }

    unsafe fn encode(
        &mut self,
        _value: &char,
        _output: &mut [u8],
        output_index: usize,
    ) -> CharsetEncodeResult<NonZeroUsize> {
        let kind = CharsetEncodeErrorKind::UnmappableCharacter {
            value: output_index as u32,
        };
        Err(CharsetEncodeError::new(Charset::ASCII, kind, output_index))
    }
}

impl CharsetCodec for HugeDecodeFinishBoundsCodec {
    fn charset(&self) -> Charset {
        Charset::ASCII
    }
}

impl Codec for HugeDecodeFinishBoundsCodec {
    type Value = char;
    type Unit = u8;
    type DecodeError = CharsetDecodeError;
    type EncodeError = CharsetEncodeError;

    const MIN_UNITS_PER_VALUE: NonZeroUsize = NonZeroUsize::MIN;

    const MAX_UNITS_PER_VALUE: NonZeroUsize = NonZeroUsize::MIN;

    const MAX_DECODE_FLUSH_VALUES: usize = usize::MAX;

    unsafe fn decode(
        &mut self,
        _input: &[u8],
        _input_index: usize,
    ) -> Result<(char, NonZeroUsize), qubit_codec::DecodeFailure<Self::DecodeError>> {
        Ok(('A', NonZeroUsize::MIN))
    }

    unsafe fn encode(
        &mut self,
        _value: &char,
        _output: &mut [u8],
        output_index: usize,
    ) -> CharsetEncodeResult<NonZeroUsize> {
        let kind = CharsetEncodeErrorKind::UnmappableCharacter {
            value: output_index as u32,
        };
        Err(CharsetEncodeError::new(Charset::ASCII, kind, output_index))
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct DecodeFlushErrorCodec;

impl CharsetCodec for DecodeFlushErrorCodec {
    fn charset(&self) -> Charset {
        Charset::ASCII
    }
}

impl Codec for DecodeFlushErrorCodec {
    type Value = char;
    type Unit = u8;
    type DecodeError = CharsetDecodeError;
    type EncodeError = CharsetEncodeError;

    const MIN_UNITS_PER_VALUE: NonZeroUsize = NonZeroUsize::MIN;

    const MAX_UNITS_PER_VALUE: NonZeroUsize = NonZeroUsize::MIN;

    unsafe fn decode(
        &mut self,
        _input: &[u8],
        _input_index: usize,
    ) -> Result<(char, NonZeroUsize), qubit_codec::DecodeFailure<Self::DecodeError>> {
        Ok(('A', NonZeroUsize::MIN))
    }

    unsafe fn decode_flush(
        &mut self,
        _output: &mut [char],
        output_index: usize,
    ) -> CharsetDecodeResult<usize> {
        let kind = CharsetDecodeErrorKind::malformed_unknown();
        Err(CharsetDecodeError::new(Charset::ASCII, kind, output_index))
    }

    unsafe fn encode(
        &mut self,
        _value: &char,
        _output: &mut [u8],
        output_index: usize,
    ) -> CharsetEncodeResult<NonZeroUsize> {
        let kind = CharsetEncodeErrorKind::UnmappableCharacter {
            value: output_index as u32,
        };
        Err(CharsetEncodeError::new(Charset::ASCII, kind, output_index))
    }
}

#[test]
fn test_charset_string_decoder_decode_to_string_decodes_complete_input() {
    let mut decoder = CharsetStringDecoder::new(Utf8Codec);

    let text = decoder
        .decode_to_string("A中".as_bytes())
        .expect("complete UTF-8 input decodes to a string");

    assert_eq!("A中", text);
}

#[test]
fn test_charset_string_decoder_exposes_configuration_and_wrapped_decoder() {
    let mut decoder = CharsetStringDecoder::with_policy(
        Utf8Codec,
        CharsetDecodePolicy::ignore_with_replacement('!'),
    );

    assert_eq!(MalformedAction::Ignore, decoder.malformed_action());
    assert_eq!('!', decoder.replacement());
    assert_eq!(
        MalformedAction::Ignore,
        decoder.decoder().malformed_action()
    );
    assert_eq!('!', decoder.decoder_mut().replacement());

    let inner = decoder.into_decoder();
    assert_eq!(MalformedAction::Ignore, inner.malformed_action());
    assert_eq!('!', inner.replacement());
}

#[test]
fn test_charset_string_decoder_decode_to_string_into_starts_at_input_index() {
    let mut decoder = CharsetStringDecoder::new(Utf8Codec);
    let mut output = String::from("prefix:");

    decoder
        .decode_to_string_into("xxA中".as_bytes(), 2, &mut output)
        .expect("UTF-8 decoder should start from requested input index");

    assert_eq!("prefix:A中", output);
}

#[test]
fn test_charset_string_decoder_decode_to_string_into_reports_invalid_input_index() {
    let mut decoder = CharsetStringDecoder::new(Utf8Codec);
    let mut output = String::from("unchanged");

    let error = decoder
        .decode_to_string_into(b"A", 2, &mut output)
        .expect_err("input index outside slice should be rejected");

    assert_eq!(
        CharsetDecodeErrorKind::InvalidInputIndex { input_len: 1 },
        error.kind(),
    );
    assert_eq!(2, error.index());
    assert_eq!("unchanged", output);
}

#[test]
fn test_charset_string_decoder_decode_to_string_supports_non_byte_units() {
    let mut decoder = CharsetStringDecoder::new(Utf32U32Codec);

    let text = decoder
        .decode_to_string(&['A' as u32, '中' as u32])
        .expect("UTF-32 code points should decode");

    assert_eq!("A中", text);
}

#[test]
fn test_charset_string_decoder_decode_to_string_reports_incomplete_tail() {
    let mut decoder = CharsetStringDecoder::new(Utf8Codec);

    let error = decoder
        .decode_to_string(&[0xe4, 0xb8])
        .expect_err("closed input with incomplete UTF-8 tail should fail");

    assert_eq!(
        CharsetDecodeErrorKind::IncompleteSequence {
            required: 3,
            available: 2,
        },
        error.kind(),
    );
    assert_eq!(0, error.index());
}

#[cfg(coverage)]
mod coverage_tests {
    use qubit_codec_text::CharsetDecodeErrorKind;
    use qubit_io_text::{CharsetStringDecoder, Utf8Codec};

    fn reset_coverage_hooks() {
        CharsetStringDecoder::<Utf8Codec>::coverage_reset_reserve_hooks();
    }

    #[test]
    fn test_charset_string_decoder_reports_char_reserve_failure() {
        reset_coverage_hooks();
        let mut decoder = CharsetStringDecoder::new(Utf8Codec);

        CharsetStringDecoder::<Utf8Codec>::coverage_fail_next_reserve();
        let error = decoder
            .decode_to_string(b"A")
            .expect_err("char buffer reserve failure should be reported");

        assert_eq!(CharsetDecodeErrorKind::OutputLengthOverflow, error.kind());
        reset_coverage_hooks();
    }

    #[test]
    fn test_charset_string_decoder_reports_string_reserve_failure() {
        reset_coverage_hooks();
        let mut decoder = CharsetStringDecoder::new(Utf8Codec);
        let mut output = String::from("seed:");

        CharsetStringDecoder::<Utf8Codec>::coverage_fail_reserve_after(1);
        let error = decoder
            .decode_to_string_into(b"A", 0, &mut output)
            .expect_err("string reserve failure should be reported");

        assert_eq!(CharsetDecodeErrorKind::OutputLengthOverflow, error.kind());
        assert_eq!("seed:", output);
        reset_coverage_hooks();
    }
}

#[test]
fn test_charset_string_decoder_decode_to_string_offsets_domain_errors() {
    let mut decoder =
        CharsetStringDecoder::with_policy(InvalidInputErrorCodec, CharsetDecodePolicy::report());
    let mut output = String::new();

    let error = decoder
        .decode_to_string_into(b"xxA", 2, &mut output)
        .expect_err("decode error should be reported at the absolute input index");

    assert_eq!(CharsetDecodeErrorKind::malformed_unknown(), error.kind());
    assert_eq!(2, error.index());
}

#[test]
fn test_charset_string_decoder_decode_to_string_reports_finish_capacity_overflow() {
    let mut decoder = CharsetStringDecoder::new(HugeDecodeFinishBoundsCodec);

    let error = decoder
        .decode_to_string(b"A")
        .expect_err("finish capacity overflow should be reported");

    assert_eq!(CharsetDecodeErrorKind::OutputLengthOverflow, error.kind());
}

#[test]
fn test_charset_string_decoder_decode_to_string_reports_char_reserve_overflow() {
    let mut decoder = CharsetStringDecoder::new(HugeDecodeResetBoundsCodec);

    let error = decoder
        .decode_to_string(b"")
        .expect_err("huge reset bound should fail char buffer reservation");

    assert_eq!(CharsetDecodeErrorKind::OutputLengthOverflow, error.kind());
}

#[test]
fn test_charset_string_decoder_decode_to_string_propagates_finish_errors() {
    let mut decoder = CharsetStringDecoder::new(DecodeFlushErrorCodec);

    let error = decoder
        .decode_to_string(b"A")
        .expect_err("decode flush errors should be propagated");

    assert_eq!(CharsetDecodeErrorKind::malformed_unknown(), error.kind());
    assert_eq!(1, error.index());
}
