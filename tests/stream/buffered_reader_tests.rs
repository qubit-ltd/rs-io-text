// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Cursor, ErrorKind};

use qubit_codec::{CapacityError, TranscodeDecoder, TranscodeError, TranscodeProgress, Transcoder};
use qubit_codec_text::{CharsetDecodePolicy, CharsetDecoder};
use qubit_io_text::{BufferedReader, CodingErrorPolicy, TextLineRead, TextRead, Utf8Codec};

#[derive(Debug, Default)]
struct FinishCharDecoder;

impl Transcoder<u8, char> for FinishCharDecoder {
    type Error = std::io::Error;

    fn max_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn max_finish_output_len(&self) -> Result<usize, CapacityError> {
        Ok(8)
    }

    fn reset(
        &mut self,
        output: &mut [char],
        output_index: usize,
    ) -> Result<usize, TranscodeError<Self::Error>> {
        TranscodeError::<Self::Error>::ensure_output_index(output.len(), output_index)?;
        Ok(0)
    }

    fn transcode(
        &mut self,
        input: &[u8],
        input_index: usize,
        _output: &mut [char],
        _output_index: usize,
    ) -> Result<TranscodeProgress, TranscodeError<Self::Error>> {
        Ok(TranscodeProgress::complete(input.len() - input_index, 0))
    }

    fn finish(
        &mut self,
        output: &mut [char],
        output_index: usize,
    ) -> Result<usize, TranscodeError<Self::Error>> {
        TranscodeError::<Self::Error>::ensure_output_index(output.len(), output_index)?;
        output[output_index] = '!';
        Ok(1)
    }
}

impl TranscodeDecoder<u8, char> for FinishCharDecoder {}

#[derive(Debug, Default)]
struct OverflowFinishDecoder;

impl Transcoder<u8, char> for OverflowFinishDecoder {
    type Error = std::io::Error;

    fn max_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn max_finish_output_len(&self) -> Result<usize, CapacityError> {
        Err(CapacityError::OutputLengthOverflow)
    }

    fn reset(
        &mut self,
        output: &mut [char],
        output_index: usize,
    ) -> Result<usize, TranscodeError<Self::Error>> {
        TranscodeError::<Self::Error>::ensure_output_index(output.len(), output_index)?;
        Ok(0)
    }

    fn transcode(
        &mut self,
        input: &[u8],
        input_index: usize,
        _output: &mut [char],
        _output_index: usize,
    ) -> Result<TranscodeProgress, TranscodeError<Self::Error>> {
        Ok(TranscodeProgress::complete(input.len() - input_index, 0))
    }

    fn finish(
        &mut self,
        _output: &mut [char],
        _output_index: usize,
    ) -> Result<usize, TranscodeError<Self::Error>> {
        unreachable!("capacity planning fails before finish")
    }
}

impl TranscodeDecoder<u8, char> for OverflowFinishDecoder {}

#[test]
fn test_buffered_reader_decodes_utf8_across_single_byte_refills() -> std::io::Result<()> {
    let bytes = "Aé🙂".as_bytes().to_vec();
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader =
        BufferedReader::with_capacity(Cursor::new(bytes), decoder, CodingErrorPolicy::Strict, 1);

    let mut output = String::new();
    let count = reader.read_to_string(&mut output)?;

    assert_eq!(3, count);
    assert_eq!("Aé🙂", output);
    Ok(())
}

#[test]
fn test_buffered_reader_accessors_and_into_inner() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::new(
        Cursor::new("abc\n".as_bytes().to_vec()),
        decoder,
        CodingErrorPolicy::Strict,
    );

    assert_eq!(0, reader.inner().position());
    reader.inner_mut().set_position(1);

    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("bc\n", line);

    let inner = reader.into_inner();
    assert_eq!(4, inner.position());
    Ok(())
}

#[test]
fn test_buffered_reader_read_chars_with_zero_limit_does_not_read() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
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
fn test_buffered_reader_replaces_incomplete_eof_tail() -> std::io::Result<()> {
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::replace('\u{FFFD}'));
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
    let decoder = CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
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
