// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::{
    future::Future,
    io,
    num::NonZeroUsize,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use qubit_codec::{Codec, DecodeFailure};
use qubit_codec_text::{
    Charset, CharsetCodec, CharsetDecodeError, CharsetDecodeErrorKind, CharsetDecodePolicy,
    CharsetEncodeError, CharsetEncodeErrorKind, CharsetEncodePolicy, Utf8Codec,
};
use qubit_io::{AsyncInput, AsyncOutput};
use qubit_io_text::{AsyncCharsetTextReader, AsyncCharsetTextWriter, LineEnding};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScriptedCodecMode {
    Lifecycle,
    DecodeResetError,
    DecodeFinishError,
    EncodeResetError,
    EncodeValueError,
    EncodeFinishError,
}

#[derive(Clone, Copy, Debug)]
struct ScriptedCodec {
    mode: ScriptedCodecMode,
}

impl ScriptedCodec {
    const fn new(mode: ScriptedCodecMode) -> Self {
        Self { mode }
    }

    const fn decode_error() -> CharsetDecodeError {
        CharsetDecodeError::new(
            Charset::UTF_8,
            CharsetDecodeErrorKind::MalformedSequence { value: None },
            0,
        )
    }

    const fn encode_error() -> CharsetEncodeError {
        CharsetEncodeError::new(
            Charset::UTF_8,
            CharsetEncodeErrorKind::UnmappableCharacter { value: 0 },
            0,
        )
    }
}

impl CharsetCodec for ScriptedCodec {
    fn charset(&self) -> Charset {
        Charset::UTF_8
    }
}

impl Codec for ScriptedCodec {
    type Value = char;
    type Unit = u8;
    type DecodeError = CharsetDecodeError;
    type EncodeError = CharsetEncodeError;

    const MIN_UNITS_PER_VALUE: usize = 1;
    const MAX_ENCODE_UNITS_PER_VALUE: usize = 8;

    const MAX_DECODE_UNITS_PER_VALUE: usize = 8;
    const MAX_ENCODE_RESET_UNITS: usize = 16;
    const MAX_ENCODE_FINISH_UNITS: usize = 32;
    const MAX_DECODE_RESET_VALUES: usize = 16;
    const MAX_DECODE_FINISH_VALUES: usize = 32;

    fn encode_len(&self, _value: &char) -> usize {
        1
    }

    unsafe fn encode_reset(
        &mut self,
        output: &mut [u8],
        output_index: usize,
    ) -> Result<usize, Self::EncodeError> {
        if self.mode == ScriptedCodecMode::EncodeResetError {
            return Err(Self::encode_error());
        }
        output[output_index] = b'^';
        Ok(1)
    }

    unsafe fn encode(
        &mut self,
        value: &char,
        output: &mut [u8],
        output_index: usize,
    ) -> Result<usize, Self::EncodeError> {
        if self.mode == ScriptedCodecMode::EncodeValueError {
            return Err(Self::encode_error());
        }
        output[output_index] = *value as u8;
        Ok(1)
    }

    unsafe fn encode_finish(
        &mut self,
        output: &mut [u8],
        output_index: usize,
    ) -> Result<usize, Self::EncodeError> {
        if self.mode == ScriptedCodecMode::EncodeFinishError {
            return Err(Self::encode_error());
        }
        output[output_index] = b'!';
        Ok(1)
    }

    unsafe fn decode_reset(
        &mut self,
        output: &mut [char],
        output_index: usize,
    ) -> Result<usize, Self::DecodeError> {
        if self.mode == ScriptedCodecMode::DecodeResetError {
            return Err(Self::decode_error());
        }
        output[output_index] = '^';
        Ok(1)
    }

    unsafe fn decode(
        &mut self,
        input: &[u8],
        input_index: usize,
    ) -> Result<(char, NonZeroUsize), DecodeFailure<Self::DecodeError>> {
        if input[input_index] == 0xF0 {
            let available = input.len() - input_index;
            let required = NonZeroUsize::new(8).expect("eight is non-zero");
            if available < required.get() {
                return Err(DecodeFailure::incomplete(required));
            }
            return Ok(('#', required));
        }
        Ok((char::from(input[input_index]), NonZeroUsize::MIN))
    }

    unsafe fn decode_finish(
        &mut self,
        output: &mut [char],
        output_index: usize,
    ) -> Result<usize, Self::DecodeError> {
        if self.mode == ScriptedCodecMode::DecodeFinishError {
            return Err(Self::decode_error());
        }
        output[output_index] = '!';
        Ok(1)
    }
}

struct ChunkedAsyncInput {
    bytes: Vec<u8>,
    position: usize,
    max_chunk: usize,
    pending: bool,
    error: Option<io::ErrorKind>,
}

impl ChunkedAsyncInput {
    fn new(bytes: Vec<u8>, max_chunk: usize, pending: bool) -> Self {
        Self {
            bytes,
            position: 0,
            max_chunk,
            pending,
            error: None,
        }
    }

    fn with_error(mut self, error: io::ErrorKind) -> Self {
        self.error = Some(error);
        self
    }
}

impl AsyncInput for ChunkedAsyncInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        if let Some(kind) = self.error.take() {
            return Poll::Ready(Err(io::Error::new(kind, "scripted input failure")));
        }
        if self.pending {
            self.pending = false;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.pending = true;
        let available = self.bytes.len().saturating_sub(self.position);
        let read = available.min(count).min(self.max_chunk);
        output[index..index + read]
            .copy_from_slice(&self.bytes[self.position..self.position + read]);
        self.position += read;
        Poll::Ready(Ok(read))
    }
}

#[derive(Debug)]
struct ChunkedAsyncOutput {
    bytes: Vec<u8>,
    max_chunk: usize,
    pending: bool,
    flushed: bool,
    write_zero: bool,
    write_error: Option<io::ErrorKind>,
    flush_error: Option<io::ErrorKind>,
}

impl ChunkedAsyncOutput {
    fn new(max_chunk: usize, pending: bool) -> Self {
        Self {
            bytes: Vec::new(),
            max_chunk,
            pending,
            flushed: false,
            write_zero: false,
            write_error: None,
            flush_error: None,
        }
    }

    fn with_write_zero(mut self) -> Self {
        self.write_zero = true;
        self
    }

    fn with_write_error(mut self, error: io::ErrorKind) -> Self {
        self.write_error = Some(error);
        self
    }

    fn with_flush_error(mut self, error: io::ErrorKind) -> Self {
        self.flush_error = Some(error);
        self
    }
}

impl AsyncOutput for ChunkedAsyncOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        if let Some(kind) = self.write_error.take() {
            return Poll::Ready(Err(io::Error::new(kind, "scripted output failure")));
        }
        if self.write_zero {
            self.write_zero = false;
            return Poll::Ready(Ok(0));
        }
        if self.pending {
            self.pending = false;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.pending = true;
        let written = count.min(self.max_chunk);
        self.bytes.extend_from_slice(&input[index..index + written]);
        Poll::Ready(Ok(written))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if let Some(kind) = self.flush_error.take() {
            return Poll::Ready(Err(io::Error::new(kind, "scripted flush failure")));
        }
        self.flushed = true;
        Poll::Ready(Ok(()))
    }
}

fn test_context() -> Context<'static> {
    Context::from_waker(Waker::noop())
}

fn complete<F>(future: F) -> F::Output
where
    F: Future,
{
    let mut context = test_context();
    let mut future = std::pin::pin!(future);
    for _ in 0..512 {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
    }
    panic!("test future did not complete");
}

#[test]
fn async_charset_reader_decodes_across_pending_and_byte_boundaries() -> io::Result<()> {
    let input = ChunkedAsyncInput::new("中🙂\nnext".as_bytes().to_vec(), 1, true);
    let mut reader = AsyncCharsetTextReader::new_with_buffer_capacity(
        input,
        Utf8Codec,
        CharsetDecodePolicy::report(),
        1,
    );
    let mut line = String::new();

    assert!(complete(reader.read_line_async(&mut line))?);
    assert_eq!("中🙂\n", line);

    line.clear();
    assert!(complete(reader.read_line_async(&mut line))?);
    assert_eq!("next", line);
    assert!(!complete(reader.read_line_async(&mut line))?);
    Ok(())
}

#[test]
fn async_charset_reader_retains_partial_character_when_future_is_cancelled() -> io::Result<()> {
    let input = ChunkedAsyncInput::new("中".as_bytes().to_vec(), 1, false);
    let mut reader = AsyncCharsetTextReader::new_with_buffer_capacity(
        input,
        Utf8Codec,
        CharsetDecodePolicy::report(),
        1,
    );
    let mut context = test_context();

    let mut future = Box::pin(reader.read_char_async());
    assert!(future.as_mut().poll(&mut context).is_pending());
    drop(future);

    assert_eq!(Some('中'), complete(reader.read_char_async())?);
    assert_eq!(None, complete(reader.read_char_async())?);
    Ok(())
}

#[test]
fn async_charset_reader_applies_incomplete_eof_policy() -> io::Result<()> {
    let input = ChunkedAsyncInput::new(vec![0xE4, 0xB8], 1, true);
    let mut reader =
        AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::replace('!'));
    let mut text = String::new();

    assert_eq!(1, complete(reader.read_to_string_async(&mut text))?);
    assert_eq!("!", text);

    let input = ChunkedAsyncInput::new(vec![0xE4, 0xB8], 1, true);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let error = complete(reader.read_char_async())
        .expect_err("strict mode must reject an incomplete EOF tail");
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    Ok(())
}

#[test]
fn async_charset_reader_accessors_and_bulk_reads_cover_buffered_state() -> io::Result<()> {
    let input = ChunkedAsyncInput::new(b"ab".to_vec(), 2, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut chars = Vec::new();

    assert_eq!(0, reader.input().position);
    reader.input_mut().max_chunk = 2;
    assert_eq!(0, complete(reader.read_chars_async(&mut chars, 0))?);
    assert_eq!(0, reader.input().position);
    assert_eq!(1, complete(reader.read_chars_async(&mut chars, 1))?);
    assert_eq!(1, complete(reader.read_chars_async(&mut chars, 4))?);
    assert_eq!(['a', 'b'], chars.as_slice());
    assert_eq!(None, complete(reader.read_char_async())?);

    assert_eq!(2, reader.input().position);
    Ok(())
}

#[test]
fn async_charset_reader_into_parts_preserves_unreturned_characters() -> io::Result<()> {
    let input = ChunkedAsyncInput::new(b"abc".to_vec(), 3, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());

    assert_eq!(Some('a'), complete(reader.read_char_async())?);

    let (input, unread, _decoder, pending_chars) = reader.into_parts();
    assert_eq!(3, input.position);
    assert!(unread.readable().is_empty());
    assert_eq!(['b', 'c'], pending_chars.as_slice());
    Ok(())
}

#[test]
fn async_charset_reader_compacts_partial_tail_across_pending_reads() -> io::Result<()> {
    let input = ChunkedAsyncInput::new("A中".as_bytes().to_vec(), 3, true);
    let mut reader = AsyncCharsetTextReader::new_with_buffer_capacity(
        input,
        Utf8Codec,
        CharsetDecodePolicy::report(),
        1,
    );
    let mut output = String::new();

    assert_eq!(2, complete(reader.read_to_string_async(&mut output))?);
    assert_eq!("A中", output);
    Ok(())
}

#[test]
fn async_charset_reader_propagates_input_and_decode_errors() {
    let input = ChunkedAsyncInput::new(Vec::new(), 1, false).with_error(io::ErrorKind::BrokenPipe);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let error =
        complete(reader.read_char_async()).expect_err("scripted input failure should propagate");
    assert_eq!(io::ErrorKind::BrokenPipe, error.kind());

    let input = ChunkedAsyncInput::new(vec![0xFF], 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let error = complete(reader.read_char_async()).expect_err("malformed UTF-8 should propagate");
    assert_eq!(io::ErrorKind::InvalidData, error.kind());

    let input =
        ChunkedAsyncInput::new(Vec::new(), 1, false).with_error(io::ErrorKind::ConnectionReset);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut chars = Vec::new();
    let error = complete(reader.read_chars_async(&mut chars, 1))
        .expect_err("bulk read failure should propagate");
    assert_eq!(io::ErrorKind::ConnectionReset, error.kind());

    let input =
        ChunkedAsyncInput::new(Vec::new(), 1, false).with_error(io::ErrorKind::ConnectionAborted);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut text = String::new();
    let error = complete(reader.read_to_string_async(&mut text))
        .expect_err("string read failure should propagate");
    assert_eq!(io::ErrorKind::ConnectionAborted, error.kind());

    let input = ChunkedAsyncInput::new(Vec::new(), 1, false).with_error(io::ErrorKind::TimedOut);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let error = complete(reader.read_line_async(&mut text))
        .expect_err("line read failure should propagate");
    assert_eq!(io::ErrorKind::TimedOut, error.kind());
}

#[test]
fn async_charset_reader_grows_lifecycle_and_incomplete_buffers() -> io::Result<()> {
    let mut encoded = vec![0xF0];
    encoded.extend_from_slice(&[0; 7]);
    let input = ChunkedAsyncInput::new(encoded, 4, false);
    let mut reader = AsyncCharsetTextReader::new_with_buffer_capacity(
        input,
        ScriptedCodec::new(ScriptedCodecMode::Lifecycle),
        CharsetDecodePolicy::report(),
        1,
    );
    let mut text = String::new();

    assert_eq!(3, complete(reader.read_to_string_async(&mut text))?);
    assert_eq!("^#!", text);
    assert_eq!(None, complete(reader.read_char_async())?);
    Ok(())
}

#[test]
fn async_charset_reader_propagates_lifecycle_errors() -> io::Result<()> {
    let input = ChunkedAsyncInput::new(Vec::new(), 4, false);
    let mut reader = AsyncCharsetTextReader::new(
        input,
        ScriptedCodec::new(ScriptedCodecMode::DecodeResetError),
        CharsetDecodePolicy::report(),
    );
    let error =
        complete(reader.read_char_async()).expect_err("decoder reset error should propagate");
    assert_eq!(io::ErrorKind::InvalidData, error.kind());

    let input = ChunkedAsyncInput::new(Vec::new(), 4, false);
    let mut reader = AsyncCharsetTextReader::new(
        input,
        ScriptedCodec::new(ScriptedCodecMode::DecodeFinishError),
        CharsetDecodePolicy::report(),
    );
    assert_eq!(Some('^'), complete(reader.read_char_async())?);
    let error =
        complete(reader.read_char_async()).expect_err("decoder finish error should propagate");
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    Ok(())
}

#[test]
fn async_charset_writer_encodes_and_flushes_async_only_output() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(2, true);
    let mut writer = AsyncCharsetTextWriter::new_with_buffer_capacity(
        output,
        Utf8Codec,
        CharsetEncodePolicy::report(),
        3,
    )
    .with_line_ending(LineEnding::CrLf);

    complete(writer.write_char_async('A'))?;
    complete(writer.write_chars_fully_async(&['中', '🙂']))?;
    complete(writer.write_line_fully_async("tail"))?;
    complete(writer.finish_async())?;
    let (output, pending) = writer.into_parts();

    assert!(pending.is_empty());
    assert_eq!("A中🙂tail\r\n".as_bytes(), output.bytes.as_slice());
    assert!(output.flushed);
    Ok(())
}

#[test]
fn async_charset_writer_commits_source_before_later_delivery_can_pend() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(2, false);
    let mut writer = AsyncCharsetTextWriter::new(output, Utf8Codec, CharsetEncodePolicy::report());
    let mut context = test_context();

    let mut future = Box::pin(writer.write_str_async("cancel-safe"));
    assert!(future.as_mut().poll(&mut context).is_ready());
    drop(future);

    complete(writer.finish_async())?;
    let (output, pending) = writer.into_parts();
    assert!(pending.is_empty());
    assert_eq!(b"cancel-safe", output.bytes.as_slice());
    Ok(())
}

#[test]
fn async_charset_writer_accessors_empty_chunks_and_finished_state() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(4, false);
    let mut writer = AsyncCharsetTextWriter::new(output, Utf8Codec, CharsetEncodePolicy::report())
        .with_line_ending(LineEnding::Cr);

    assert_eq!(LineEnding::Cr, writer.configured_line_ending());
    assert!(writer.output().bytes.is_empty());
    writer.output_mut().max_chunk = 4;
    assert_eq!(0, complete(writer.write_chars_async(&[]))?);
    assert_eq!(0, complete(writer.write_str_async(""))?);
    complete(writer.write_str_fully_async(&"a".repeat(256)))?;
    complete(writer.flush_async())?;
    assert!(writer.output().flushed);
    complete(writer.finish_async())?;
    complete(writer.finish_async())?;

    let error = complete(writer.write_char_async('x'))
        .expect_err("character write after finish should fail");
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
    let error = complete(writer.write_chars_async(&['x']))
        .expect_err("character slice write after finish should fail");
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
    let error =
        complete(writer.write_str_async("x")).expect_err("string write after finish should fail");
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
    Ok(())
}

#[test]
fn async_charset_writer_grows_across_need_output_and_pending_writes() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(1, true);
    let mut writer = AsyncCharsetTextWriter::new_with_buffer_capacity(
        output,
        Utf8Codec,
        CharsetEncodePolicy::report(),
        1,
    );

    complete(writer.write_chars_fully_async(&['a', 'b', 'c', 'd', 'e']))?;
    complete(writer.finish_async())?;
    let (output, pending) = writer.into_parts();
    assert!(pending.is_empty());
    assert_eq!(b"abcde", output.bytes.as_slice());
    Ok(())
}

#[test]
fn async_charset_writer_retains_bytes_after_zero_and_write_errors() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(4, false).with_write_zero();
    let mut writer = AsyncCharsetTextWriter::new(output, Utf8Codec, CharsetEncodePolicy::report());
    complete(writer.write_char_async('A'))?;
    let error = complete(writer.flush_async()).expect_err("zero-length write should fail");
    assert_eq!(io::ErrorKind::WriteZero, error.kind());
    complete(writer.flush_async())?;
    assert_eq!(b"A", writer.output().bytes.as_slice());

    let output = ChunkedAsyncOutput::new(4, false).with_write_error(io::ErrorKind::BrokenPipe);
    let mut writer = AsyncCharsetTextWriter::new(output, Utf8Codec, CharsetEncodePolicy::report());
    complete(writer.write_char_async('B'))?;
    let error = complete(writer.flush_async()).expect_err("scripted write error should fail");
    assert_eq!(io::ErrorKind::BrokenPipe, error.kind());
    complete(writer.flush_async())?;
    assert_eq!(b"B", writer.output().bytes.as_slice());
    Ok(())
}

#[test]
fn async_charset_writer_propagates_flush_errors() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(4, false).with_flush_error(io::ErrorKind::BrokenPipe);
    let mut writer = AsyncCharsetTextWriter::new(output, Utf8Codec, CharsetEncodePolicy::report());

    let error = complete(writer.flush_async()).expect_err("scripted flush error should propagate");
    assert_eq!(io::ErrorKind::BrokenPipe, error.kind());
    complete(writer.flush_async())?;
    Ok(())
}

#[test]
fn async_charset_writer_finish_error_leaves_writer_available_for_retry() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(4, false).with_flush_error(io::ErrorKind::BrokenPipe);
    let mut writer = AsyncCharsetTextWriter::new(output, Utf8Codec, CharsetEncodePolicy::report());

    let error =
        complete(writer.finish_async()).expect_err("finish should report the scripted flush error");
    assert_eq!(io::ErrorKind::BrokenPipe, error.kind());
    complete(writer.finish_async())?;
    let (output, pending) = writer.into_parts();

    assert!(pending.is_empty());
    assert!(output.flushed);
    Ok(())
}

#[test]
fn async_charset_writer_grows_and_emits_codec_lifecycle_output() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(64, false);
    let mut writer = AsyncCharsetTextWriter::new_with_buffer_capacity(
        output,
        ScriptedCodec::new(ScriptedCodecMode::Lifecycle),
        CharsetEncodePolicy::report(),
        1,
    );

    complete(writer.write_char_async('A'))?;
    complete(writer.finish_async())?;
    let (output, pending) = writer.into_parts();
    assert!(pending.is_empty());
    assert_eq!(b"^A!", output.bytes.as_slice());
    Ok(())
}

#[test]
fn async_charset_writer_propagates_codec_lifecycle_errors() {
    let output = ChunkedAsyncOutput::new(64, false);
    let mut writer = AsyncCharsetTextWriter::new(
        output,
        ScriptedCodec::new(ScriptedCodecMode::EncodeResetError),
        CharsetEncodePolicy::report(),
    );
    let error =
        complete(writer.write_char_async('A')).expect_err("encoder reset error should propagate");
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());

    let output = ChunkedAsyncOutput::new(64, false);
    let mut writer = AsyncCharsetTextWriter::new(
        output,
        ScriptedCodec::new(ScriptedCodecMode::EncodeValueError),
        CharsetEncodePolicy::report(),
    );
    let error =
        complete(writer.write_char_async('A')).expect_err("encoder value error should propagate");
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
    assert_eq!(b"^", writer.output().bytes.as_slice());

    let output = ChunkedAsyncOutput::new(64, false);
    let mut writer = AsyncCharsetTextWriter::new(
        output,
        ScriptedCodec::new(ScriptedCodecMode::EncodeFinishError),
        CharsetEncodePolicy::report(),
    );
    let error = complete(writer.finish_async()).expect_err("encoder finish error should propagate");
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());

    let output = ChunkedAsyncOutput::new(64, false);
    let mut writer = AsyncCharsetTextWriter::new(
        output,
        ScriptedCodec::new(ScriptedCodecMode::EncodeResetError),
        CharsetEncodePolicy::report(),
    );
    let error = complete(writer.finish_async()).expect_err("finish error should propagate");
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
}

#[test]
fn async_charset_reader_limited_read_rolls_back_appended_text() {
    let input = ChunkedAsyncInput::new("A中".as_bytes().to_vec(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut output = String::from("prefix:");

    let error = complete(reader.read_to_string_limited_async(&mut output, 3))
        .expect_err("decoded text beyond the append limit must fail");

    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert_eq!("prefix:", output);
}
