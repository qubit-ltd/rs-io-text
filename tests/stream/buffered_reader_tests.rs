// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;
use std::io::ErrorKind;
use std::io::Read;
use std::num::NonZeroUsize;

use qubit_codec::CapacityError;
use qubit_codec::TranscodeDecodeError;
use qubit_codec::TranscodeDecoder;
use qubit_codec::TranscodeDomainError;
use qubit_codec::TranscodeFailure;
use qubit_codec::TranscodeProgress;
use qubit_codec::TranscodeStatus;
use qubit_codec::Transcoder;
use qubit_codec_text::CharsetDecodePolicy;
use qubit_codec_text::CharsetDecoder;
use qubit_codec_text::Utf8Codec;
use qubit_io_text::BufferedReader;
#[cfg(coverage)]
use qubit_io_text::LineEnding;
use qubit_io_text::LineEndingSet;
use qubit_io_text::TextLineRead;
use qubit_io_text::TextRead;

#[derive(Debug, Default)]
struct ExpandingDecoder;

impl Transcoder for ExpandingDecoder {
    type Input = u8;
    type Output = char;
    type Error = TranscodeDecodeError<std::io::Error>;
    fn max_transcode_output_len(&self, _: usize) -> Result<usize, CapacityError> {
        Ok(5)
    }
    fn max_finish_output_len(&self) -> Result<usize, CapacityError> {
        Ok(0)
    }
    fn reset(&mut self, output: &mut [char], index: usize) -> Result<usize, Self::Error> {
        TranscodeFailure::ensure_output_index(output.len(), index)?;
        Ok(0)
    }
    fn transcode(
        &mut self,
        input: &[u8],
        index: usize,
        output: &mut [char],
        output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        TranscodeFailure::ensure_output_index(output.len(), output_index)?;
        if input.len() == index {
            return Ok(TranscodeProgress::complete(0, 0));
        }
        let available = output.len() - output_index;
        if available < 5 {
            return Ok(TranscodeProgress::new(
                TranscodeStatus::need_output(NonZeroUsize::new(5).unwrap()),
                0,
                0,
            ));
        }
        for slot in &mut output[output_index..output_index + 5] {
            *slot = 'x';
        }
        Ok(TranscodeProgress::complete(1, 5))
    }
    fn finish(&mut self, output: &mut [char], index: usize) -> Result<usize, Self::Error> {
        TranscodeFailure::ensure_output_index(output.len(), index)?;
        Ok(0)
    }
}

#[test]
fn test_buffered_reader_accepts_crlf_across_decode_windows() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader =
        BufferedReader::with_capacity(Cursor::new("first\r\nsecond\rthird".as_bytes().to_vec()), decoder, 1);
    let mut line = String::new();

    assert_eq!(LineEndingSet::ALL, reader.line_endings());
    assert!(reader.read_line(&mut line)?);
    assert_eq!("first\r\n", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("second\r", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("third", line);
    Ok(())
}

#[test]
fn test_buffered_reader_pending_character_is_returned_by_other_read_methods() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(Cursor::new(b"first\rsecond".to_vec()), decoder, 1);
    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    assert_eq!(Some('s'), reader.read_char()?);

    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(Cursor::new(b"first\rsecond".to_vec()), decoder, 1);
    assert!(reader.read_line(&mut line)?);
    let mut chars = Vec::new();
    assert_eq!(1, reader.read_chars(&mut chars, 1)?);
    assert_eq!(vec!['s'], chars);

    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(Cursor::new(b"first\rsecond".to_vec()), decoder, 1);
    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    let mut output = String::new();
    assert_eq!(6, reader.read_to_string(&mut output)?);
    assert_eq!("second", output);
    Ok(())
}

#[test]
fn test_buffered_reader_line_ending_configuration_and_parts() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(Cursor::new(b"first\rsecond".to_vec()), decoder, 1)
        .with_line_endings(LineEndingSet::CR);
    assert_eq!(LineEndingSet::CR, reader.line_endings());
    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    let parts = reader.into_parts();
    assert_eq!(&['s', 'e'], &parts.pending_chars[..2]);

    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let reader = BufferedReader::with_capacity(Cursor::new(b"first\rsecond".to_vec()), decoder, 1);
    let mut reader = reader;
    line.clear();
    assert!(reader.read_line(&mut line)?);
    let parts = reader.into_parts();
    assert_eq!('s', parts.pending_chars[0]);
    Ok(())
}

#[test]
fn test_buffered_reader_read_line_limited_returns_unterminated_tail() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(Cursor::new(b"tail".to_vec()), decoder, 1);
    let mut line = String::new();
    assert!(reader.read_line_limited(&mut line, 16)?);
    assert_eq!("tail", line);
    assert!(!reader.read_line_limited(&mut line, 16)?);

    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut empty = BufferedReader::new(Cursor::new(Vec::<u8>::new()), decoder);
    let mut output = String::new();
    assert!(!empty.read_line_limited(&mut output, 16)?);
    Ok(())
}

#[test]
fn test_buffered_reader_read_to_string_limited_enforces_utf8_limit() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(Cursor::new("A中".as_bytes().to_vec()), decoder, 1);
    let mut output = String::from("prefix:");
    assert_eq!(2, reader.read_to_string_limited(&mut output, 4)?);
    assert_eq!("prefix:A中", output);
    assert_eq!(0, reader.read_to_string_limited(&mut output, 4)?);

    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut empty = BufferedReader::new(Cursor::new(Vec::<u8>::new()), decoder);
    assert_eq!(0, empty.read_to_string_limited(&mut output, 4)?);

    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(Cursor::new("A中".as_bytes().to_vec()), decoder, 1);
    let mut output = String::from("prefix:");
    let error = reader
        .read_to_string_limited(&mut output, 1)
        .expect_err("the multibyte character should exceed the limit");
    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("prefix:", output);
    Ok(())
}

#[test]
fn test_buffered_reader_limited_reads_preserve_pending_order_and_endings() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(Cursor::new(b"first\rbc".to_vec()), decoder, 1);
    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("first\r", line);
    let mut output = String::new();
    assert_eq!(2, reader.read_to_string_limited(&mut output, 8)?);
    assert_eq!("bc", output);

    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(Cursor::new(b"first\rsecond\nthird".to_vec()), decoder, 1)
        .with_line_endings(LineEndingSet::CR);
    line.clear();
    assert!(reader.read_line_limited(&mut line, 32)?);
    assert_eq!("first\r", line);
    line.clear();
    assert!(reader.read_line_limited(&mut line, 32)?);
    assert_eq!("second\nthird", line);
    Ok(())
}

#[cfg(coverage)]
#[test]
fn test_buffered_reader_covers_limited_line_ending_branches() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::new(Cursor::new(b"a\r\nb".to_vec()), decoder);
    let mut line = String::new();
    assert!(reader.read_line_limited(&mut line, 8)?);
    assert_eq!("a\r\n", line);

    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::new(Cursor::new(b"\r\n".to_vec()), decoder);
    let error = reader
        .read_line_limited(&mut String::new(), 1)
        .expect_err("the LF in CRLF should exceed the limit");
    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(None, reader.read_char()?);

    for (input, endings, expected) in [
        (b"a\rb".as_slice(), LineEndingSet::ALL, "a\r"),
        (b"a\rb".as_slice(), LineEndingSet::CRLF, "a\rb"),
        (
            b"a\r\nb".as_slice(),
            LineEndingSet::only(LineEnding::Lf).without(LineEnding::Lf),
            "a\r\nb",
        ),
        (b"a\r".as_slice(), LineEndingSet::CRLF, "a\r"),
    ] {
        let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
        let mut reader = BufferedReader::new(Cursor::new(input.to_vec()), decoder).with_line_endings(endings);
        let mut line = String::new();
        assert!(reader.read_line_limited(&mut line, 8)?);
        assert_eq!(expected, line);
    }
    Ok(())
}

#[cfg(coverage)]
#[test]
fn test_buffered_reader_covers_discard_line_branches() -> std::io::Result<()> {
    for (input, endings) in [
        (b"aX\n".as_slice(), LineEndingSet::ALL),
        (b"aX\n".as_slice(), LineEndingSet::CRLF),
        (b"aX\r\n".as_slice(), LineEndingSet::CRLF),
        (b"aX\rY\n".as_slice(), LineEndingSet::CRLF),
        (b"aX\rY".as_slice(), LineEndingSet::ALL),
        (b"aX\r".as_slice(), LineEndingSet::CRLF),
        (b"aX\r".as_slice(), LineEndingSet::CR),
    ] {
        let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
        let mut reader = BufferedReader::new(Cursor::new(input.to_vec()), decoder).with_line_endings(endings);
        let error = reader
            .read_line_limited(&mut String::new(), 1)
            .expect_err("the second character should exceed the limit");
        assert_eq!(ErrorKind::InvalidData, error.kind());
    }
    Ok(())
}

#[cfg(coverage)]
#[test]
fn test_buffered_reader_covers_decoder_status_recovery_branches() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::new(Cursor::new(Vec::<u8>::new()), decoder);
    BufferedReader::<Cursor<Vec<u8>>, CharsetDecoder<Utf8Codec>>::coverage_force_next_status(1);
    let error = reader
        .read_char()
        .expect_err("forced EOF input request should be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());

    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::new(Cursor::new(Vec::<u8>::new()), decoder);
    BufferedReader::<Cursor<Vec<u8>>, CharsetDecoder<Utf8Codec>>::coverage_force_next_status(2);
    assert_eq!(None, reader.read_char()?);

    let mut reader = BufferedReader::new(Cursor::new(Vec::<u8>::new()), ExpandingDecoder);
    BufferedReader::<Cursor<Vec<u8>>, ExpandingDecoder>::coverage_force_next_status(3);
    assert_eq!(None, reader.read_char()?);

    let mut reader = BufferedReader::new(Cursor::new(Vec::<u8>::new()), ExpandingDecoder);
    BufferedReader::<Cursor<Vec<u8>>, ExpandingDecoder>::coverage_force_next_status(4);
    assert_eq!(None, reader.read_char()?);

    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::new(Cursor::new(Vec::<u8>::new()), decoder);
    assert!(!reader.coverage_finish_again()?);

    let mut reader = BufferedReader::new(Cursor::new(Vec::<u8>::new()), ErrorResetDecoder);
    reader.coverage_touch_buffer_state();

    let mut reader = BufferedReader::new(Cursor::new(Vec::<u8>::new()), OverflowResetDecoder);
    reader.coverage_touch_buffer_state();

    let mut reader = BufferedReader::new(Cursor::new(Vec::<u8>::new()), ExpandingDecoder);
    BufferedReader::<Cursor<Vec<u8>>, ExpandingDecoder>::coverage_force_next_status(3);
    BufferedReader::<Cursor<Vec<u8>>, ExpandingDecoder>::coverage_force_next_fill_error();
    let error = reader.read_char().expect_err("forced refill errors should propagate");
    assert_eq!(ErrorKind::Other, error.kind());

    Ok(())
}
impl TranscodeDecoder for ExpandingDecoder {
    type DecodeError = std::io::Error;
}

#[derive(Debug, Default)]
struct NeedInputDecoder;

#[derive(Debug)]
struct FirstChunkThenError {
    first: bool,
}

impl Read for FirstChunkThenError {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if self.first {
            self.first = false;
            output[..4].copy_from_slice(&[1, 2, 3, 4]);
            Ok(4)
        } else {
            Err(std::io::Error::other("refill failed"))
        }
    }
}

#[derive(Debug)]
struct AlwaysFailInput;

impl Read for AlwaysFailInput {
    fn read(&mut self, _output: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("read failed"))
    }
}

#[derive(Debug)]
struct CrThenErrorThenByte {
    state: u8,
}

impl Read for CrThenErrorThenByte {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        match self.state {
            0 => {
                output[0] = b'\r';
                self.state = 1;
                Ok(1)
            }
            1 => {
                self.state = 2;
                Err(std::io::Error::other("transient refill failure"))
            }
            2 => {
                output[0] = b'X';
                self.state = 3;
                Ok(1)
            }
            _ => Ok(0),
        }
    }
}

impl Transcoder for NeedInputDecoder {
    type Input = u8;
    type Output = char;
    type Error = TranscodeDecodeError<std::io::Error>;

    fn max_transcode_output_len(&self, _: usize) -> Result<usize, CapacityError> {
        Ok(1)
    }

    fn max_finish_output_len(&self) -> Result<usize, CapacityError> {
        Ok(0)
    }

    fn reset(&mut self, output: &mut [char], index: usize) -> Result<usize, Self::Error> {
        TranscodeFailure::ensure_output_index(output.len(), index)?;
        Ok(0)
    }

    fn transcode(
        &mut self,
        input: &[u8],
        index: usize,
        output: &mut [char],
        output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        TranscodeFailure::ensure_output_index(output.len(), output_index)?;
        if input.len() - index < 5 {
            return Ok(TranscodeProgress::new(
                TranscodeStatus::need_input(NonZeroUsize::new(5).unwrap()),
                0,
                0,
            ));
        }
        output[output_index] = 'n';
        Ok(TranscodeProgress::complete(5, 1))
    }

    fn finish(&mut self, output: &mut [char], index: usize) -> Result<usize, Self::Error> {
        TranscodeFailure::ensure_output_index(output.len(), index)?;
        Ok(0)
    }
}

impl TranscodeDecoder for NeedInputDecoder {
    type DecodeError = std::io::Error;
}

#[derive(Debug, Default)]
struct FinishCharDecoder;

impl Transcoder for FinishCharDecoder {
    type Input = u8;
    type Output = char;
    type Error = TranscodeDecodeError<std::io::Error>;

    fn max_transcode_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn max_finish_output_len(&self) -> Result<usize, CapacityError> {
        Ok(8)
    }

    fn reset(&mut self, output: &mut [char], output_index: usize) -> Result<usize, Self::Error> {
        TranscodeFailure::ensure_output_index(output.len(), output_index)?;
        Ok(0)
    }

    fn transcode(
        &mut self,
        input: &[u8],
        input_index: usize,
        _output: &mut [char],
        _output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        Ok(TranscodeProgress::complete(input.len() - input_index, 0))
    }

    fn finish(&mut self, output: &mut [char], output_index: usize) -> Result<usize, Self::Error> {
        TranscodeFailure::ensure_output_index(output.len(), output_index)?;
        output[output_index] = '!';
        Ok(1)
    }
}

impl TranscodeDecoder for FinishCharDecoder {
    type DecodeError = std::io::Error;
}

#[derive(Debug, Default)]
struct LifecycleDecoder {
    started: bool,
    finished: bool,
}

impl Transcoder for LifecycleDecoder {
    type Input = u8;
    type Output = char;
    type Error = TranscodeDecodeError<std::io::Error>;

    fn max_reset_output_len(&self) -> Result<usize, CapacityError> {
        Ok(5)
    }

    fn max_transcode_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn max_finish_output_len(&self) -> Result<usize, CapacityError> {
        Ok(1)
    }

    fn reset(&mut self, output: &mut [char], output_index: usize) -> Result<usize, Self::Error> {
        assert!(!self.started, "decoder reset must run exactly once");
        output[output_index] = '^';
        output[output_index + 1] = '~';
        output[output_index + 2] = '+';
        output[output_index + 3] = '*';
        output[output_index + 4] = '-';
        self.started = true;
        Ok(5)
    }

    fn transcode(
        &mut self,
        input: &[u8],
        input_index: usize,
        output: &mut [char],
        output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        assert!(self.started, "decoder must reset before transcode");
        let input = &input[input_index..];
        for (offset, byte) in input.iter().enumerate() {
            output[output_index + offset] = char::from(*byte);
        }
        Ok(TranscodeProgress::complete(input.len(), input.len()))
    }

    fn finish(&mut self, output: &mut [char], output_index: usize) -> Result<usize, Self::Error> {
        assert!(self.started, "decoder must reset before finish");
        assert!(!self.finished, "decoder finish must run exactly once");
        output[output_index] = '!';
        self.finished = true;
        Ok(1)
    }
}

impl TranscodeDecoder for LifecycleDecoder {
    type DecodeError = std::io::Error;
}

#[derive(Debug, Default)]
struct OverflowFinishDecoder;

impl Transcoder for OverflowFinishDecoder {
    type Input = u8;
    type Output = char;
    type Error = TranscodeDecodeError<std::io::Error>;

    fn max_transcode_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn max_finish_output_len(&self) -> Result<usize, CapacityError> {
        Err(CapacityError::OutputLengthOverflow)
    }

    fn reset(&mut self, output: &mut [char], output_index: usize) -> Result<usize, Self::Error> {
        TranscodeFailure::ensure_output_index(output.len(), output_index)?;
        Ok(0)
    }

    fn transcode(
        &mut self,
        input: &[u8],
        input_index: usize,
        _output: &mut [char],
        _output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        Ok(TranscodeProgress::complete(input.len() - input_index, 0))
    }

    fn finish(&mut self, _output: &mut [char], _output_index: usize) -> Result<usize, Self::Error> {
        unreachable!("capacity planning fails before finish")
    }
}

impl TranscodeDecoder for OverflowFinishDecoder {
    type DecodeError = std::io::Error;
}

#[derive(Debug, Default)]
struct ErrorFinishDecoder;

impl Transcoder for ErrorFinishDecoder {
    type Input = u8;
    type Output = char;
    type Error = TranscodeDecodeError<std::io::Error>;

    fn max_transcode_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn reset(&mut self, _output: &mut [char], _output_index: usize) -> Result<usize, Self::Error> {
        Ok(0)
    }

    fn transcode(
        &mut self,
        input: &[u8],
        input_index: usize,
        _output: &mut [char],
        _output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        Ok(TranscodeProgress::complete(input.len() - input_index, 0))
    }

    fn finish(&mut self, _output: &mut [char], _output_index: usize) -> Result<usize, Self::Error> {
        Err(TranscodeDecodeError::Domain(TranscodeDomainError::Finish {
            source: std::io::Error::other("finish failed"),
        }))
    }
}

impl TranscodeDecoder for ErrorFinishDecoder {
    type DecodeError = std::io::Error;
}

#[derive(Debug, Default)]
struct ErrorResetDecoder;

impl Transcoder for ErrorResetDecoder {
    type Input = u8;
    type Output = char;
    type Error = TranscodeDecodeError<std::io::Error>;

    fn max_transcode_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn reset(&mut self, _output: &mut [char], _output_index: usize) -> Result<usize, Self::Error> {
        Err(TranscodeDecodeError::Domain(TranscodeDomainError::Reset {
            source: std::io::Error::other("reset failed"),
        }))
    }

    fn transcode(
        &mut self,
        _input: &[u8],
        _input_index: usize,
        _output: &mut [char],
        _output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        unreachable!("reset failure prevents transcoding")
    }

    fn finish(&mut self, _output: &mut [char], _output_index: usize) -> Result<usize, Self::Error> {
        unreachable!("reset failure prevents finishing")
    }
}

impl TranscodeDecoder for ErrorResetDecoder {
    type DecodeError = std::io::Error;
}

#[derive(Debug, Default)]
struct OverflowResetDecoder;

impl Transcoder for OverflowResetDecoder {
    type Input = u8;
    type Output = char;
    type Error = TranscodeDecodeError<std::io::Error>;

    fn max_reset_output_len(&self) -> Result<usize, CapacityError> {
        Err(CapacityError::OutputLengthOverflow)
    }

    fn max_transcode_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn reset(&mut self, _output: &mut [char], _output_index: usize) -> Result<usize, Self::Error> {
        unreachable!("capacity planning fails before reset")
    }

    fn transcode(
        &mut self,
        _input: &[u8],
        _input_index: usize,
        _output: &mut [char],
        _output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        unreachable!("capacity planning fails before transcoding")
    }

    fn finish(&mut self, _output: &mut [char], _output_index: usize) -> Result<usize, Self::Error> {
        unreachable!("capacity planning fails before finishing")
    }
}

impl TranscodeDecoder for OverflowResetDecoder {
    type DecodeError = std::io::Error;
}

#[test]
fn test_buffered_reader_decodes_utf8_across_single_byte_refills() -> std::io::Result<()> {
    let bytes = "Aé🙂".as_bytes().to_vec();
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(Cursor::new(bytes), decoder, 1);

    let mut output = String::new();
    let count = reader.read_to_string(&mut output)?;

    assert_eq!(3, count);
    assert_eq!("Aé🙂", output);
    Ok(())
}

#[test]
fn test_buffered_reader_preserves_need_output_for_expanding_decoder() -> std::io::Result<()> {
    let mut reader = BufferedReader::with_capacity(Cursor::new(vec![1]), ExpandingDecoder, 1);
    let mut output = String::new();
    assert_eq!(5, reader.read_to_string(&mut output)?);
    assert_eq!("xxxxx", output);
    Ok(())
}

#[test]
fn test_buffered_reader_refills_when_decoder_needs_more_input() -> std::io::Result<()> {
    let mut reader = BufferedReader::with_capacity(Cursor::new(vec![1, 2, 3, 4, 5]), NeedInputDecoder, 1);
    assert_eq!(Some('n'), reader.read_char()?);
    assert_eq!(None, reader.read_char()?);
    Ok(())
}

#[test]
fn test_buffered_reader_propagates_limited_input_errors() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::new(AlwaysFailInput, decoder);
    let mut output = String::new();
    assert!(
        reader
            .read_to_string_limited(&mut output, 8)
            .expect_err("limited string reads should report input errors")
            .kind()
            == ErrorKind::Other
    );

    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::new(AlwaysFailInput, decoder);
    assert_eq!(
        ErrorKind::Other,
        reader
            .read_line_limited(&mut output, 8)
            .expect_err("limited line reads should report input errors")
            .kind(),
    );
    Ok(())
}

#[test]
fn test_buffered_reader_propagates_refill_errors_after_need_input() -> std::io::Result<()> {
    let mut reader = BufferedReader::with_capacity(FirstChunkThenError { first: true }, NeedInputDecoder, 4);
    let error = reader
        .read_char()
        .expect_err("a refill error after NeedInput should propagate");
    assert_eq!(ErrorKind::Other, error.kind());
    Ok(())
}

#[test]
fn test_buffered_reader_accessors() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::new(Cursor::new("abc\n".as_bytes().to_vec()), decoder);

    assert_eq!(0, reader.inner().position());
    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("abc\n", line);

    let parts = reader.into_parts();
    assert_eq!(4, parts.input.position());
    assert!(parts.unread_bytes.readable().is_empty());
    assert!(parts.pending_chars.is_empty());
    Ok(())
}

#[test]
fn test_buffered_reader_into_parts_preserves_unreturned_characters() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(Cursor::new(b"abc".to_vec()), decoder, 4);

    assert_eq!(Some('a'), reader.read_char()?);

    let parts = reader.into_parts();
    assert_eq!(3, parts.input.position());
    assert!(parts.unread_bytes.readable().is_empty());
    assert_eq!(['b', 'c'], parts.pending_chars.as_slice());
    Ok(())
}

#[test]
fn test_buffered_reader_read_chars_with_zero_limit_does_not_read() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::new(Cursor::new("abc".as_bytes().to_vec()), decoder);
    let mut chars = Vec::new();

    assert_eq!(0, reader.read_chars(&mut chars, 0)?);
    assert!(chars.is_empty());
    assert_eq!(0, reader.inner().position());
    Ok(())
}

#[test]
fn test_buffered_reader_read_line_limited_restores_output_on_overflow() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::new(Cursor::new("a中\nnext".as_bytes().to_vec()), decoder);
    let mut output = String::from("prefix-");

    let error = reader
        .read_line_limited(&mut output, 4)
        .expect_err("line exceeding the UTF-8 byte limit should fail");
    assert_eq!(std::io::ErrorKind::InvalidData, error.kind());
    assert_eq!("prefix-", output);

    output.clear();
    assert!(reader.read_line_limited(&mut output, 16)?);
    assert_eq!("next", output);
    Ok(())
}

#[test]
fn test_buffered_reader_read_line_limited_accepts_exact_utf8_limit() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::new(Cursor::new("hello 世界\nnext".as_bytes().to_vec()), decoder);
    let mut output = String::new();

    assert!(reader.read_line_limited(&mut output, "hello 世界\n".len())?);
    assert_eq!("hello 世界\n", output);
    Ok(())
}

#[test]
fn test_buffered_reader_emits_decoder_finish_output() -> std::io::Result<()> {
    let mut reader = BufferedReader::with_capacity(Cursor::new(Vec::new()), FinishCharDecoder, 1);

    assert_eq!(Some('!'), reader.read_char()?);
    assert_eq!(None, reader.read_char()?);
    assert_eq!(None, reader.read_char()?);
    Ok(())
}

#[test]
fn test_buffered_reader_runs_complete_lifecycle_on_first_read() -> std::io::Result<()> {
    let mut reader = BufferedReader::with_capacity(Cursor::new(b"A".to_vec()), LifecycleDecoder::default(), 1);

    let mut output = String::new();
    reader.read_to_string(&mut output)?;

    assert_eq!("^~+*-A!", output);
    Ok(())
}

#[test]
fn test_buffered_reader_runs_complete_lifecycle_for_empty_stream() -> std::io::Result<()> {
    let mut reader = BufferedReader::with_capacity(Cursor::new(Vec::new()), LifecycleDecoder::default(), 1);

    let mut output = String::new();
    reader.read_to_string(&mut output)?;

    assert_eq!("^~+*-!", output);
    Ok(())
}

#[test]
fn test_buffered_reader_reports_finish_capacity_errors() {
    let mut reader = BufferedReader::new(Cursor::new(Vec::new()), OverflowFinishDecoder);

    let error = reader
        .read_char()
        .expect_err("finish capacity errors must become I/O errors");

    assert_eq!(ErrorKind::OutOfMemory, error.kind());
}

#[test]
fn test_buffered_reader_propagates_finish_errors() {
    let mut reader = BufferedReader::new(Cursor::new(Vec::new()), ErrorFinishDecoder);

    let error = reader
        .read_char()
        .expect_err("decoder finish errors must become I/O errors");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_buffered_reader_propagates_reset_errors() {
    let mut reader = BufferedReader::new(Cursor::new(Vec::new()), ErrorResetDecoder);

    let error = reader
        .read_char()
        .expect_err("decoder reset errors must become I/O errors");

    assert_eq!(ErrorKind::InvalidData, error.kind());

    let mut reader = BufferedReader::new(Cursor::new(Vec::new()), ErrorResetDecoder);
    let mut output = Vec::new();
    let error = reader
        .read_chars(&mut output, 1)
        .expect_err("bulk reads must propagate decoder reset errors");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_buffered_reader_reports_reset_capacity_errors() {
    let mut reader = BufferedReader::new(Cursor::new(Vec::new()), OverflowResetDecoder);

    let error = reader
        .read_char()
        .expect_err("decoder reset capacity errors must become I/O errors");

    assert_eq!(ErrorKind::OutOfMemory, error.kind());
}

#[test]
fn test_buffered_reader_replaces_incomplete_eof_tail() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::replace('\u{FFFD}'));
    let mut reader = BufferedReader::with_capacity(Cursor::new(vec![0xE2, 0x82]), decoder, 1);

    let mut output = String::new();
    let count = reader.read_to_string(&mut output)?;

    assert_eq!(1, count);
    assert_eq!("\u{FFFD}", output);
    Ok(())
}

#[test]
fn test_buffered_reader_reports_strict_incomplete_eof_tail() {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(Cursor::new(vec![0xE2, 0x82]), decoder, 1);

    let error = reader
        .read_char()
        .expect_err("strict incomplete EOF should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_buffered_reader_preserves_cr_when_crlf_lookahead_fails() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(CrThenErrorThenByte { state: 0 }, decoder, 1);
    let mut line = String::new();

    let error = reader.read_line(&mut line).expect_err("lookahead fails");
    assert_eq!(ErrorKind::Other, error.kind());
    assert!(line.is_empty());

    assert!(reader.read_line(&mut line)?);
    assert_eq!("\r", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("X", line);
    Ok(())
}
