// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Cursor, ErrorKind};

use qubit_codec::{
    CapacityError, TranscodeDomainError, TranscodeEncodeError, TranscodeEncoder, TranscodeProgress,
    Transcoder,
};
use qubit_codec_text::{AsciiCodec, CharsetEncodePolicy, CharsetEncoder, Utf8Codec};
use qubit_io_text::{BufferedWriter, LineEnding, TextWrite};

#[derive(Debug, Default)]
struct PartialEncoder;

impl Transcoder for PartialEncoder {
    type Input = char;
    type Output = u8;
    type Error = TranscodeEncodeError<std::io::Error, char>;

    fn max_transcode_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn reset(&mut self, output: &mut [u8], output_index: usize) -> Result<usize, Self::Error> {
        Self::Error::ensure_output_index(output.len(), output_index)?;
        Ok(0)
    }

    fn transcode(
        &mut self,
        input: &[char],
        input_index: usize,
        _output: &mut [u8],
        _output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        let read = usize::from(input_index < input.len());
        Ok(TranscodeProgress::complete(read, 0))
    }

    fn finish(&mut self, output: &mut [u8], output_index: usize) -> Result<usize, Self::Error> {
        Self::Error::ensure_output_index(output.len(), output_index)?;
        Ok(0)
    }
}

impl TranscodeEncoder for PartialEncoder {
    type EncodeError = std::io::Error;
}

#[derive(Debug, Default)]
struct FinishByteEncoder;

impl Transcoder for FinishByteEncoder {
    type Input = char;
    type Output = u8;
    type Error = TranscodeEncodeError<std::io::Error, char>;

    fn max_transcode_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn max_finish_output_len(&self) -> Result<usize, CapacityError> {
        Ok(1)
    }

    fn reset(&mut self, output: &mut [u8], output_index: usize) -> Result<usize, Self::Error> {
        Self::Error::ensure_output_index(output.len(), output_index)?;
        Ok(0)
    }

    fn transcode(
        &mut self,
        input: &[char],
        input_index: usize,
        output: &mut [u8],
        output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        Self::Error::ensure_output_index(output.len(), output_index)?;
        let mut read = 0;
        let mut written = 0;
        while input_index + read < input.len() && output_index + written < output.len() {
            output[output_index + written] = input[input_index + read] as u8;
            read += 1;
            written += 1;
        }
        Ok(TranscodeProgress::complete(read, written))
    }

    fn finish(&mut self, output: &mut [u8], output_index: usize) -> Result<usize, Self::Error> {
        Self::Error::ensure_output_index(output.len(), output_index)?;
        output[output_index] = b'!';
        Ok(1)
    }
}

impl TranscodeEncoder for FinishByteEncoder {
    type EncodeError = std::io::Error;
}

#[derive(Debug, Default)]
struct LifecycleEncoder {
    started: bool,
    finished: bool,
}

impl Transcoder for LifecycleEncoder {
    type Input = char;
    type Output = u8;
    type Error = TranscodeEncodeError<std::io::Error, char>;

    fn max_reset_output_len(&self) -> Result<usize, CapacityError> {
        Ok(1)
    }

    fn max_transcode_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn max_finish_output_len(&self) -> Result<usize, CapacityError> {
        Ok(1)
    }

    fn reset(&mut self, output: &mut [u8], output_index: usize) -> Result<usize, Self::Error> {
        assert!(!self.started, "encoder reset must run exactly once");
        output[output_index] = b'^';
        self.started = true;
        Ok(1)
    }

    fn transcode(
        &mut self,
        input: &[char],
        input_index: usize,
        output: &mut [u8],
        output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        assert!(self.started, "encoder must reset before transcode");
        let mut read = 0;
        let mut written = 0;
        while input_index + read < input.len() && output_index + written < output.len() {
            output[output_index + written] = input[input_index + read] as u8;
            read += 1;
            written += 1;
        }
        if input_index + read == input.len() {
            return Ok(TranscodeProgress::complete(read, written));
        }
        Ok(TranscodeProgress::need_output(
            output_index + written,
            core::num::NonZeroUsize::MIN,
            0,
            read,
            written,
        ))
    }

    fn finish(&mut self, output: &mut [u8], output_index: usize) -> Result<usize, Self::Error> {
        assert!(self.started, "encoder must reset before finish");
        assert!(!self.finished, "encoder finish must run exactly once");
        output[output_index] = b'!';
        self.finished = true;
        Ok(1)
    }
}

impl TranscodeEncoder for LifecycleEncoder {
    type EncodeError = std::io::Error;
}

#[derive(Debug, Default)]
struct OverflowResetEncoder;

impl Transcoder for OverflowResetEncoder {
    type Input = char;
    type Output = u8;
    type Error = TranscodeEncodeError<std::io::Error, char>;

    fn max_reset_output_len(&self) -> Result<usize, CapacityError> {
        Err(CapacityError::OutputLengthOverflow)
    }

    fn max_transcode_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn reset(&mut self, _output: &mut [u8], _output_index: usize) -> Result<usize, Self::Error> {
        unreachable!("capacity planning fails before reset")
    }

    fn transcode(
        &mut self,
        _input: &[char],
        _input_index: usize,
        _output: &mut [u8],
        _output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        unreachable!("capacity planning fails before transcode")
    }

    fn finish(&mut self, _output: &mut [u8], _output_index: usize) -> Result<usize, Self::Error> {
        unreachable!("capacity planning fails before finish")
    }
}

impl TranscodeEncoder for OverflowResetEncoder {
    type EncodeError = std::io::Error;
}

#[derive(Debug, Default)]
struct OverreportedResetEncoder;

impl Transcoder for OverreportedResetEncoder {
    type Input = char;
    type Output = u8;
    type Error = TranscodeEncodeError<std::io::Error, char>;

    fn max_transcode_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn reset(&mut self, _output: &mut [u8], _output_index: usize) -> Result<usize, Self::Error> {
        Ok(1)
    }

    fn transcode(
        &mut self,
        _input: &[char],
        _input_index: usize,
        _output: &mut [u8],
        _output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        unreachable!("reset contract violation fails before transcode")
    }

    fn finish(&mut self, _output: &mut [u8], _output_index: usize) -> Result<usize, Self::Error> {
        unreachable!("reset contract violation fails before finish")
    }
}

impl TranscodeEncoder for OverreportedResetEncoder {
    type EncodeError = std::io::Error;
}

#[derive(Debug, Default)]
struct ErrorResetEncoder;

impl Transcoder for ErrorResetEncoder {
    type Input = char;
    type Output = u8;
    type Error = TranscodeEncodeError<std::io::Error, char>;

    fn max_transcode_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn reset(&mut self, _output: &mut [u8], _output_index: usize) -> Result<usize, Self::Error> {
        Err(TranscodeEncodeError::Domain(TranscodeDomainError::Reset {
            source: std::io::Error::other("reset failed"),
        }))
    }

    fn transcode(
        &mut self,
        _input: &[char],
        _input_index: usize,
        _output: &mut [u8],
        _output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        unreachable!("reset failure prevents transcoding")
    }

    fn finish(&mut self, _output: &mut [u8], _output_index: usize) -> Result<usize, Self::Error> {
        unreachable!("reset failure prevents finishing")
    }
}

impl TranscodeEncoder for ErrorResetEncoder {
    type EncodeError = std::io::Error;
}

#[derive(Debug, Default)]
struct ErrorFinishEncoder;

impl Transcoder for ErrorFinishEncoder {
    type Input = char;
    type Output = u8;
    type Error = TranscodeEncodeError<std::io::Error, char>;

    fn max_transcode_output_len(&self, input_len: usize) -> Result<usize, CapacityError> {
        Ok(input_len)
    }

    fn reset(&mut self, _output: &mut [u8], _output_index: usize) -> Result<usize, Self::Error> {
        Ok(0)
    }

    fn transcode(
        &mut self,
        input: &[char],
        input_index: usize,
        output: &mut [u8],
        output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        let count = input.len() - input_index;
        for (offset, ch) in input[input_index..].iter().enumerate() {
            output[output_index + offset] = *ch as u8;
        }
        Ok(TranscodeProgress::complete(count, count))
    }

    fn finish(&mut self, _output: &mut [u8], _output_index: usize) -> Result<usize, Self::Error> {
        Err(TranscodeEncodeError::Domain(TranscodeDomainError::Finish {
            source: std::io::Error::other("finish failed"),
        }))
    }
}

impl TranscodeEncoder for ErrorFinishEncoder {
    type EncodeError = std::io::Error;
}

#[test]
fn test_buffered_writer_encodes_utf8_into_shared_output_buffer() -> std::io::Result<()> {
    let encoder = CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
        .expect("strict UTF-8 encoder should be constructible");
    let mut writer = BufferedWriter::with_capacity(Cursor::new(Vec::new()), encoder, 1);

    writer.write_str("Aé🙂")?;
    writer.finish()?;
    let (cursor, pending) = writer.into_parts();

    assert!(pending.is_empty());
    assert_eq!("Aé🙂".as_bytes(), cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_accessors_empty_writes_and_finish_state() -> std::io::Result<()> {
    let encoder = CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
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
    let error = writer
        .write_str("C")
        .expect_err("string writes after finish must be rejected");
    assert_eq!(ErrorKind::InvalidInput, error.kind());

    let (cursor, pending) = writer.into_parts();
    assert!(pending.is_empty());
    assert_eq!(b"prefix:A", cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_flushes_exact_string_chunks() -> std::io::Result<()> {
    let encoder = CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
        .expect("strict UTF-8 encoder should be constructible");
    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), encoder);
    let text = "a".repeat(256);

    writer.write_str(text.as_str())?;
    writer.finish()?;
    let (cursor, pending) = writer.into_parts();

    assert!(pending.is_empty());
    assert_eq!(text.as_bytes(), cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_reports_incomplete_encoder_consumption() {
    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), PartialEncoder);

    let error = writer
        .write_chars(&['x', 'y'])
        .expect_err("encoders must consume complete requested input");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_buffered_writer_emits_finish_output() -> std::io::Result<()> {
    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), FinishByteEncoder);

    writer.finish()?;
    let (cursor, pending) = writer.into_parts();

    assert!(pending.is_empty());
    assert_eq!(b"!", cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_runs_complete_lifecycle_on_first_write() -> std::io::Result<()> {
    let mut writer =
        BufferedWriter::with_capacity(Cursor::new(Vec::new()), LifecycleEncoder::default(), 1);

    writer.write_char('A')?;
    writer.write_char('B')?;
    writer.finish()?;
    let (cursor, pending) = writer.into_parts();

    assert!(pending.is_empty());
    assert_eq!(b"^AB!", cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_runs_complete_lifecycle_for_empty_stream() -> std::io::Result<()> {
    let mut writer =
        BufferedWriter::with_capacity(Cursor::new(Vec::new()), LifecycleEncoder::default(), 1);

    writer.finish()?;
    let (cursor, pending) = writer.into_parts();

    assert!(pending.is_empty());
    assert_eq!(b"^!", cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_maps_encoder_errors_to_io_errors() {
    let encoder = CharsetEncoder::with_policy(AsciiCodec, CharsetEncodePolicy::report())
        .expect("strict ASCII encoder should be constructible");
    let mut writer = BufferedWriter::with_capacity(Cursor::new(Vec::new()), encoder, 1);

    let error = writer
        .write_char('🙂')
        .expect_err("strict ASCII should reject non-ASCII characters");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_buffered_writer_reports_reset_capacity_errors() {
    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), OverflowResetEncoder);

    let error = writer
        .write_char('A')
        .expect_err("reset capacity errors must become I/O errors");

    assert_eq!(ErrorKind::OutOfMemory, error.kind());
}

#[cfg(coverage)]
#[test]
fn test_buffered_writer_reports_reset_output_reserve_errors() {
    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), LifecycleEncoder::default());

    BufferedWriter::<Cursor<Vec<u8>>, LifecycleEncoder>::coverage_fail_next_reset_reserve();
    let error = writer
        .write_char('A')
        .expect_err("reset output reserve failure should be reported");

    assert_eq!(ErrorKind::OutOfMemory, error.kind());
}

#[test]
fn test_buffered_writer_reports_reset_domain_errors() {
    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), ErrorResetEncoder);

    let error = writer
        .write_char('A')
        .expect_err("reset domain errors must become I/O errors");

    assert_eq!(ErrorKind::InvalidInput, error.kind());

    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), ErrorResetEncoder);
    let error = writer
        .write_str(&"a".repeat(256))
        .expect_err("chunk flush must propagate reset domain errors");
    assert_eq!(ErrorKind::InvalidInput, error.kind());

    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), ErrorResetEncoder);
    let error = writer
        .write_line("line")
        .expect_err("line content errors must stop before the line ending");
    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_buffered_writer_reports_finish_domain_errors() {
    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), ErrorFinishEncoder);

    let error = writer
        .finish()
        .expect_err("finish domain errors must become I/O errors");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_buffered_writer_finish_propagates_lazy_reset_errors() {
    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), OverflowResetEncoder);

    let error = writer
        .finish()
        .expect_err("finish must propagate lazy reset errors");

    assert_eq!(ErrorKind::OutOfMemory, error.kind());
    assert!(writer.inner().get_ref().is_empty());
}

#[test]
fn test_buffered_writer_finish_error_leaves_writer_available_for_retry() {
    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), ErrorFinishEncoder);

    let error = writer
        .finish()
        .expect_err("finish should report the encoder error");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
    assert!(writer.inner().get_ref().is_empty());
}

#[test]
#[should_panic(expected = "reset wrote beyond its bound")]
fn test_buffered_writer_rejects_overreported_reset_output() {
    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), OverreportedResetEncoder);

    let _ = writer.write_char('A');
}

#[test]
fn test_buffered_writer_applies_configured_line_ending() -> std::io::Result<()> {
    let encoder = CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
        .expect("strict UTF-8 encoder should be constructible");
    let mut writer =
        BufferedWriter::new(Cursor::new(Vec::new()), encoder).with_line_ending(LineEnding::CrLf);

    writer.write_line("line")?;
    writer.finish()?;
    let (cursor, pending) = writer.into_parts();

    assert!(pending.is_empty());
    assert_eq!(b"line\r\n", cursor.into_inner().as_slice());
    Ok(())
}
