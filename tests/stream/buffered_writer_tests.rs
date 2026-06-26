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

use qubit_codec::{
    CapacityError,
    TranscodeEncoder,
    TranscodeError,
    TranscodeProgress,
    Transcoder,
};
use qubit_codec_text::{
    AsciiCodec,
    CharsetEncodePolicy,
    CharsetEncoder,
};
use qubit_io_text::{
    BufferedWriter,
    LineEnding,
    TextWrite,
    Utf8Codec,
};

#[derive(Debug, Default)]
struct PartialEncoder;

impl Transcoder<char, u8> for PartialEncoder {
    type Error = std::io::Error;

    fn max_transcode_output_len(
        &self,
        input_len: usize,
    ) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn reset(
        &mut self,
        output: &mut [u8],
        output_index: usize,
    ) -> Result<usize, TranscodeError<Self::Error>> {
        TranscodeError::<Self::Error>::ensure_output_index(
            output.len(),
            output_index,
        )?;
        Ok(0)
    }

    fn transcode(
        &mut self,
        _input: &[char],
        _input_index: usize,
        _output: &mut [u8],
        _output_index: usize,
    ) -> Result<TranscodeProgress, TranscodeError<Self::Error>> {
        Ok(TranscodeProgress::complete(0, 0))
    }

    fn finish(
        &mut self,
        output: &mut [u8],
        output_index: usize,
    ) -> Result<usize, TranscodeError<Self::Error>> {
        TranscodeError::<Self::Error>::ensure_output_index(
            output.len(),
            output_index,
        )?;
        Ok(0)
    }
}

impl TranscodeEncoder<char, u8> for PartialEncoder {}

#[derive(Debug, Default)]
struct FinishByteEncoder;

impl Transcoder<char, u8> for FinishByteEncoder {
    type Error = std::io::Error;

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
        output: &mut [u8],
        output_index: usize,
    ) -> Result<usize, TranscodeError<Self::Error>> {
        TranscodeError::<Self::Error>::ensure_output_index(
            output.len(),
            output_index,
        )?;
        Ok(0)
    }

    fn transcode(
        &mut self,
        input: &[char],
        input_index: usize,
        output: &mut [u8],
        output_index: usize,
    ) -> Result<TranscodeProgress, TranscodeError<Self::Error>> {
        TranscodeError::<Self::Error>::ensure_output_index(
            output.len(),
            output_index,
        )?;
        let mut read = 0;
        let mut written = 0;
        while input_index + read < input.len()
            && output_index + written < output.len()
        {
            output[output_index + written] = input[input_index + read] as u8;
            read += 1;
            written += 1;
        }
        Ok(TranscodeProgress::complete(read, written))
    }

    fn finish(
        &mut self,
        output: &mut [u8],
        output_index: usize,
    ) -> Result<usize, TranscodeError<Self::Error>> {
        TranscodeError::<Self::Error>::ensure_output_index(
            output.len(),
            output_index,
        )?;
        output[output_index] = b'!';
        Ok(1)
    }
}

impl TranscodeEncoder<char, u8> for FinishByteEncoder {}

#[test]
fn test_buffered_writer_encodes_utf8_into_shared_output_buffer()
-> std::io::Result<()> {
    let encoder =
        CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
            .expect("strict UTF-8 encoder should be constructible");
    let mut writer =
        BufferedWriter::with_capacity(Cursor::new(Vec::new()), encoder, 1);

    writer.write_str("Aé🙂")?;
    let cursor = writer.into_inner()?;

    assert_eq!("Aé🙂".as_bytes(), cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_accessors_empty_writes_and_finish_state()
-> std::io::Result<()> {
    let encoder =
        CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
            .expect("strict UTF-8 encoder should be constructible");
    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), encoder);

    assert_eq!(LineEnding::Lf, writer.configured_line_ending());
    assert_eq!(LineEnding::Lf, writer.line_ending());
    assert!(writer.inner().get_ref().is_empty());
    writer.inner_mut().get_mut().extend_from_slice(b"prefix:");
    writer.inner_mut().set_position(7);

    writer.write_chars(&[])?;
    writer.write_str("")?;
    writer.write_char('A')?;
    writer.finish()?;
    writer.finish()?;

    let error = writer
        .write_char('B')
        .expect_err("writes after finish must be rejected");
    assert_eq!(ErrorKind::InvalidInput, error.kind());

    let cursor = writer.into_inner()?;
    assert_eq!(b"prefix:A", cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_flushes_exact_string_chunks() -> std::io::Result<()> {
    let encoder =
        CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
            .expect("strict UTF-8 encoder should be constructible");
    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), encoder);
    let text = "a".repeat(256);

    writer.write_str(text.as_str())?;
    let cursor = writer.into_inner()?;

    assert_eq!(text.as_bytes(), cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_reports_incomplete_encoder_consumption() {
    let mut writer =
        BufferedWriter::new(Cursor::new(Vec::new()), PartialEncoder);

    let error = writer
        .write_chars(&['x'])
        .expect_err("encoders must consume complete requested input");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_buffered_writer_emits_finish_output() -> std::io::Result<()> {
    let mut writer =
        BufferedWriter::new(Cursor::new(Vec::new()), FinishByteEncoder);

    writer.finish()?;
    let cursor = writer.into_inner()?;

    assert_eq!(b"!", cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_maps_encoder_errors_to_io_errors() {
    let encoder =
        CharsetEncoder::with_policy(AsciiCodec, CharsetEncodePolicy::report())
            .expect("strict ASCII encoder should be constructible");
    let mut writer =
        BufferedWriter::with_capacity(Cursor::new(Vec::new()), encoder, 1);

    let error = writer
        .write_char('🙂')
        .expect_err("strict ASCII should reject non-ASCII characters");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_buffered_writer_applies_configured_line_ending() -> std::io::Result<()>
{
    let encoder =
        CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
            .expect("strict UTF-8 encoder should be constructible");
    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), encoder)
        .with_line_ending(LineEnding::CrLf);

    writer.write_line("line")?;
    let cursor = writer.into_inner()?;

    assert_eq!(b"line\r\n", cursor.into_inner().as_slice());
    Ok(())
}
