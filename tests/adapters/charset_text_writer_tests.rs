// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::io::ErrorKind;
use std::io::Write;
use std::num::NonZeroUsize;

use qubit_codec::Codec;
use qubit_codec::DecodeFailure;
use qubit_codec::TranscodeEncodeError;
use qubit_codec_text::AsciiCodec;
use qubit_codec_text::Charset;
use qubit_codec_text::CharsetCodec;
use qubit_codec_text::CharsetDecodeError;
use qubit_codec_text::CharsetEncodeError;
use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::CharsetEncodeResult;
use qubit_codec_text::Latin1Codec;
use qubit_codec_text::Utf8Codec;
use qubit_io::Output;
use qubit_io_text::CharsetTextWriter;
use qubit_io_text::CharsetWriteExt;
use qubit_io_text::LineEnding;
use qubit_io_text::TextWriter;

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("write failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("flush failed"))
    }
}

#[derive(Default)]
struct OneShotWriteFailure {
    bytes: Vec<u8>,
    calls: usize,
}

impl Write for OneShotWriteFailure {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if input.is_empty() {
            return Ok(0);
        }
        self.calls += 1;
        if self.calls > 1 {
            return Err(io::Error::new(ErrorKind::BrokenPipe, "one-shot output failure"));
        }
        self.bytes.push(input[0]);
        Ok(1)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Default)]
struct OutputOnlyWriter {
    bytes: Vec<u8>,
}

impl OutputOnlyWriter {
    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl Output for OutputOnlyWriter {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, input: &[u8], index: usize, count: usize) -> io::Result<usize> {
        let end = index + count;
        self.bytes.extend_from_slice(&input[index..end]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct NeedOutputCodec;

impl CharsetCodec for NeedOutputCodec {
    fn charset(&self) -> Charset {
        Charset::ASCII
    }
}

impl Codec for NeedOutputCodec {
    type Value = char;
    type Unit = u8;
    type DecodeError = CharsetDecodeError;
    type EncodeError = CharsetEncodeError;

    const MIN_UNITS_PER_VALUE: usize = 1;

    const MAX_ENCODE_UNITS_PER_VALUE: usize = 2;

    const MAX_DECODE_UNITS_PER_VALUE: usize = 2;

    fn encode_len(&self, value: &char) -> usize {
        if *value == 'B' { 2 } else { 1 }
    }

    unsafe fn decode(
        &mut self,
        _input: &[u8],
        _index: usize,
    ) -> Result<(char, NonZeroUsize), DecodeFailure<Self::DecodeError>> {
        unreachable!("writer tests do not decode with NeedOutputCodec")
    }

    unsafe fn encode(&mut self, value: &char, output: &mut [u8], index: usize) -> CharsetEncodeResult<usize> {
        let required = self.encode_len(value);
        debug_assert!(index.checked_add(required).is_some_and(|end| end <= output.len()));
        unsafe {
            // SAFETY: The caller guarantees that `required` units are writable
            // from `index`.
            *output.as_mut_ptr().add(index) = *value as u8;
            if required == 2 {
                *output.as_mut_ptr().add(index + 1) = *value as u8;
            }
        }
        Ok(self.encode_len(value))
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
fn test_write_utf8_text_to_byte_writer() -> std::io::Result<()> {
    let mut output = Vec::new();
    {
        let mut writer = CharsetTextWriter::new(&mut output, Utf8Codec, CharsetEncodePolicy::report())
            .with_line_ending(LineEnding::CrLf);

        writer.write_char('A')?;
        writer.write_chars(&['B', 'C'])?;
        writer.write_line("中文")?;
        writer.flush()?;
    }

    assert_eq!("ABC中文\r\n".as_bytes(), output.as_slice());
    Ok(())
}

#[test]
fn test_new_accepts_qubit_output_without_std_write() -> std::io::Result<()> {
    let output = OutputOnlyWriter::default();
    let mut writer = CharsetTextWriter::new(output, Utf8Codec, CharsetEncodePolicy::report());

    writer.write_str("output中文")?;
    writer.finish()?;
    let (output, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!("output中文".as_bytes(), output.into_bytes().as_slice());
    Ok(())
}

#[test]
fn test_write_rejects_unencodable_text_in_strict_mode() {
    let mut output = Vec::new();
    let mut writer = CharsetTextWriter::new(&mut output, AsciiCodec, CharsetEncodePolicy::report());

    let error = writer
        .write_str("🙂")
        .expect_err("strict mode must reject unencodable text");
    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_one_shot_write_error_preserves_delivered_prefix() {
    let mut output = OneShotWriteFailure::default();
    let error = output
        .write_str_with_charset("abc", Utf8Codec, CharsetEncodePolicy::report())
        .expect_err("output fails after a prefix");

    assert_eq!(ErrorKind::BrokenPipe, error.kind());
    assert_eq!(b"a", output.bytes.as_slice());
    assert!(output.calls >= 2);
}

fn assert_charset_main_index(error: &io::Error, expected: usize) {
    assert_eq!(ErrorKind::InvalidInput, error.kind());
    let encoded = error
        .get_ref()
        .and_then(|source| source.downcast_ref::<TranscodeEncodeError<CharsetEncodeError, char>>())
        .expect("preserve the transcode error type");
    let domain = encoded.domain_error_ref().expect("main charset failure");
    assert_eq!(Some(expected), domain.input_index());
    let source = encoded.domain_ref().expect("charset source");
    assert_eq!(expected, source.index());
    assert_eq!(Some('中' as u32), source.value());
}

#[test]
fn test_write_str_reports_call_relative_character_index() {
    for expected in [0, 255, 256, 300, 512] {
        let mut writer = CharsetTextWriter::new(Vec::<u8>::new(), AsciiCodec, CharsetEncodePolicy::report());
        let text = format!("{}中", "A".repeat(expected));
        let error = writer.write_str(&text).expect_err("ASCII rejects 中");
        assert_charset_main_index(&error, expected);
    }
}

#[test]
fn test_write_str_reports_character_index_after_multibyte_prefix() {
    let mut writer = CharsetTextWriter::new(Vec::<u8>::new(), Latin1Codec, CharsetEncodePolicy::report());
    let text = format!("{}中", "é".repeat(300));
    let error = writer.write_str(&text).expect_err("Latin-1 rejects 中");
    assert_charset_main_index(&error, 300);
}

#[test]
fn test_write_str_reports_index_relative_to_each_call() {
    let mut writer = CharsetTextWriter::new(Vec::<u8>::new(), AsciiCodec, CharsetEncodePolicy::report());
    writer.write_str("prefix").expect("prefix is ASCII");
    let text = format!("{}中", "A".repeat(300));
    let error = writer.write_str(&text).expect_err("ASCII rejects 中");
    assert_charset_main_index(&error, 300);
}

#[test]
fn test_write_line_reports_body_character_index() {
    let mut writer = CharsetTextWriter::new(Vec::<u8>::new(), AsciiCodec, CharsetEncodePolicy::report());
    let line = format!("{}中", "A".repeat(300));
    let error = writer.write_line(&line).expect_err("ASCII rejects 中");
    assert_charset_main_index(&error, 300);
}

#[test]
fn test_write_chars_rejects_unencodable_text_in_strict_mode() {
    let mut output = Vec::new();
    let mut writer = CharsetTextWriter::new(&mut output, AsciiCodec, CharsetEncodePolicy::report());

    let error = writer
        .write_chars(&['🙂'])
        .expect_err("strict mode must reject unencodable chars");
    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_write_chars_reports_original_slice_indices_across_chunks() {
    for expected in [255, 256, 512] {
        let mut writer = CharsetTextWriter::new(Vec::<u8>::new(), AsciiCodec, CharsetEncodePolicy::report());
        let mut chars = vec!['A'; expected];
        chars.push('中');
        let error = writer.write_chars(&chars).expect_err("ASCII rejects 中");
        assert_charset_main_index(&error, expected);
    }

    let mut writer = CharsetTextWriter::new(Vec::<u8>::new(), AsciiCodec, CharsetEncodePolicy::report());
    writer.write_chars(&['A'; 300]).expect("ASCII prefix is encodable");
    let error = writer.write_chars(&['中']).expect_err("the second call rejects 中");
    assert_charset_main_index(&error, 0);
}

#[test]
fn test_write_chars_output_failure_preserves_an_ordered_prefix() {
    let mut output = OneShotWriteFailure::default();
    let mut writer =
        CharsetTextWriter::new_with_buffer_capacity(&mut output, Utf8Codec, CharsetEncodePolicy::report(), 8);
    let text: String = (0..1024).map(|index| char::from(b'A' + (index % 26) as u8)).collect();
    let chars: Vec<char> = text.chars().collect();
    assert_eq!(ErrorKind::BrokenPipe, writer.write_chars(&chars).unwrap_err().kind());
    let (output, recovery) = writer.into_parts();
    let qubit_io::BufferedOutputRecovery::Pending(buffer) = recovery else {
        panic!("ordinary writes must have determinate recovery");
    };
    let mut delivered_and_pending = output.bytes.clone();
    delivered_and_pending.extend_from_slice(buffer.readable());
    assert!(text.as_bytes().starts_with(&delivered_and_pending));
}

#[test]
fn test_write_replaces_unencodable_text_in_replace_mode() -> std::io::Result<()> {
    let mut output = Vec::new();
    {
        let mut writer = CharsetTextWriter::new(&mut output, AsciiCodec, CharsetEncodePolicy::replace('?'));

        writer.write_str("🙂")?;
        writer.flush()?;
    }

    assert_eq!(b"?", output.as_slice());
    Ok(())
}

#[test]
fn test_accessors_and_into_output() -> std::io::Result<()> {
    let output = b"prefix:inner:".to_vec();
    let mut writer = CharsetTextWriter::new(output, AsciiCodec, CharsetEncodePolicy::report());

    assert_eq!(b"prefix:inner:", writer.output().as_slice());
    assert_eq!(LineEnding::Lf, writer.line_ending());
    writer.write_str("ascii")?;
    writer.flush()?;

    writer.finish()?;
    let (output, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };
    assert!(pending.readable().is_empty());
    assert_eq!(b"prefix:inner:ascii", output.as_slice());
    Ok(())
}

#[test]
fn test_finish_then_into_parts_returns_output() -> std::io::Result<()> {
    let mut writer = CharsetTextWriter::new(Vec::new(), Utf8Codec, CharsetEncodePolicy::report());
    writer.write_str("recoverable")?;

    writer.finish()?;
    let (output, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!(b"recoverable", output.as_slice());
    Ok(())
}

#[test]
fn test_finish_error_leaves_charset_writer_available_for_retry() {
    let mut writer = CharsetTextWriter::new(FailingWriter, Utf8Codec, CharsetEncodePolicy::report());

    let error = writer.finish().expect_err("finish should report write failure");

    assert_eq!(ErrorKind::Other, error.kind());
    let _ = writer.output();
}

#[test]
fn test_write_methods_buffer_with_sufficient_capacity() {
    let mut writer =
        CharsetTextWriter::new_with_buffer_capacity(FailingWriter, AsciiCodec, CharsetEncodePolicy::report(), 64);
    writer.write_char('x').unwrap();
    writer.write_chars(&['x']).unwrap();
    writer.write_line("x").unwrap();
    assert_eq!(ErrorKind::Other, writer.flush().unwrap_err().kind());
    let (_, recovery) = writer.into_parts();
    match recovery {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => assert_eq!(b"xxx\n", buffer.readable()),
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("no inner write panicked"),
    }
}

#[test]
fn test_capacity_pressure_reports_output_error_and_keeps_pending_bytes() {
    let mut writer =
        CharsetTextWriter::new_with_buffer_capacity(FailingWriter, AsciiCodec, CharsetEncodePolicy::report(), 1);
    writer.write_char('x').unwrap();
    assert_eq!(ErrorKind::Other, writer.write_chars(&['y']).unwrap_err().kind());
    let (_, recovery) = writer.into_parts();
    match recovery {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => assert_eq!(b"x", buffer.readable()),
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("no inner write panicked"),
    }
}

#[test]
fn test_capacity_pressure_from_single_char_keeps_pending_bytes() {
    let mut writer =
        CharsetTextWriter::new_with_buffer_capacity(FailingWriter, AsciiCodec, CharsetEncodePolicy::report(), 1);
    writer.write_char('x').unwrap();
    assert_eq!(ErrorKind::Other, writer.write_char('y').unwrap_err().kind());
    let (_, recovery) = writer.into_parts();
    match recovery {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => assert_eq!(b"x", buffer.readable()),
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("no inner write panicked"),
    }
}

#[test]
fn test_capacity_pressure_from_line_keeps_pending_bytes() {
    let mut writer =
        CharsetTextWriter::new_with_buffer_capacity(FailingWriter, AsciiCodec, CharsetEncodePolicy::report(), 1);
    writer.write_char('x').unwrap();
    assert_eq!(ErrorKind::Other, writer.write_line("y").unwrap_err().kind());
    let (_, recovery) = writer.into_parts();
    match recovery {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => assert_eq!(b"x", buffer.readable()),
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("no inner write panicked"),
    }
}

#[test]
fn test_write_raises_buffer_to_single_character_max_output() -> std::io::Result<()> {
    let mut writer =
        CharsetTextWriter::new_with_buffer_capacity(Vec::new(), NeedOutputCodec, CharsetEncodePolicy::report(), 1);

    writer.write_char('B')?;
    writer.finish()?;
    let (output, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!(b"BB", output.as_slice());
    Ok(())
}

#[test]
fn test_with_capacity_buffers_until_flush() -> std::io::Result<()> {
    let mut output = Vec::new();
    {
        let mut writer =
            CharsetTextWriter::new_with_buffer_capacity(&mut output, Utf8Codec, CharsetEncodePolicy::report(), 64);

        writer.write_str("buffered")?;
        assert!(writer.output().is_empty());
        writer.flush()?;
    }

    assert_eq!(b"buffered", output.as_slice());
    Ok(())
}

#[test]
fn test_charset_write_ext_creates_stream_writer() -> std::io::Result<()> {
    let output = Vec::new();
    let mut writer = output.charset_text_writer(AsciiCodec, CharsetEncodePolicy::replace('?'));

    writer.write_line("A🙂")?;
    writer.finish()?;
    let (output, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!(b"A?\n", output.as_slice());
    Ok(())
}

#[test]
fn test_charset_text_writer_try_new_reports_unencodable_replacement() {
    let result = CharsetTextWriter::try_new(Vec::new(), AsciiCodec, CharsetEncodePolicy::replace('中'));
    assert!(result.is_err());
}

#[test]
fn test_charset_write_ext_try_writer_reports_unencodable_replacement() {
    let result = Vec::new().try_charset_text_writer(AsciiCodec, CharsetEncodePolicy::replace('中'));
    assert!(result.is_err());
}

#[test]
#[should_panic(expected = "charset encode policy replacement must be encodable")]
fn test_charset_text_writer_new_panics_for_unencodable_replacement() {
    let _ = CharsetTextWriter::new(Vec::<u8>::new(), AsciiCodec, CharsetEncodePolicy::replace('中'));
}

#[test]
#[should_panic(expected = "charset encode policy replacement must be encodable")]
fn test_charset_text_writer_new_with_buffer_capacity_panics_for_unencodable_replacement() {
    let _ = CharsetTextWriter::new_with_buffer_capacity(
        Vec::<u8>::new(),
        AsciiCodec,
        CharsetEncodePolicy::replace('中'),
        8,
    );
}

#[test]
#[should_panic(expected = "charset encode policy replacement must be encodable")]
fn test_charset_write_ext_writer_panics_for_unencodable_replacement() {
    let _ = Vec::<u8>::new().charset_text_writer(AsciiCodec, CharsetEncodePolicy::replace('中'));
}

#[test]
#[should_panic(expected = "charset encode policy replacement must be encodable")]
fn test_charset_write_ext_buffered_writer_panics_for_unencodable_replacement() {
    let _ = Vec::<u8>::new().buffered_charset_text_writer(AsciiCodec, CharsetEncodePolicy::replace('中'), 8);
}

#[test]
fn test_charset_write_ext_accepts_qubit_output_without_std_write() -> std::io::Result<()> {
    let output = OutputOnlyWriter::default();
    let mut writer = output.charset_text_writer(Utf8Codec, CharsetEncodePolicy::report());

    writer.write_str("ext输出")?;
    writer.finish()?;
    let (output, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!("ext输出".as_bytes(), output.into_bytes().as_slice());
    Ok(())
}

#[test]
fn test_charset_write_ext_creates_buffered_stream_writer() -> std::io::Result<()> {
    let output = Vec::new();
    let mut writer = output.buffered_charset_text_writer(Utf8Codec, CharsetEncodePolicy::report(), 1);

    writer.write_str("é")?;
    writer.finish()?;
    let (output, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!("é".as_bytes(), output.as_slice());
    Ok(())
}

#[test]
fn test_charset_write_ext_try_creates_buffered_stream_writer() -> std::io::Result<()> {
    let output = Vec::new();
    let mut writer = output
        .try_buffered_charset_text_writer(Utf8Codec, CharsetEncodePolicy::report(), 1)
        .expect("UTF-8 writer configuration should be valid");

    writer.write_str("buffered")?;
    writer.finish()?;
    let (output, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!(b"buffered", output.as_slice());
    Ok(())
}

#[test]
fn test_charset_write_ext_writes_one_shot_to_qubit_output() -> std::io::Result<()> {
    let mut output = OutputOnlyWriter::default();

    output.write_str_with_charset("one-shot输出", Utf8Codec, CharsetEncodePolicy::report())?;

    assert_eq!("one-shot输出".as_bytes(), output.into_bytes().as_slice());
    Ok(())
}

#[test]
fn test_charset_write_ext_writes_one_shot_text() -> std::io::Result<()> {
    let mut output = Vec::new();

    output.write_str_with_charset("A🙂", AsciiCodec, CharsetEncodePolicy::replace('?'))?;

    assert_eq!(b"A?", output.as_slice());
    Ok(())
}

#[test]
fn test_charset_write_ext_one_shot_reports_unencodable_replacement() -> std::io::Result<()> {
    let mut output = Vec::new();
    let result = output.write_str_with_charset("A", AsciiCodec, CharsetEncodePolicy::replace('中'));
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
    Ok(())
}

#[test]
fn test_into_parts_preserves_indeterminate_after_inner_write_panics() {
    let mut writer = CharsetTextWriter::new(PanicOutput::default(), Utf8Codec, CharsetEncodePolicy::report());
    writer.write_str("text").expect("the encoded value should be buffered");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| TextWriter::flush(&mut writer)));
    assert!(result.is_err());

    let (inner, recovery) = writer.into_parts();
    assert_eq!(b"t", inner.bytes.as_slice());
    assert!(matches!(recovery, qubit_io::BufferedOutputRecovery::Indeterminate(_)));
}
