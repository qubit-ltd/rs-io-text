// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{
    Cursor,
    ErrorKind,
};
use std::num::NonZeroUsize;

use qubit_codec::{
    CapacityError,
    TranscodeDecodeError,
    TranscodeDecoder,
    TranscodeDomainError,
    TranscodeProgress,
    Transcoder,
};
use qubit_codec_text::{
    CharsetDecodePolicy,
    CharsetDecoder,
    Utf8Codec,
};
use qubit_io_text::{
    BufferedReader,
    CodingErrorPolicy,
    TextLineRead,
    TextRead,
};

#[derive(Debug, Default)]
struct ExpandingDecoder;

impl Transcoder for ExpandingDecoder {
    type Input = u8;
    type Output = char;
    type Error = TranscodeDecodeError<std::io::Error>;
    fn max_transcode_output_len(
        &self,
        _: usize,
    ) -> Result<usize, CapacityError> {
        Ok(5)
    }
    fn max_finish_output_len(&self) -> Result<usize, CapacityError> {
        Ok(0)
    }
    fn reset(
        &mut self,
        output: &mut [char],
        index: usize,
    ) -> Result<usize, Self::Error> {
        qubit_codec::TranscodeFailure::ensure_output_index(output.len(), index)?;
        Ok(0)
    }
    fn transcode(
        &mut self,
        input: &[u8],
        index: usize,
        output: &mut [char],
        output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        qubit_codec::TranscodeFailure::ensure_output_index(output.len(), output_index)?;
        if input.len() == index {
            return Ok(TranscodeProgress::complete(0, 0));
        }
        let available = output.len() - output_index;
        if available < 5 {
            return Ok(TranscodeProgress::new(
                qubit_codec::TranscodeStatus::need_output(
                    NonZeroUsize::new(5).unwrap(),
                ),
                0,
                0,
            ));
        }
        for slot in &mut output[output_index..output_index + 5] {
            *slot = 'x';
        }
        Ok(TranscodeProgress::complete(1, 5))
    }
    fn finish(
        &mut self,
        output: &mut [char],
        index: usize,
    ) -> Result<usize, Self::Error> {
        qubit_codec::TranscodeFailure::ensure_output_index(output.len(), index)?;
        Ok(0)
    }
}
impl TranscodeDecoder for ExpandingDecoder {
    type DecodeError = std::io::Error;
}

#[derive(Debug, Default)]
struct FinishCharDecoder;

impl Transcoder for FinishCharDecoder {
    type Input = u8;
    type Output = char;
    type Error = TranscodeDecodeError<std::io::Error>;

    fn max_transcode_output_len(
        &self,
        input_len: usize,
    ) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn max_finish_output_len(&self) -> Result<usize, CapacityError> {
        Ok(8)
    }

    fn reset(
        &mut self,
        output: &mut [char],
        output_index: usize,
    ) -> Result<usize, Self::Error> {
        qubit_codec::TranscodeFailure::ensure_output_index(output.len(), output_index)?;
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

    fn finish(
        &mut self,
        output: &mut [char],
        output_index: usize,
    ) -> Result<usize, Self::Error> {
        qubit_codec::TranscodeFailure::ensure_output_index(output.len(), output_index)?;
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

    fn max_transcode_output_len(
        &self,
        input_len: usize,
    ) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn max_finish_output_len(&self) -> Result<usize, CapacityError> {
        Ok(1)
    }

    fn reset(
        &mut self,
        output: &mut [char],
        output_index: usize,
    ) -> Result<usize, Self::Error> {
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

    fn finish(
        &mut self,
        output: &mut [char],
        output_index: usize,
    ) -> Result<usize, Self::Error> {
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

    fn max_transcode_output_len(
        &self,
        input_len: usize,
    ) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn max_finish_output_len(&self) -> Result<usize, CapacityError> {
        Err(CapacityError::OutputLengthOverflow)
    }

    fn reset(
        &mut self,
        output: &mut [char],
        output_index: usize,
    ) -> Result<usize, Self::Error> {
        qubit_codec::TranscodeFailure::ensure_output_index(output.len(), output_index)?;
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

    fn finish(
        &mut self,
        _output: &mut [char],
        _output_index: usize,
    ) -> Result<usize, Self::Error> {
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

    fn max_transcode_output_len(
        &self,
        input_len: usize,
    ) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn reset(
        &mut self,
        _output: &mut [char],
        _output_index: usize,
    ) -> Result<usize, Self::Error> {
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

    fn finish(
        &mut self,
        _output: &mut [char],
        _output_index: usize,
    ) -> Result<usize, Self::Error> {
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

    fn max_transcode_output_len(
        &self,
        input_len: usize,
    ) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn reset(
        &mut self,
        _output: &mut [char],
        _output_index: usize,
    ) -> Result<usize, Self::Error> {
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

    fn finish(
        &mut self,
        _output: &mut [char],
        _output_index: usize,
    ) -> Result<usize, Self::Error> {
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

    fn max_transcode_output_len(
        &self,
        input_len: usize,
    ) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn reset(
        &mut self,
        _output: &mut [char],
        _output_index: usize,
    ) -> Result<usize, Self::Error> {
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

    fn finish(
        &mut self,
        _output: &mut [char],
        _output_index: usize,
    ) -> Result<usize, Self::Error> {
        unreachable!("capacity planning fails before finishing")
    }
}

impl TranscodeDecoder for OverflowResetDecoder {
    type DecodeError = std::io::Error;
}

#[test]
fn test_buffered_reader_decodes_utf8_across_single_byte_refills()
-> std::io::Result<()> {
    let bytes = "Aé🙂".as_bytes().to_vec();
    let decoder =
        CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(
        Cursor::new(bytes),
        decoder,
        CodingErrorPolicy::Strict,
        1,
    );

    let mut output = String::new();
    let count = reader.read_to_string(&mut output)?;

    assert_eq!(3, count);
    assert_eq!("Aé🙂", output);
    Ok(())
}

#[test]
fn test_buffered_reader_preserves_need_output_for_expanding_decoder()
-> std::io::Result<()> {
    let mut reader = BufferedReader::with_capacity(
        Cursor::new(vec![1]),
        ExpandingDecoder,
        CodingErrorPolicy::Strict,
        1,
    );
    let mut output = String::new();
    assert_eq!(5, reader.read_to_string(&mut output)?);
    assert_eq!("xxxxx", output);
    Ok(())
}

#[test]
fn test_buffered_reader_accessors() -> std::io::Result<()> {
    let decoder =
        CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::new(
        Cursor::new("abc\n".as_bytes().to_vec()),
        decoder,
        CodingErrorPolicy::Strict,
    );

    assert_eq!(0, reader.inner().position());
    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("abc\n", line);

    let (inner, unread, _decoder, pending_chars) = reader.into_parts();
    assert_eq!(4, inner.position());
    assert!(unread.readable().is_empty());
    assert!(pending_chars.is_empty());
    Ok(())
}

#[test]
fn test_buffered_reader_into_parts_preserves_unreturned_characters()
-> std::io::Result<()> {
    let decoder =
        CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(
        Cursor::new(b"abc".to_vec()),
        decoder,
        CodingErrorPolicy::Strict,
        4,
    );

    assert_eq!(Some('a'), reader.read_char()?);

    let (inner, unread, _decoder, pending_chars) = reader.into_parts();
    assert_eq!(3, inner.position());
    assert!(unread.readable().is_empty());
    assert_eq!(['b', 'c'], pending_chars.as_slice());
    Ok(())
}

#[test]
fn test_buffered_reader_read_chars_with_zero_limit_does_not_read()
-> std::io::Result<()> {
    let decoder =
        CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::new(
        Cursor::new("abc".as_bytes().to_vec()),
        decoder,
        CodingErrorPolicy::Strict,
    );
    let mut chars = Vec::new();

    assert_eq!(0, reader.read_chars(&mut chars, 0)?);
    assert!(chars.is_empty());
    assert_eq!(0, reader.inner().position());
    Ok(())
}

#[test]
fn test_buffered_reader_emits_decoder_finish_output() -> std::io::Result<()> {
    let mut reader = BufferedReader::with_capacity(
        Cursor::new(Vec::new()),
        FinishCharDecoder,
        CodingErrorPolicy::Strict,
        1,
    );

    assert_eq!(Some('!'), reader.read_char()?);
    assert_eq!(None, reader.read_char()?);
    assert_eq!(None, reader.read_char()?);
    Ok(())
}

#[test]
fn test_buffered_reader_runs_complete_lifecycle_on_first_read()
-> std::io::Result<()> {
    let mut reader = BufferedReader::with_capacity(
        Cursor::new(b"A".to_vec()),
        LifecycleDecoder::default(),
        CodingErrorPolicy::Strict,
        1,
    );

    let mut output = String::new();
    reader.read_to_string(&mut output)?;

    assert_eq!("^~+*-A!", output);
    Ok(())
}

#[test]
fn test_buffered_reader_runs_complete_lifecycle_for_empty_stream()
-> std::io::Result<()> {
    let mut reader = BufferedReader::with_capacity(
        Cursor::new(Vec::new()),
        LifecycleDecoder::default(),
        CodingErrorPolicy::Strict,
        1,
    );

    let mut output = String::new();
    reader.read_to_string(&mut output)?;

    assert_eq!("^~+*-!", output);
    Ok(())
}

#[test]
fn test_buffered_reader_reports_finish_capacity_errors() {
    let mut reader = BufferedReader::new(
        Cursor::new(Vec::new()),
        OverflowFinishDecoder,
        CodingErrorPolicy::Strict,
    );

    let error = reader
        .read_char()
        .expect_err("finish capacity errors must become I/O errors");

    assert_eq!(ErrorKind::OutOfMemory, error.kind());
}

#[test]
fn test_buffered_reader_propagates_finish_errors() {
    let mut reader = BufferedReader::new(
        Cursor::new(Vec::new()),
        ErrorFinishDecoder,
        CodingErrorPolicy::Strict,
    );

    let error = reader
        .read_char()
        .expect_err("decoder finish errors must become I/O errors");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_buffered_reader_propagates_reset_errors() {
    let mut reader = BufferedReader::new(
        Cursor::new(Vec::new()),
        ErrorResetDecoder,
        CodingErrorPolicy::Strict,
    );

    let error = reader
        .read_char()
        .expect_err("decoder reset errors must become I/O errors");

    assert_eq!(ErrorKind::InvalidData, error.kind());

    let mut reader = BufferedReader::new(
        Cursor::new(Vec::new()),
        ErrorResetDecoder,
        CodingErrorPolicy::Strict,
    );
    let mut output = Vec::new();
    let error = reader
        .read_chars(&mut output, 1)
        .expect_err("bulk reads must propagate decoder reset errors");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_buffered_reader_reports_reset_capacity_errors() {
    let mut reader = BufferedReader::new(
        Cursor::new(Vec::new()),
        OverflowResetDecoder,
        CodingErrorPolicy::Strict,
    );

    let error = reader
        .read_char()
        .expect_err("decoder reset capacity errors must become I/O errors");

    assert_eq!(ErrorKind::OutOfMemory, error.kind());
}

#[test]
fn test_buffered_reader_replaces_incomplete_eof_tail() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(
        Utf8Codec,
        CharsetDecodePolicy::replace('\u{FFFD}'),
    );
    let mut reader = BufferedReader::with_capacity(
        Cursor::new(vec![0xE2, 0x82]),
        decoder,
        CodingErrorPolicy::Replace,
        1,
    );

    let mut output = String::new();
    let count = reader.read_to_string(&mut output)?;

    assert_eq!(1, count);
    assert_eq!("\u{FFFD}", output);
    Ok(())
}

#[test]
fn test_buffered_reader_reports_strict_incomplete_eof_tail() {
    let decoder =
        CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::with_capacity(
        Cursor::new(vec![0xE2, 0x82]),
        decoder,
        CodingErrorPolicy::Strict,
        1,
    );

    let error = reader
        .read_char()
        .expect_err("strict incomplete EOF should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}
