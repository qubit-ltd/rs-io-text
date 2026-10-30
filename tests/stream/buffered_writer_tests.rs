// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;
use std::io::ErrorKind;

use qubit_codec::CapacityError;
use qubit_codec::TranscodeDomainError;
use qubit_codec::TranscodeEncodeError;
use qubit_codec::TranscodeEncoder;
use qubit_codec::TranscodeFailure;
use qubit_codec::TranscodeProgress;
use qubit_codec::Transcoder;
use qubit_codec_text::AsciiCodec;
use qubit_codec_text::CharsetEncodeError;
use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::CharsetEncoder;
use qubit_codec_text::Utf8Codec;
use qubit_io::Output;
use qubit_io_text::BufferedTextWriter;
use qubit_io_text::LineEnding;
use qubit_io_text::TextWriter;

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
        TranscodeFailure::ensure_output_index(output.len(), output_index)?;
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
        TranscodeFailure::ensure_output_index(output.len(), output_index)?;
        Ok(0)
    }
}

impl TranscodeEncoder for PartialEncoder {
    type EncodeError = std::io::Error;
}

#[derive(Debug, Default)]
struct MarkerUnencodableEncoder;

impl Transcoder for MarkerUnencodableEncoder {
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
        for (index, ch) in input.iter().enumerate().skip(input_index) {
            if *ch == '#' {
                return Err(TranscodeEncodeError::unencodable(index, *ch));
            }
            output[output_index + index - input_index] = *ch as u8;
        }
        let count = input.len() - input_index;
        Ok(TranscodeProgress::complete(count, count))
    }

    fn finish(&mut self, _output: &mut [u8], _output_index: usize) -> Result<usize, Self::Error> {
        Ok(0)
    }
}

impl TranscodeEncoder for MarkerUnencodableEncoder {
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
        TranscodeFailure::ensure_output_index(output.len(), output_index)?;
        Ok(0)
    }

    fn transcode(
        &mut self,
        input: &[char],
        input_index: usize,
        output: &mut [u8],
        output_index: usize,
    ) -> Result<TranscodeProgress, Self::Error> {
        TranscodeFailure::ensure_output_index(output.len(), output_index)?;
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
        TranscodeFailure::ensure_output_index(output.len(), output_index)?;
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
            core::num::NonZeroUsize::MIN,
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

#[derive(Debug)]
struct FlushFailOnceOutput {
    bytes: Vec<u8>,
    fail_flush: bool,
    flushed: bool,
}

impl Default for FlushFailOnceOutput {
    fn default() -> Self {
        Self {
            bytes: Vec::new(),
            fail_flush: true,
            flushed: false,
        }
    }
}

impl Output for FlushFailOnceOutput {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, input: &[u8], index: usize, count: usize) -> std::io::Result<usize> {
        self.bytes.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.fail_flush {
            self.fail_flush = false;
            return Err(std::io::Error::new(ErrorKind::BrokenPipe, "scripted flush failure"));
        }
        self.flushed = true;
        Ok(())
    }
}

#[derive(Default)]
struct PanicOutput {
    bytes: Vec<u8>,
}

impl Output for PanicOutput {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, input: &[u8], index: usize, count: usize) -> std::io::Result<usize> {
        let committed = count.min(1);
        self.bytes.extend_from_slice(&input[index..index + committed]);
        panic!("scripted output panic after committing a prefix");
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_buffered_writer_encodes_utf8_into_shared_output_buffer() -> std::io::Result<()> {
    let encoder = CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
        .expect("strict UTF-8 encoder should be constructible");
    let mut writer = BufferedTextWriter::with_capacity(Cursor::new(Vec::new()), encoder, 1);

    writer.write_str("Aé🙂")?;
    writer.finish()?;
    let (cursor, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!("Aé🙂".as_bytes(), cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_accessors_empty_writes_and_finish_state() -> std::io::Result<()> {
    let encoder = CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
        .expect("strict UTF-8 encoder should be constructible");
    let mut inner = Cursor::new(b"prefix:".to_vec());
    inner.set_position(7);
    let mut writer = BufferedTextWriter::new(inner, encoder);

    assert_eq!(LineEnding::Lf, writer.configured_line_ending());
    assert_eq!(LineEnding::Lf, writer.line_ending());
    assert_eq!(b"prefix:", writer.inner().get_ref().as_slice());
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
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };
    assert!(pending.readable().is_empty());
    assert_eq!(b"prefix:A", cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_flushes_exact_string_chunks() -> std::io::Result<()> {
    let encoder = CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
        .expect("strict UTF-8 encoder should be constructible");
    let mut writer = BufferedTextWriter::new(Cursor::new(Vec::new()), encoder);
    let text = "a".repeat(256);

    writer.write_str(text.as_str())?;
    writer.finish()?;
    let (cursor, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!(text.as_bytes(), cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_reports_incomplete_encoder_consumption() {
    let mut writer = BufferedTextWriter::new(Cursor::new(Vec::new()), PartialEncoder);

    let error = writer
        .write_chars(&['x', 'y'])
        .expect_err("encoders must consume complete requested input");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_buffered_writer_emits_finish_output() -> std::io::Result<()> {
    let mut writer = BufferedTextWriter::new(Cursor::new(Vec::new()), FinishByteEncoder);

    writer.finish()?;
    let (cursor, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!(b"!", cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_runs_complete_lifecycle_on_first_write() -> std::io::Result<()> {
    let mut writer = BufferedTextWriter::with_capacity(Cursor::new(Vec::new()), LifecycleEncoder::default(), 1);

    writer.write_char('A')?;
    writer.write_char('B')?;
    writer.finish()?;
    let (cursor, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!(b"^AB!", cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_runs_complete_lifecycle_for_empty_stream() -> std::io::Result<()> {
    let mut writer = BufferedTextWriter::with_capacity(Cursor::new(Vec::new()), LifecycleEncoder::default(), 1);

    writer.finish()?;
    let (cursor, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!(b"^!", cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_maps_encoder_errors_to_io_errors() {
    let encoder = CharsetEncoder::with_policy(AsciiCodec, CharsetEncodePolicy::report())
        .expect("strict ASCII encoder should be constructible");
    let mut writer = BufferedTextWriter::with_capacity(Cursor::new(Vec::new()), encoder, 1);

    let error = writer
        .write_char('🙂')
        .expect_err("strict ASCII should reject non-ASCII characters");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_buffered_writer_reports_call_relative_main_error_index() {
    let encoder = CharsetEncoder::with_policy(AsciiCodec, CharsetEncodePolicy::report())
        .expect("strict ASCII encoder should be constructible");
    let mut writer = BufferedTextWriter::new(Cursor::new(Vec::new()), encoder);
    let text = format!("{}中", "A".repeat(300));

    let error = writer.write_str(&text).expect_err("strict ASCII rejects 中");
    assert_eq!(ErrorKind::InvalidInput, error.kind());
    let encoded = error
        .get_ref()
        .and_then(|source| source.downcast_ref::<TranscodeEncodeError<CharsetEncodeError, char>>())
        .expect("preserve the transcode error type");
    let domain = encoded.domain_error_ref().expect("main encoder error");
    assert_eq!(Some(300), domain.input_index());
    assert_eq!(Some('中' as u32), encoded.domain_ref().expect("domain source").value());
}

#[test]
fn test_buffered_writer_offsets_unencodable_error_after_chunk() {
    let mut writer = BufferedTextWriter::new(Cursor::new(Vec::new()), MarkerUnencodableEncoder);

    let error = writer
        .write_str(&format!("{}#", "A".repeat(256)))
        .expect_err("marker is not encodable");
    assert_eq!(ErrorKind::InvalidInput, error.kind());
    let encoded = error
        .get_ref()
        .and_then(|source| source.downcast_ref::<TranscodeEncodeError<std::io::Error, char>>())
        .expect("preserve the transcode error type");
    assert_eq!(Some((256, Some(&'#'))), encoded.unencodable_ref());
}

#[test]
fn test_buffered_writer_reports_reset_capacity_errors() {
    let mut writer = BufferedTextWriter::new(Cursor::new(Vec::new()), OverflowResetEncoder);

    let error = writer
        .write_char('A')
        .expect_err("reset capacity errors must become I/O errors");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_buffered_writer_reports_reset_domain_errors() {
    let mut writer = BufferedTextWriter::new(Cursor::new(Vec::new()), ErrorResetEncoder);

    let error = writer
        .write_char('A')
        .expect_err("reset domain errors must become I/O errors");

    assert_eq!(ErrorKind::InvalidInput, error.kind());

    let mut writer = BufferedTextWriter::new(Cursor::new(Vec::new()), ErrorResetEncoder);
    let error = writer
        .write_str(&"a".repeat(256))
        .expect_err("chunk flush must propagate reset domain errors");
    assert_eq!(ErrorKind::InvalidInput, error.kind());

    let mut writer = BufferedTextWriter::new(Cursor::new(Vec::new()), ErrorResetEncoder);
    let error = writer
        .write_line("line")
        .expect_err("line content errors must stop before the line ending");
    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_buffered_writer_reports_finish_domain_errors() {
    let mut writer = BufferedTextWriter::new(Cursor::new(Vec::new()), ErrorFinishEncoder);

    let error = writer
        .finish()
        .expect_err("finish domain errors must become I/O errors");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_buffered_writer_finish_propagates_lazy_reset_errors() {
    let mut writer = BufferedTextWriter::new(Cursor::new(Vec::new()), OverflowResetEncoder);

    let error = writer.finish().expect_err("finish must propagate lazy reset errors");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert!(writer.inner().get_ref().is_empty());
}

#[test]
fn test_buffered_writer_finish_error_leaves_writer_available_for_retry() {
    let mut writer = BufferedTextWriter::new(Cursor::new(Vec::new()), ErrorFinishEncoder);

    let error = writer.finish().expect_err("finish should report the encoder error");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
    assert!(writer.inner().get_ref().is_empty());
}

#[test]
fn test_buffered_writer_retries_finish_after_flush_failure() -> std::io::Result<()> {
    let mut writer = BufferedTextWriter::new(FlushFailOnceOutput::default(), LifecycleEncoder::default());

    writer.write_char('A')?;
    let error = writer
        .finish()
        .expect_err("first finish should report the scripted flush failure");
    assert_eq!(ErrorKind::BrokenPipe, error.kind());

    let error = writer
        .write_str("extra")
        .expect_err("finalized writer must reject new text after a flush error");
    assert_eq!(ErrorKind::InvalidInput, error.kind());

    writer.finish()?;
    let (output, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!(b"^A!", output.bytes.as_slice());
    assert!(output.flushed);
    Ok(())
}

#[test]
fn test_buffered_writer_reuses_string_path_after_empty_and_across_chunks() -> std::io::Result<()> {
    let encoder = CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
        .expect("strict UTF-8 encoder should be constructible");
    let mut writer = BufferedTextWriter::with_capacity(Cursor::new(Vec::new()), encoder, 8);
    writer.write_str("")?;
    let long_text = "中🙂é".repeat(110);
    assert!(long_text.chars().count() >= 300);
    writer.write_str("前")?;
    writer.write_char('字')?;
    writer.write_str(&long_text)?;
    writer.write_char('🙂')?;
    writer.write_str("后終")?;
    writer.finish()?;

    let (cursor, recovery) = writer.into_parts();
    let qubit_io::BufferedOutputRecovery::Pending(pending) = recovery else {
        panic!("unexpected indeterminate recovery")
    };
    assert!(pending.readable().is_empty());
    assert_eq!(
        format!("前字{long_text}🙂后終").as_bytes(),
        cursor.into_inner().as_slice()
    );
    Ok(())
}

#[test]
fn test_buffered_writer_char_slice_capacity_is_independent_of_total_length() -> std::io::Result<()> {
    for alphabet in [vec!['A'], vec!['中', '🙂', 'é']] {
        let mut capacities = Vec::new();
        for len in [256, 65_536] {
            let chars: Vec<char> = alphabet.iter().copied().cycle().take(len).collect();
            let expected: String = chars.iter().copied().collect();
            let encoder = CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
                .expect("valid UTF-8 report encoder");
            let mut writer = BufferedTextWriter::with_capacity(Cursor::new(Vec::new()), encoder, 8);
            writer.write_chars(&chars)?;
            writer.finish()?;
            let (output, recovery) = writer.into_parts();
            let qubit_io::BufferedOutputRecovery::Pending(buffer) = recovery else {
                panic!("ordinary writes must have determinate recovery");
            };
            assert!(buffer.readable().is_empty());
            assert_eq!(expected.as_bytes(), output.get_ref().as_slice());
            capacities.push(buffer.capacity());
        }
        assert_eq!(capacities[0], capacities[1], "input length enlarged output cache");
    }
    Ok(())
}

#[test]
fn test_buffered_writer_char_slice_boundaries_preserve_lifecycle() -> std::io::Result<()> {
    for len in [0, 1, 255, 256, 257] {
        let chars = vec!['A'; len];
        let mut writer = BufferedTextWriter::with_capacity(Cursor::new(Vec::new()), LifecycleEncoder::default(), 1);
        writer.write_chars(&[])?;
        writer.write_char('B')?;
        writer.write_chars(&chars)?;
        writer.write_str("C")?;
        writer.finish()?;
        let (output, recovery) = writer.into_parts();
        let qubit_io::BufferedOutputRecovery::Pending(buffer) = recovery else {
            panic!("ordinary writes must have determinate recovery");
        };
        assert!(buffer.readable().is_empty());
        assert_eq!(
            format!("^B{}C!", "A".repeat(len)).as_bytes(),
            output.get_ref().as_slice()
        );
    }
    Ok(())
}

#[test]
fn test_buffered_writer_clears_string_staging_after_encode_error() -> std::io::Result<()> {
    let mut writer = BufferedTextWriter::with_capacity(Cursor::new(Vec::new()), MarkerUnencodableEncoder, 1);
    let text = format!("{}#", "A".repeat(256));
    let error = writer
        .write_str(&text)
        .expect_err("the marker should fail encoding after the first chunk");
    assert_eq!(ErrorKind::InvalidInput, error.kind());

    writer.write_str("Z")?;
    writer.finish()?;
    let (output, recovery) = writer.into_parts();
    let qubit_io::BufferedOutputRecovery::Pending(pending) = recovery else {
        panic!("unexpected indeterminate recovery")
    };
    assert!(pending.readable().is_empty());
    assert_eq!(
        format!("{}Z", "A".repeat(256)).as_bytes(),
        output.into_inner().as_slice()
    );
    Ok(())
}

#[test]
#[should_panic(expected = "reset wrote beyond its bound")]
fn test_buffered_writer_rejects_overreported_reset_output() {
    let mut writer = BufferedTextWriter::new(Cursor::new(Vec::new()), OverreportedResetEncoder);

    let _ = writer.write_char('A');
}

#[test]
fn test_buffered_writer_applies_configured_line_ending() -> std::io::Result<()> {
    let encoder = CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
        .expect("strict UTF-8 encoder should be constructible");
    let mut writer = BufferedTextWriter::new(Cursor::new(Vec::new()), encoder).with_line_ending(LineEnding::CrLf);

    writer.write_line("line")?;
    writer.finish()?;
    let (cursor, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!(b"line\r\n", cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_into_parts_preserves_indeterminate_after_inner_write_panics() {
    let mut writer = BufferedTextWriter::new(PanicOutput::default(), CharsetEncoder::new(Utf8Codec));
    writer.write_str("text").expect("the encoded value should be buffered");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| TextWriter::flush(&mut writer)));
    assert!(result.is_err());

    let (inner, recovery) = writer.into_parts();
    assert_eq!(b"t", inner.bytes.as_slice());
    assert!(matches!(recovery, qubit_io::BufferedOutputRecovery::Indeterminate(_)));
}
