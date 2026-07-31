// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::{
    io::{
        self,
        ErrorKind,
        Write,
    },
    num::NonZeroUsize,
};

use qubit_codec::Codec;
use qubit_codec_text::{
    AsciiCodec,
    CharsetCodec,
    Utf8Codec,
};
use qubit_codec_text::{
    Charset,
    CharsetDecodeError,
    CharsetEncodeError,
    CharsetEncodeResult,
};
use qubit_io::Output;
use qubit_io_text::{
    CharsetTextWriter,
    CharsetWriteExt,
    CodingErrorPolicy,
    LineEnding,
    TextWrite,
};

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("write failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("flush failed"))
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

    unsafe fn write_unchecked(
        &mut self,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
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
    ) -> Result<
        (char, NonZeroUsize),
        qubit_codec::DecodeFailure<Self::DecodeError>,
    > {
        unreachable!("writer tests do not decode with NeedOutputCodec")
    }

    unsafe fn encode(
        &mut self,
        value: &char,
        output: &mut [u8],
        index: usize,
    ) -> CharsetEncodeResult<usize> {
        let required = self.encode_len(value);
        debug_assert!(
            index
                .checked_add(required)
                .is_some_and(|end| end <= output.len())
        );
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

#[test]
fn test_write_utf8_text_to_byte_writer() -> std::io::Result<()> {
    let mut output = Vec::new();
    {
        let mut writer = CharsetTextWriter::new(
            &mut output,
            Utf8Codec,
            CodingErrorPolicy::Strict,
        )
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
    let mut writer =
        CharsetTextWriter::new(output, Utf8Codec, CodingErrorPolicy::Strict);

    writer.write_str("output中文")?;
    writer.finish()?;
    let (output, pending) = writer.into_parts();

    assert!(pending.is_empty());
    assert_eq!("output中文".as_bytes(), output.into_bytes().as_slice());
    Ok(())
}

#[test]
fn test_write_rejects_unencodable_text_in_strict_mode() {
    let mut output = Vec::new();
    let mut writer = CharsetTextWriter::new(
        &mut output,
        AsciiCodec,
        CodingErrorPolicy::Strict,
    );

    let error = writer
        .write_str("🙂")
        .expect_err("strict mode must reject unencodable text");
    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_write_chars_rejects_unencodable_text_in_strict_mode() {
    let mut output = Vec::new();
    let mut writer = CharsetTextWriter::new(
        &mut output,
        AsciiCodec,
        CodingErrorPolicy::Strict,
    );

    let error = writer
        .write_chars(&['🙂'])
        .expect_err("strict mode must reject unencodable chars");
    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_write_replaces_unencodable_text_in_replace_mode() -> std::io::Result<()>
{
    let mut output = Vec::new();
    {
        let mut writer = CharsetTextWriter::new(
            &mut output,
            AsciiCodec,
            CodingErrorPolicy::Replace,
        );

        writer.write_str("🙂")?;
        writer.flush()?;
    }

    assert_eq!(b"?", output.as_slice());
    Ok(())
}

#[test]
fn test_accessors_and_into_output() -> std::io::Result<()> {
    let output = b"prefix:inner:".to_vec();
    let mut writer =
        CharsetTextWriter::new(output, AsciiCodec, CodingErrorPolicy::Strict);

    assert_eq!(b"prefix:inner:", writer.output().as_slice());
    assert_eq!(LineEnding::Lf, writer.line_ending());
    writer.write_str("ascii")?;
    writer.flush()?;

    writer.finish()?;
    let (output, pending) = writer.into_parts();
    assert!(pending.is_empty());
    assert_eq!(b"prefix:inner:ascii", output.as_slice());
    Ok(())
}

#[test]
fn test_finish_then_into_parts_returns_output() -> std::io::Result<()> {
    let mut writer = CharsetTextWriter::new(
        Vec::new(),
        Utf8Codec,
        CodingErrorPolicy::Strict,
    );
    writer.write_str("recoverable")?;

    writer.finish()?;
    let (output, pending) = writer.into_parts();

    assert!(pending.is_empty());
    assert_eq!(b"recoverable", output.as_slice());
    Ok(())
}

#[test]
fn test_finish_error_leaves_charset_writer_available_for_retry() {
    let mut writer = CharsetTextWriter::new(
        FailingWriter,
        Utf8Codec,
        CodingErrorPolicy::Strict,
    );

    let error = writer
        .finish()
        .expect_err("finish should report write failure");

    assert_eq!(ErrorKind::Other, error.kind());
    let _ = writer.output();
}

#[test]
fn test_write_methods_defer_underlying_errors_until_flush() {
    let mut writer = CharsetTextWriter::new_with_buffer_capacity(
        FailingWriter,
        AsciiCodec,
        CodingErrorPolicy::Strict,
        1,
    );

    writer
        .write_char('x')
        .expect("first single-byte write should stay buffered");
    writer
        .write_chars(&['x'])
        .expect("write_chars should grow the persistent buffer");
    writer
        .write_line("x")
        .expect("write_line should keep pending bytes buffered");
    assert_eq!(
        ErrorKind::Other,
        writer.flush().expect_err("flush must fail").kind(),
    );
}

#[test]
fn test_write_raises_buffer_to_single_character_max_output()
-> std::io::Result<()> {
    let mut writer = CharsetTextWriter::new_with_buffer_capacity(
        Vec::new(),
        NeedOutputCodec,
        CodingErrorPolicy::Strict,
        1,
    );

    writer.write_char('B')?;
    writer.finish()?;
    let (output, pending) = writer.into_parts();

    assert!(pending.is_empty());
    assert_eq!(b"BB", output.as_slice());
    Ok(())
}

#[test]
fn test_with_capacity_buffers_until_flush() -> std::io::Result<()> {
    let mut output = Vec::new();
    {
        let mut writer = CharsetTextWriter::new_with_buffer_capacity(
            &mut output,
            Utf8Codec,
            CodingErrorPolicy::Strict,
            64,
        );

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
    let mut writer =
        output.charset_text_writer(AsciiCodec, CodingErrorPolicy::Replace);

    writer.write_line("A🙂")?;
    writer.finish()?;
    let (output, pending) = writer.into_parts();

    assert!(pending.is_empty());
    assert_eq!(b"A?\n", output.as_slice());
    Ok(())
}

#[test]
fn test_charset_write_ext_accepts_qubit_output_without_std_write()
-> std::io::Result<()> {
    let output = OutputOnlyWriter::default();
    let mut writer =
        output.charset_text_writer(Utf8Codec, CodingErrorPolicy::Strict);

    writer.write_str("ext输出")?;
    writer.finish()?;
    let (output, pending) = writer.into_parts();

    assert!(pending.is_empty());
    assert_eq!("ext输出".as_bytes(), output.into_bytes().as_slice());
    Ok(())
}

#[test]
fn test_charset_write_ext_creates_buffered_stream_writer() -> std::io::Result<()>
{
    let output = Vec::new();
    let mut writer = output.buffered_charset_text_writer(
        Utf8Codec,
        CodingErrorPolicy::Strict,
        1,
    );

    writer.write_str("é")?;
    writer.finish()?;
    let (output, pending) = writer.into_parts();

    assert!(pending.is_empty());
    assert_eq!("é".as_bytes(), output.as_slice());
    Ok(())
}

#[test]
fn test_charset_write_ext_writes_one_shot_to_qubit_output()
-> std::io::Result<()> {
    let mut output = OutputOnlyWriter::default();

    output.write_str_with_charset(
        "one-shot输出",
        Utf8Codec,
        CodingErrorPolicy::Strict,
    )?;

    assert_eq!("one-shot输出".as_bytes(), output.into_bytes().as_slice());
    Ok(())
}

#[test]
fn test_charset_write_ext_writes_one_shot_text() -> std::io::Result<()> {
    let mut output = Vec::new();

    output.write_str_with_charset(
        "A🙂",
        AsciiCodec,
        CodingErrorPolicy::Replace,
    )?;

    assert_eq!(b"A?", output.as_slice());
    Ok(())
}
