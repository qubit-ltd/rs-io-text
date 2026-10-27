// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::Cell;
use std::future::Future;
use std::io;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::rc::Rc;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_codec::ByteOrder;
use qubit_codec::Codec;
use qubit_codec::DecodeFailure;
use qubit_codec_text::AsciiCodec;
use qubit_codec_text::Charset;
use qubit_codec_text::CharsetCodec;
use qubit_codec_text::CharsetDecodeError;
use qubit_codec_text::CharsetDecodeErrorKind;
use qubit_codec_text::CharsetDecodePolicy;
use qubit_codec_text::CharsetEncodeError;
use qubit_codec_text::CharsetEncodeErrorKind;
use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::Utf8Codec;
use qubit_codec_text::Utf16ByteCodec;
use qubit_codec_text::Utf32ByteCodec;
use qubit_io::AsyncInput;
use qubit_io::AsyncOutput;
use qubit_io_text::AsyncCharsetTextReader;
use qubit_io_text::AsyncCharsetTextWriter;
use qubit_io_text::AsyncTextLineReader;
use qubit_io_text::AsyncTextReader;
use qubit_io_text::AsyncTextWriter;
use qubit_io_text::AsyncUtf8TextReader;
use qubit_io_text::AsyncUtf8TextWriter;
use qubit_io_text::LineEnding;
use qubit_io_text::LineEndingSet;

fn assert_async_reader<T>()
where
    T: AsyncTextReader + AsyncTextLineReader,
{
}

#[test]
fn test_async_charset_writer_try_new_reports_unencodable_replacement() {
    let result = AsyncCharsetTextWriter::try_new(
        ChunkedAsyncOutput::new(1, false),
        AsciiCodec,
        CharsetEncodePolicy::replace('中'),
    );
    assert!(result.is_err());
}

fn assert_async_writer<T>()
where
    T: AsyncTextWriter,
{
}

#[test]
fn test_async_text_traits_cover_charset_and_utf8_adapters() {
    assert_async_reader::<AsyncCharsetTextReader<ChunkedAsyncInput, Utf8Codec>>();
    assert_async_reader::<AsyncUtf8TextReader<ChunkedAsyncInput>>();
    assert_async_writer::<AsyncCharsetTextWriter<ChunkedAsyncOutput, Utf8Codec>>();
    assert_async_writer::<AsyncUtf8TextWriter<ChunkedAsyncOutput>>();
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScriptedCodecMode {
    Lifecycle,
    DecodeResetError,
    DecodeFinishError,
    DecodeFinishTail,
    DecodeFinishLines,
    EncodeResetError,
    EncodeValueError,
    EncodeFinishError,
}

#[derive(Clone, Debug)]
struct ScriptedCodec {
    mode: ScriptedCodecMode,
    encode_finish_calls: Option<Rc<Cell<usize>>>,
}

impl ScriptedCodec {
    const fn new(mode: ScriptedCodecMode) -> Self {
        Self {
            mode,
            encode_finish_calls: None,
        }
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

    unsafe fn encode_reset(&mut self, output: &mut [u8], output_index: usize) -> Result<usize, Self::EncodeError> {
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

    unsafe fn encode_finish(&mut self, output: &mut [u8], output_index: usize) -> Result<usize, Self::EncodeError> {
        if self.mode == ScriptedCodecMode::EncodeFinishError {
            return Err(Self::encode_error());
        }
        if let Some(calls) = &self.encode_finish_calls {
            calls.set(calls.get() + 1);
        }
        output[output_index] = b'!';
        Ok(1)
    }

    unsafe fn decode_reset(&mut self, output: &mut [char], output_index: usize) -> Result<usize, Self::DecodeError> {
        if self.mode == ScriptedCodecMode::DecodeResetError {
            return Err(Self::decode_error());
        }
        if matches!(
            self.mode,
            ScriptedCodecMode::DecodeFinishTail | ScriptedCodecMode::DecodeFinishLines
        ) {
            return Ok(0);
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

    unsafe fn decode_finish(&mut self, output: &mut [char], output_index: usize) -> Result<usize, Self::DecodeError> {
        if self.mode == ScriptedCodecMode::DecodeFinishError {
            return Err(Self::decode_error());
        }
        if self.mode == ScriptedCodecMode::DecodeFinishTail {
            output[output_index..output_index + 2].copy_from_slice(&['X', 'Y']);
            return Ok(2);
        }
        if self.mode == ScriptedCodecMode::DecodeFinishLines {
            output[output_index..output_index + 3].copy_from_slice(&['X', '\n', 'Y']);
            return Ok(3);
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
    error_at_position: Option<(usize, io::ErrorKind)>,
    progress: Option<Rc<Cell<usize>>>,
}

impl ChunkedAsyncInput {
    fn new(bytes: Vec<u8>, max_chunk: usize, pending: bool) -> Self {
        Self {
            bytes,
            position: 0,
            max_chunk,
            pending,
            error: None,
            error_at_position: None,
            progress: None,
        }
    }

    fn with_error(mut self, error: io::ErrorKind) -> Self {
        self.error = Some(error);
        self
    }

    fn with_progress(mut self, progress: Rc<Cell<usize>>) -> Self {
        self.progress = Some(progress);
        self
    }

    fn with_error_at_position(mut self, position: usize, error: io::ErrorKind) -> Self {
        self.error_at_position = Some((position, error));
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
        if self
            .error_at_position
            .is_some_and(|(position, _)| self.position == position)
        {
            let (_, kind) = self.error_at_position.take().expect("checked error position");
            return Poll::Ready(Err(io::Error::new(kind, "scripted positioned input failure")));
        }
        if self.pending {
            self.pending = false;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.pending = true;
        let available = self.bytes.len().saturating_sub(self.position);
        let read = available.min(count).min(self.max_chunk);
        output[index..index + read].copy_from_slice(&self.bytes[self.position..self.position + read]);
        self.position += read;
        if let Some(progress) = &self.progress {
            progress.set(self.position);
        }
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
    flush_pending: bool,
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
            flush_pending: false,
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

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if let Some(kind) = self.flush_error.take() {
            return Poll::Ready(Err(io::Error::new(kind, "scripted flush failure")));
        }
        if self.flush_pending {
            self.flush_pending = false;
            cx.waker().wake_by_ref();
            return Poll::Pending;
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
fn test_read_ahead_error_boundary_async_line() {
    for chunk in 1..=4 {
        for pending in [false, true] {
            let input = ChunkedAsyncInput::new(b"ok\n\xff".to_vec(), chunk, pending);
            let mut reader =
                AsyncCharsetTextReader::new_with_buffer_capacity(input, Utf8Codec, CharsetDecodePolicy::report(), 8);
            let mut line = String::new();
            let result = complete(reader.read_line_async(&mut line));
            match chunk {
                1 | 3 => {
                    assert!(result.unwrap());
                    assert_eq!(line, "ok\n");
                }
                2 => {
                    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
                    assert_eq!(line, "ok");
                }
                4 => {
                    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
                    assert!(line.is_empty());
                }
                _ => unreachable!(),
            }
        }
    }
}

#[test]
fn test_read_ahead_error_boundary_async_character_and_control() {
    for chunk in 1..=4 {
        for pending in [false, true] {
            let input = ChunkedAsyncInput::new(b"ok\n\xff".to_vec(), chunk, pending);
            let mut reader =
                AsyncCharsetTextReader::new_with_buffer_capacity(input, Utf8Codec, CharsetDecodePolicy::report(), 8);
            let result = complete(reader.read_char_async());
            if chunk == 4 {
                assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
            } else {
                assert_eq!(result.unwrap(), Some('o'));
            }

            let input = ChunkedAsyncInput::new(b"ok\n".to_vec(), chunk, pending);
            let mut reader =
                AsyncCharsetTextReader::new_with_buffer_capacity(input, Utf8Codec, CharsetDecodePolicy::report(), 8);
            let mut line = String::new();
            assert!(complete(reader.read_line_async(&mut line)).unwrap());
            assert_eq!(line, "ok\n");
            assert_eq!(complete(reader.read_char_async()).unwrap(), None);
        }
    }
}

#[derive(Default)]
struct CountingAsyncOutput {
    bytes: Vec<u8>,
    writes: usize,
}

impl AsyncOutput for CountingAsyncOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        self.writes += 1;
        self.bytes.extend_from_slice(&input[index..index + count]);
        Poll::Ready(Ok(count))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[test]
fn test_async_charset_writer_aggregates_small_writes() -> io::Result<()> {
    let mut writer =
        AsyncCharsetTextWriter::new(CountingAsyncOutput::default(), Utf8Codec, CharsetEncodePolicy::report());
    for _ in 0..1000 {
        complete(writer.write_char_async('x'))?;
    }
    assert_eq!(0, writer.output().writes);
    complete(writer.finish_async())?;
    assert_eq!(1, writer.output().writes);
    assert_eq!(vec![b'x'; 1000], writer.output().bytes);
    Ok(())
}

#[test]
fn test_async_charset_writer_small_capacity_write_cancellation_preserves_bytes() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(4, true);
    let mut writer =
        AsyncCharsetTextWriter::new_with_buffer_capacity(output, Utf8Codec, CharsetEncodePolicy::report(), 4);
    let text = "abcd";
    let mut offset = 0;
    while offset < text.len() {
        let consumed = complete(writer.write_str_async(&text[offset..]))?;
        assert!(consumed > 0);
        offset += consumed;
    }
    let mut context = test_context();
    {
        let mut future = Box::pin(writer.write_char_async('e'));
        assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));
    }
    complete(writer.write_char_async('e'))?;
    complete(writer.finish_async())?;
    assert_eq!(b"abcde", writer.output().bytes.as_slice());
    Ok(())
}

#[test]
fn test_async_charset_writer_small_capacity_error_does_not_consume_new_char() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(4, false).with_write_error(io::ErrorKind::BrokenPipe);
    let mut writer =
        AsyncCharsetTextWriter::new_with_buffer_capacity(output, Utf8Codec, CharsetEncodePolicy::report(), 4);
    assert_eq!(4, complete(writer.write_chars_async(&['a', 'b', 'c', 'd']))?);
    assert_eq!(
        io::ErrorKind::BrokenPipe,
        complete(writer.write_char_async('e')).unwrap_err().kind()
    );
    complete(writer.write_char_async('e'))?;
    complete(writer.finish_async())?;
    assert_eq!(b"abcde", writer.output().bytes.as_slice());
    Ok(())
}

#[test]
fn test_async_charset_writer_string_counts_utf8_prefixes() -> io::Result<()> {
    let mut writer =
        AsyncCharsetTextWriter::new(CountingAsyncOutput::default(), Utf8Codec, CharsetEncodePolicy::report());
    let text = "A中😀";
    let mut consumed = 0;
    while consumed < text.len() {
        let count = complete(writer.write_str_async(&text[consumed..]))?;
        assert!(text.is_char_boundary(consumed + count));
        consumed += count;
    }
    complete(writer.finish_async())?;
    assert_eq!(text.as_bytes(), writer.output().bytes.as_slice());
    Ok(())
}

#[test]
fn test_async_charset_writer_finish_error_retries_lifecycle_output_once() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(64, false).with_write_error(io::ErrorKind::BrokenPipe);
    let mut writer = AsyncCharsetTextWriter::new(
        output,
        ScriptedCodec::new(ScriptedCodecMode::Lifecycle),
        CharsetEncodePolicy::report(),
    );
    complete(writer.write_char_async('A'))?;
    assert_eq!(
        io::ErrorKind::BrokenPipe,
        complete(writer.finish_async()).unwrap_err().kind()
    );
    complete(writer.finish_async())?;
    let (output, pending) = writer.into_parts();
    assert!(pending.is_empty());
    assert_eq!(b"^A!", output.bytes.as_slice());
    Ok(())
}

#[test]
fn test_async_charset_writer_finish_cancellation_retries_without_repeating_lifecycle() -> io::Result<()> {
    for during_flush in [false, true] {
        let calls = Rc::new(Cell::new(0));
        let mut codec = ScriptedCodec::new(ScriptedCodecMode::Lifecycle);
        codec.encode_finish_calls = Some(calls.clone());
        let mut output = ChunkedAsyncOutput::new(if during_flush { 64 } else { 1 }, false);
        output.flush_pending = during_flush;
        let mut writer = AsyncCharsetTextWriter::new(output, codec, CharsetEncodePolicy::report());
        complete(writer.write_char_async('A'))?;
        assert_eq!(0, calls.get());
        let mut context = test_context();
        {
            let mut future = Box::pin(writer.finish_async());
            assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));
        }
        let expected: &[u8] = if during_flush { b"^A!" } else { b"^" };
        assert_eq!(expected, writer.output().bytes.as_slice());
        assert!(!writer.output().flushed);
        assert_eq!(1, calls.get());
        assert_eq!(
            io::ErrorKind::InvalidInput,
            complete(writer.write_char_async('B')).unwrap_err().kind()
        );
        complete(writer.finish_async())?;
        assert_eq!(b"^A!", writer.output().bytes.as_slice());
        assert!(writer.output().flushed);
        assert_eq!(1, calls.get());
        complete(writer.finish_async())?;
        assert_eq!(1, calls.get());
        let (output, pending) = writer.into_parts();
        assert_eq!(b"^A!", output.bytes.as_slice());
        assert!(pending.is_empty());
    }
    Ok(())
}

#[test]
fn test_async_charset_writer_flush_cancellation_preserves_suffix_and_open_state() -> io::Result<()> {
    for during_flush in [false, true] {
        let mut output = ChunkedAsyncOutput::new(if during_flush { 64 } else { 1 }, false);
        output.flush_pending = during_flush;
        let mut writer = AsyncCharsetTextWriter::new(output, Utf8Codec, CharsetEncodePolicy::report());
        complete(writer.write_str_fully_async("abc"))?;
        let mut context = test_context();
        {
            let mut future = Box::pin(writer.flush_async());
            assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));
        }
        let expected: &[u8] = if during_flush { b"abc" } else { b"a" };
        assert_eq!(expected, writer.output().bytes.as_slice());
        assert!(!writer.output().flushed);
        complete(writer.flush_async())?;
        assert_eq!(b"abc", writer.output().bytes.as_slice());
        assert!(writer.output().flushed);
        complete(writer.write_char_async('d'))?;
        complete(writer.finish_async())?;
        let (output, pending) = writer.into_parts();
        assert_eq!(b"abcd", output.bytes.as_slice());
        assert!(pending.is_empty());
    }
    Ok(())
}

fn poll_until_position<F: Future>(
    mut future: Pin<&mut F>,
    context: &mut Context<'_>,
    progress: &Cell<usize>,
    target: usize,
) -> bool {
    for _ in 0..128 {
        match future.as_mut().poll(context) {
            Poll::Pending if progress.get() == target => return true,
            Poll::Pending => {}
            Poll::Ready(_) => return false,
        }
    }
    false
}

#[test]
fn test_async_charset_reader_decodes_across_pending_and_byte_boundaries() -> io::Result<()> {
    let input = ChunkedAsyncInput::new("中🙂\nnext".as_bytes().to_vec(), 1, true);
    let mut reader =
        AsyncCharsetTextReader::new_with_buffer_capacity(input, Utf8Codec, CharsetDecodePolicy::report(), 1);
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
fn test_async_charset_reader_finish_tail_characters_survive_eof() -> io::Result<()> {
    let input = ChunkedAsyncInput::new(Vec::new(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(
        input,
        ScriptedCodec::new(ScriptedCodecMode::DecodeFinishTail),
        CharsetDecodePolicy::report(),
    );
    assert_eq!(Some('X'), complete(reader.read_char_async())?);
    assert_eq!(Some('Y'), complete(reader.read_char_async())?);
    assert_eq!(None, complete(reader.read_char_async())?);
    assert_eq!(None, complete(reader.read_char_async())?);

    let input = ChunkedAsyncInput::new(Vec::new(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(
        input,
        ScriptedCodec::new(ScriptedCodecMode::DecodeFinishTail),
        CharsetDecodePolicy::report(),
    );
    let mut chars = Vec::new();
    assert_eq!(1, complete(reader.read_chars_async(&mut chars, 1))?);
    assert_eq!(vec!['X'], chars);
    chars.clear();
    assert_eq!(1, complete(reader.read_chars_async(&mut chars, 1))?);
    assert_eq!(vec!['Y'], chars);
    chars.clear();
    assert_eq!(0, complete(reader.read_chars_async(&mut chars, 1))?);

    let input = ChunkedAsyncInput::new(Vec::new(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(
        input,
        ScriptedCodec::new(ScriptedCodecMode::DecodeFinishLines),
        CharsetDecodePolicy::report(),
    );
    let mut line = String::new();
    assert!(complete(reader.read_line_async(&mut line))?);
    assert_eq!("X\n", line);
    line.clear();
    assert!(complete(reader.read_line_async(&mut line))?);
    assert_eq!("Y", line);
    line.clear();
    assert!(!complete(reader.read_line_async(&mut line))?);
    assert!(line.is_empty());

    let input = ChunkedAsyncInput::new(Vec::new(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(
        input,
        ScriptedCodec::new(ScriptedCodecMode::DecodeFinishTail),
        CharsetDecodePolicy::report(),
    );
    assert_eq!(Some('X'), complete(reader.read_char_async())?);
    let mut output = String::new();
    complete(reader.read_to_string_async(&mut output))?;
    assert_eq!("Y", output);
    Ok(())
}

#[test]
fn test_async_charset_reader_accepts_common_line_endings_by_default() -> io::Result<()> {
    let input = ChunkedAsyncInput::new("first\r\nsecond\rthird\nend".as_bytes().to_vec(), 1, true);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut line = String::new();

    assert!(complete(reader.read_line_async(&mut line))?);
    assert_eq!("first\r\n", line);
    line.clear();
    assert!(complete(reader.read_line_async(&mut line))?);
    assert_eq!("second\r", line);
    line.clear();
    assert!(complete(reader.read_line_async(&mut line))?);
    assert_eq!("third\n", line);
    line.clear();
    assert!(complete(reader.read_line_async(&mut line))?);
    assert_eq!("end", line);
    Ok(())
}

#[test]
fn test_async_charset_reader_preserves_pending_line_tail_for_all_read_methods() -> io::Result<()> {
    let input = ChunkedAsyncInput::new(b"first\rsecond".to_vec(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut line = String::new();
    assert!(complete(reader.read_line_async(&mut line))?);
    let mut chars = Vec::new();
    assert_eq!(1, complete(reader.read_chars_async(&mut chars, 1))?);
    assert_eq!(vec!['s'], chars);

    let input = ChunkedAsyncInput::new(b"first\rsecond".to_vec(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    assert!(complete(reader.read_line_async(&mut line))?);
    let mut output = String::new();
    assert_eq!(6, complete(reader.read_to_string_async(&mut output))?);
    assert_eq!("second", output);

    let input = ChunkedAsyncInput::new(b"first\rsecond".to_vec(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    assert!(complete(reader.read_line_async(&mut line))?);
    let parts = reader.into_parts();
    assert_eq!(&['s'], &parts.pending_chars[..1]);
    Ok(())
}

#[test]
fn test_async_charset_reader_configured_line_endings_cover_crlf_paths() -> io::Result<()> {
    let input = ChunkedAsyncInput::new(b"first\rsecond\r\nlast".to_vec(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report())
        .with_line_endings(LineEndingSet::CRLF);
    let mut line = String::new();
    assert!(complete(reader.read_line_async(&mut line))?);
    assert_eq!("first\rsecond\r\n", line);

    let input = ChunkedAsyncInput::new(b"tail\r".to_vec(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report())
        .with_line_endings(LineEndingSet::CRLF);
    line.clear();
    assert!(complete(reader.read_line_async(&mut line))?);
    assert_eq!("tail\r", line);

    let input = ChunkedAsyncInput::new(b"tail\rnext".to_vec(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report())
        .with_line_endings(LineEndingSet::CR);
    line.clear();
    assert!(complete(reader.read_line_async(&mut line))?);
    assert_eq!("tail\r", line);

    let input = ChunkedAsyncInput::new(vec![0xF0], 1, false);
    let mut reader = AsyncCharsetTextReader::new(
        input,
        ScriptedCodec::new(ScriptedCodecMode::Lifecycle),
        CharsetDecodePolicy::report(),
    );
    assert_eq!(Some('^'), complete(reader.read_char_async())?);
    let error = complete(reader.read_char_async()).expect_err("incomplete scripted EOF must be rejected");
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    Ok(())
}

#[test]
fn test_async_charset_reader_limited_cr_line_is_a_boundary() -> io::Result<()> {
    let input = ChunkedAsyncInput::new(b"first\nnext".to_vec(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut line = String::new();
    assert!(complete(reader.read_line_limited_async(&mut line, 16))?);
    assert_eq!("first\n", line);

    let input = ChunkedAsyncInput::new(b"tail".to_vec(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    line.clear();
    assert!(complete(reader.read_line_limited_async(&mut line, 16))?);
    assert_eq!("tail", line);

    let input = ChunkedAsyncInput::new(b"tail".to_vec(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut output = String::new();
    assert_eq!(4, complete(reader.read_to_string_limited_async(&mut output, 16))?);
    assert_eq!("tail", output);
    Ok(())
}

#[test]
fn test_limited_crlf_overflow_preserves_next_record() -> io::Result<()> {
    let input = ChunkedAsyncInput::new(b"a\r\nok\r\n".to_vec(), 1, true);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report())
        .with_line_endings(LineEndingSet::CRLF);
    let mut output = String::from("seed:");
    let error = complete(reader.read_line_limited_async(&mut output, 2)).expect_err("LF exceeds limit");
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert_eq!("seed:", output);
    assert!(complete(reader.read_line_limited_async(&mut output, 16))?);
    assert_eq!("seed:ok\r\n", output);
    assert!(!complete(reader.read_line_limited_async(&mut output, 16))?);

    for line_endings in [
        LineEndingSet::CRLF,
        LineEndingSet::ALL,
        LineEndingSet::CRLF.with(LineEnding::Cr),
    ] {
        for limit in 0..=3 {
            let input = ChunkedAsyncInput::new(b"a\r\nok\r\n".to_vec(), 1, true);
            let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report())
                .with_line_endings(line_endings);
            let mut output = String::new();
            if limit < 3 {
                let error =
                    complete(reader.read_line_limited_async(&mut output, limit)).expect_err("first line exceeds limit");
                assert_eq!(io::ErrorKind::InvalidData, error.kind());
                assert_eq!("", output);
            } else {
                assert!(complete(reader.read_line_limited_async(&mut output, limit))?);
                assert_eq!("a\r\n", output);
            }
            output.clear();
            assert!(complete(reader.read_line_limited_async(&mut output, 16))?);
            assert_eq!("ok\r\n", output);
        }
    }
    Ok(())
}

#[cfg(coverage)]
#[test]
fn test_async_charset_reader_covers_eof_status_recovery_branches() -> io::Result<()> {
    let input = ChunkedAsyncInput::new(Vec::new(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    AsyncCharsetTextReader::<ChunkedAsyncInput, Utf8Codec>::coverage_force_next_eof_status(1);
    let error = complete(reader.read_char_async()).expect_err("forced EOF input request should be rejected");
    assert_eq!(io::ErrorKind::InvalidData, error.kind());

    let input = ChunkedAsyncInput::new(Vec::new(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    AsyncCharsetTextReader::<ChunkedAsyncInput, Utf8Codec>::coverage_force_next_eof_status(2);
    assert_eq!(None, complete(reader.read_char_async())?);
    Ok(())
}

#[test]
fn test_async_charset_reader_retains_partial_character_when_future_is_cancelled() -> io::Result<()> {
    let input = ChunkedAsyncInput::new("中".as_bytes().to_vec(), 1, false);
    let mut reader =
        AsyncCharsetTextReader::new_with_buffer_capacity(input, Utf8Codec, CharsetDecodePolicy::report(), 1);
    let mut context = test_context();

    let mut future = Box::pin(reader.read_char_async());
    assert!(future.as_mut().poll(&mut context).is_pending());
    drop(future);

    assert_eq!(Some('中'), complete(reader.read_char_async())?);
    assert_eq!(None, complete(reader.read_char_async())?);
    Ok(())
}

#[test]
fn test_async_charset_reader_line_cancellation_preserves_cr_lookahead() -> io::Result<()> {
    let progress = Rc::new(Cell::new(0));
    let input = ChunkedAsyncInput::new(b"first\r\nnext".to_vec(), 1, false).with_progress(progress.clone());
    let mut reader =
        AsyncCharsetTextReader::new_with_buffer_capacity(input, Utf8Codec, CharsetDecodePolicy::report(), 1);
    let mut line = String::new();
    let mut context = test_context();
    let mut future = Box::pin(reader.read_line_async(&mut line));
    assert!(poll_until_position(
        future.as_mut(),
        &mut context,
        &progress,
        b"first\r".len(),
    ));
    drop(future);
    assert_eq!("first", line);
    assert!(complete(reader.read_line_async(&mut line))?);
    assert_eq!("first\r\n", line);
    Ok(())
}

#[test]
fn test_rejected_line_survives_cancellation() -> io::Result<()> {
    let progress = Rc::new(Cell::new(0));
    let input = ChunkedAsyncInput::new(b"abcdef\nok\n".to_vec(), 1, false).with_progress(progress.clone());
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut output = String::from("seed:");
    let mut context = test_context();
    let mut future = Box::pin(reader.read_line_limited_async(&mut output, 4));
    let mut stopped = false;
    for _ in 0..128 {
        match future.as_mut().poll(&mut context) {
            Poll::Pending if progress.get() == 5 => {
                stopped = true;
                break;
            }
            Poll::Pending => {}
            Poll::Ready(result) => panic!("expected cancellation point, got {result:?}"),
        }
    }
    assert!(stopped, "discard must suspend after consuming e");
    drop(future);
    assert_eq!("seed:", output);
    let error = complete(reader.read_line_limited_async(&mut output, 4)).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert_eq!("seed:", output);
    assert!(complete(reader.read_line_limited_async(&mut output, 4))?);
    assert_eq!("seed:ok\n", output);
    Ok(())
}

#[test]
fn test_rejected_line_survives_repeated_cancellation() -> io::Result<()> {
    let progress = Rc::new(Cell::new(0));
    let input = ChunkedAsyncInput::new(b"abcdefgh\nok\n".to_vec(), 1, false).with_progress(progress.clone());
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut output = String::from("seed:");
    let mut context = test_context();
    let mut future = Box::pin(reader.read_line_limited_async(&mut output, 4));
    assert!(poll_until_position(future.as_mut(), &mut context, &progress, 5));
    drop(future);
    let mut future = Box::pin(reader.read_line_limited_async(&mut output, 4));
    assert!(poll_until_position(future.as_mut(), &mut context, &progress, 6));
    drop(future);

    let error = complete(reader.read_line_limited_async(&mut output, 4)).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert_eq!("seed:", output);
    assert!(complete(reader.read_line_limited_async(&mut output, 4))?);
    assert_eq!("seed:ok\n", output);
    Ok(())
}

#[test]
fn test_rejected_line_preserves_crlf_boundary_and_config_snapshot() -> io::Result<()> {
    let progress = Rc::new(Cell::new(0));
    let input = ChunkedAsyncInput::new(b"abcde\r\nok\n".to_vec(), 1, false).with_progress(progress.clone());
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report())
        .with_line_endings(LineEndingSet::CRLF);
    let mut output = String::new();
    let mut context = test_context();
    let mut future = Box::pin(reader.read_line_limited_async(&mut output, 4));
    assert!(poll_until_position(future.as_mut(), &mut context, &progress, 6));
    drop(future);
    reader = reader.with_line_endings(LineEndingSet::CR);
    let error = complete(reader.read_line_async(&mut output)).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert!(error.to_string().contains("4-byte append limit"));
    assert_eq!("", output);
    assert!(complete(reader.read_line_async(&mut output))?);
    assert_eq!("ok\n", output);
    Ok(())
}

#[test]
fn test_rejected_line_recovers_cr_and_lf_endings_at_eof() -> io::Result<()> {
    for (line_ending, endings) in [(b'\r', LineEndingSet::CR), (b'\n', LineEndingSet::LF)] {
        let mut bytes = b"abcde".to_vec();
        bytes.extend([line_ending, b'o', b'k', line_ending]);
        let progress = Rc::new(Cell::new(0));
        let input = ChunkedAsyncInput::new(bytes, 1, false).with_progress(progress.clone());
        let mut reader =
            AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report()).with_line_endings(endings);
        let mut output = String::new();
        let mut context = test_context();
        let mut future = Box::pin(reader.read_line_limited_async(&mut output, 4));
        assert!(poll_until_position(future.as_mut(), &mut context, &progress, 5));
        drop(future);
        let error = complete(reader.read_line_limited_async(&mut output, 4)).unwrap_err();
        assert_eq!(io::ErrorKind::InvalidData, error.kind());
        assert!(complete(reader.read_line_limited_async(&mut output, 8))?);
        assert_eq!(format!("ok{}", line_ending as char), output);
    }

    let progress = Rc::new(Cell::new(0));
    let input = ChunkedAsyncInput::new(b"abcdef".to_vec(), 1, false).with_progress(progress.clone());
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut output = String::new();
    let mut context = test_context();
    let mut future = Box::pin(reader.read_line_limited_async(&mut output, 4));
    assert!(poll_until_position(future.as_mut(), &mut context, &progress, 5));
    drop(future);
    let error = complete(reader.read_line_limited_async(&mut output, 4)).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert!(!complete(reader.read_line_limited_async(&mut output, 4))?);
    Ok(())
}

#[test]
fn test_rejected_line_preserves_state_after_one_shot_io_error() -> io::Result<()> {
    let input =
        ChunkedAsyncInput::new(b"abcdef\nok\n".to_vec(), 1, false).with_error_at_position(5, io::ErrorKind::Other);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut output = String::from("seed:");
    let error = complete(reader.read_line_limited_async(&mut output, 4)).unwrap_err();
    assert_eq!(io::ErrorKind::Other, error.kind());
    assert_eq!("seed:", output);
    let error = complete(reader.read_line_limited_async(&mut output, 99)).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert_eq!("seed:", output);
    assert!(complete(reader.read_line_limited_async(&mut output, 99))?);
    assert_eq!("seed:ok\n", output);
    Ok(())
}

#[test]
fn test_rejected_line_keeps_original_limit_when_resumed_unbounded() -> io::Result<()> {
    let progress = Rc::new(Cell::new(0));
    let input = ChunkedAsyncInput::new(b"abcdef\nok\n".to_vec(), 1, false).with_progress(progress.clone());
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut output = String::new();
    let mut context = test_context();
    let mut future = Box::pin(reader.read_line_limited_async(&mut output, 4));
    assert!(poll_until_position(future.as_mut(), &mut context, &progress, 5));
    drop(future);
    let error = complete(reader.read_line_async(&mut output)).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert!(error.to_string().contains("4-byte append limit"));
    assert!(complete(reader.read_line_async(&mut output))?);
    assert_eq!("ok\n", output);
    Ok(())
}

#[test]
fn test_line_prefix_cancellation_keeps_append_limit_per_call() -> io::Result<()> {
    let progress = Rc::new(Cell::new(0));
    let input = ChunkedAsyncInput::new(b"abcde".to_vec(), 1, false).with_progress(progress.clone());
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut output = String::new();
    let mut context = test_context();
    let mut future = Box::pin(reader.read_line_limited_async(&mut output, 4));
    assert!(poll_until_position(future.as_mut(), &mut context, &progress, 2));
    drop(future);
    assert_eq!("ab", output);

    assert!(complete(reader.read_line_limited_async(&mut output, 4))?);
    assert_eq!("abcde", output);
    Ok(())
}

#[test]
fn test_async_charset_reader_limited_reads_preserve_pending_order_and_endings() -> io::Result<()> {
    let input = ChunkedAsyncInput::new(b"first\rbc".to_vec(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut line = String::new();
    assert!(complete(reader.read_line_async(&mut line))?);
    assert_eq!("first\r", line);
    let mut output = String::new();
    assert_eq!(2, complete(reader.read_to_string_limited_async(&mut output, 8))?);
    assert_eq!("bc", output);

    let input = ChunkedAsyncInput::new(b"first\rsecond\nthird".to_vec(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report())
        .with_line_endings(LineEndingSet::CR);
    line.clear();
    assert!(complete(reader.read_line_limited_async(&mut line, 32))?);
    assert_eq!("first\r", line);
    line.clear();
    assert!(complete(reader.read_line_limited_async(&mut line, 32))?);
    assert_eq!("second\nthird", line);
    Ok(())
}

#[test]
fn test_async_charset_reader_covers_limited_line_ending_branches() -> io::Result<()> {
    let mut reader = AsyncCharsetTextReader::new(
        ChunkedAsyncInput::new(b"a\r\nb".to_vec(), 1, false),
        Utf8Codec,
        CharsetDecodePolicy::report(),
    );
    let mut line = String::new();
    assert!(complete(reader.read_line_limited_async(&mut line, 8))?);
    assert_eq!("a\r\n", line);

    let mut reader = AsyncCharsetTextReader::new(
        ChunkedAsyncInput::new(b"\r".to_vec(), 1, false),
        Utf8Codec,
        CharsetDecodePolicy::report(),
    );
    let error = complete(reader.read_line_limited_async(&mut String::new(), 0))
        .expect_err("CR should exceed a zero-byte limit");
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert_eq!(None, complete(reader.read_char_async())?);

    let mut reader = AsyncCharsetTextReader::new(
        ChunkedAsyncInput::new(b"\r\n".to_vec(), 1, false),
        Utf8Codec,
        CharsetDecodePolicy::report(),
    );
    let error = complete(reader.read_line_limited_async(&mut String::new(), 1))
        .expect_err("the LF in CRLF should exceed the limit");
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert_eq!(None, complete(reader.read_char_async())?);

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
        let mut reader = AsyncCharsetTextReader::new(
            ChunkedAsyncInput::new(input.to_vec(), 1, false),
            Utf8Codec,
            CharsetDecodePolicy::report(),
        )
        .with_line_endings(endings);
        let mut line = String::new();
        assert!(complete(reader.read_line_limited_async(&mut line, 8))?);
        assert_eq!(expected, line);
    }
    Ok(())
}

#[test]
fn test_async_charset_reader_applies_incomplete_eof_policy() -> io::Result<()> {
    let input = ChunkedAsyncInput::new(vec![0xE4, 0xB8], 1, true);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::replace('!'));
    let mut text = String::new();

    assert_eq!(1, complete(reader.read_to_string_async(&mut text))?);
    assert_eq!("!", text);

    let input = ChunkedAsyncInput::new(vec![0xE4, 0xB8], 1, true);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let error = complete(reader.read_char_async()).expect_err("strict mode must reject an incomplete EOF tail");
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    Ok(())
}

#[test]
fn test_async_charset_reader_accessors_and_bulk_reads_cover_buffered_state() -> io::Result<()> {
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
fn test_async_charset_reader_into_parts_preserves_unreturned_characters() -> io::Result<()> {
    let input = ChunkedAsyncInput::new(b"abc".to_vec(), 3, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());

    assert_eq!(Some('a'), complete(reader.read_char_async())?);

    let parts = reader.into_parts();
    assert_eq!(3, parts.input.position);
    assert!(parts.unread_bytes.readable().is_empty());
    assert_eq!(['b', 'c'], parts.pending_chars.as_slice());
    Ok(())
}

#[test]
fn test_async_charset_reader_parts_preserve_chars_bytes_and_source() -> io::Result<()> {
    let input = ChunkedAsyncInput::new("abc🙂tail".as_bytes().to_vec(), 4, false);
    let mut reader =
        AsyncCharsetTextReader::new_with_buffer_capacity(input, Utf8Codec, CharsetDecodePolicy::report(), 4);
    assert_eq!(Some('a'), complete(reader.read_char_async())?);
    let mut parts = reader.into_parts();
    assert_eq!(4, parts.input.position);
    assert_eq!(['b', 'c'], parts.pending_chars.as_slice());
    assert_eq!([0xf0], parts.unread_bytes.readable());

    let mut bytes = parts.unread_bytes.readable().to_vec();
    bytes.extend_from_slice(&parts.input.bytes[parts.input.position..]);
    let mut restored: String = parts.pending_chars.into_iter().collect();
    restored.push_str(std::str::from_utf8(&bytes).expect("valid UTF-8 remainder"));
    assert_eq!("bc🙂tail", restored);

    let mut chars = ['\0'; 16];
    let progress = parts
        .decoder
        .transcode_eof(&bytes, 0, &mut chars, 0)
        .expect("the returned decoder remains started");
    assert_eq!(bytes.len(), progress.read());
    assert_eq!("🙂tail", chars[..progress.written()].iter().collect::<String>());
    assert_eq!(
        0,
        parts
            .decoder
            .finish(&mut chars, 0)
            .expect("finish the retained decoder")
    );
    Ok(())
}

#[test]
fn test_async_charset_reader_parts_preserve_partial_input_after_cancel() {
    let input = ChunkedAsyncInput::new("🙂tail".as_bytes().to_vec(), 1, false);
    let mut reader =
        AsyncCharsetTextReader::new_with_buffer_capacity(input, Utf8Codec, CharsetDecodePolicy::report(), 4);
    let mut future = Box::pin(reader.read_char_async());
    let mut context = Context::from_waker(Waker::noop());
    assert!(future.as_mut().poll(&mut context).is_pending());
    drop(future);

    let parts = reader.into_parts();
    assert_eq!(1, parts.input.position);
    assert_eq!([0xf0], parts.unread_bytes.readable());
    assert!(parts.pending_chars.is_empty());
    let mut bytes = parts.unread_bytes.readable().to_vec();
    bytes.extend_from_slice(&parts.input.bytes[parts.input.position..]);
    assert_eq!("🙂tail", std::str::from_utf8(&bytes).expect("retained UTF-8 tail"));
}

#[test]
fn test_async_charset_reader_compacts_partial_tail_across_pending_reads() -> io::Result<()> {
    let input = ChunkedAsyncInput::new("A中".as_bytes().to_vec(), 3, true);
    let mut reader =
        AsyncCharsetTextReader::new_with_buffer_capacity(input, Utf8Codec, CharsetDecodePolicy::report(), 1);
    let mut output = String::new();

    assert_eq!(2, complete(reader.read_to_string_async(&mut output))?);
    assert_eq!("A中", output);
    Ok(())
}

#[test]
fn test_async_charset_reader_read_line_limited_restores_output_on_overflow() -> io::Result<()> {
    let input = ChunkedAsyncInput::new("a中\nnext".as_bytes().to_vec(), 2, true);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut output = String::from("prefix-");

    let error = complete(reader.read_line_limited_async(&mut output, 4)).expect_err("oversized line should fail");
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert_eq!("prefix-", output);

    output.clear();
    assert!(complete(reader.read_line_limited_async(&mut output, 16))?);
    assert_eq!("next", output);
    Ok(())
}

#[test]
fn test_async_charset_reader_propagates_input_and_decode_errors() {
    let input = ChunkedAsyncInput::new(Vec::new(), 1, false).with_error(io::ErrorKind::BrokenPipe);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let error = complete(reader.read_char_async()).expect_err("scripted input failure should propagate");
    assert_eq!(io::ErrorKind::BrokenPipe, error.kind());

    let input = ChunkedAsyncInput::new(vec![0xFF], 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let error = complete(reader.read_char_async()).expect_err("malformed UTF-8 should propagate");
    assert_eq!(io::ErrorKind::InvalidData, error.kind());

    let input = ChunkedAsyncInput::new(Vec::new(), 1, false).with_error(io::ErrorKind::ConnectionReset);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut chars = Vec::new();
    let error = complete(reader.read_chars_async(&mut chars, 1)).expect_err("bulk read failure should propagate");
    assert_eq!(io::ErrorKind::ConnectionReset, error.kind());

    let input = ChunkedAsyncInput::new(Vec::new(), 1, false).with_error(io::ErrorKind::ConnectionAborted);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut text = String::new();
    let error = complete(reader.read_to_string_async(&mut text)).expect_err("string read failure should propagate");
    assert_eq!(io::ErrorKind::ConnectionAborted, error.kind());

    let input = ChunkedAsyncInput::new(Vec::new(), 1, false).with_error(io::ErrorKind::TimedOut);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let error = complete(reader.read_line_async(&mut text)).expect_err("line read failure should propagate");
    assert_eq!(io::ErrorKind::TimedOut, error.kind());
}

#[test]
fn test_async_charset_reader_grows_lifecycle_and_incomplete_buffers() -> io::Result<()> {
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
fn test_async_charset_reader_propagates_lifecycle_errors() -> io::Result<()> {
    let input = ChunkedAsyncInput::new(Vec::new(), 4, false);
    let mut reader = AsyncCharsetTextReader::new(
        input,
        ScriptedCodec::new(ScriptedCodecMode::DecodeResetError),
        CharsetDecodePolicy::report(),
    );
    let error = complete(reader.read_char_async()).expect_err("decoder reset error should propagate");
    assert_eq!(io::ErrorKind::InvalidData, error.kind());

    let input = ChunkedAsyncInput::new(Vec::new(), 4, false);
    let mut reader = AsyncCharsetTextReader::new(
        input,
        ScriptedCodec::new(ScriptedCodecMode::DecodeFinishError),
        CharsetDecodePolicy::report(),
    );
    assert_eq!(Some('^'), complete(reader.read_char_async())?);
    let error = complete(reader.read_char_async()).expect_err("decoder finish error should propagate");
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    Ok(())
}

#[test]
fn test_async_charset_writer_encodes_and_flushes_async_only_output() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(2, true);
    let mut writer =
        AsyncCharsetTextWriter::new_with_buffer_capacity(output, Utf8Codec, CharsetEncodePolicy::report(), 3)
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
fn test_async_charset_reader_trait_methods_forward_to_reader() -> io::Result<()> {
    let input = ChunkedAsyncInput::new(b"A\nB".to_vec(), 4, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());

    assert_eq!(Some('A'), complete(AsyncTextReader::read_char_async(&mut reader))?);

    let mut chars = Vec::new();
    assert_eq!(
        1,
        complete(AsyncTextReader::read_chars_async(&mut reader, &mut chars, 1))?
    );
    assert_eq!(vec!['\n'], chars);

    let mut text = String::new();
    assert_eq!(
        1,
        complete(AsyncTextReader::read_to_string_async(&mut reader, &mut text))?
    );
    assert_eq!("B", text);

    let input = ChunkedAsyncInput::new(b"line\n".to_vec(), 4, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut line = String::new();
    assert!(complete(AsyncTextLineReader::read_line_async(&mut reader, &mut line))?);
    assert_eq!("line\n", line);
    Ok(())
}

#[test]
fn test_async_charset_writer_trait_methods_forward_to_writer() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(64, false);
    let mut writer = AsyncCharsetTextWriter::new(output, Utf8Codec, CharsetEncodePolicy::report())
        .with_line_ending(LineEnding::CrLf);

    assert_eq!(LineEnding::CrLf, AsyncTextWriter::line_ending(&writer));
    complete(AsyncTextWriter::write_char_async(&mut writer, 'A'))?;
    assert_eq!(
        2,
        complete(AsyncTextWriter::write_chars_async(&mut writer, &['B', 'C']))?
    );
    assert_eq!(1, complete(AsyncTextWriter::write_str_async(&mut writer, "D"))?);
    complete(AsyncTextWriter::write_line_fully_async(&mut writer, "E"))?;
    complete(AsyncTextWriter::flush_async(&mut writer))?;
    complete(AsyncTextWriter::finish_async(&mut writer))?;

    let (output, pending) = writer.into_parts();
    assert!(pending.is_empty());
    assert_eq!(b"ABCDE\r\n", output.bytes.as_slice());
    assert!(output.flushed);
    Ok(())
}

#[test]
fn test_async_charset_writer_commits_source_before_later_delivery_can_pend() -> io::Result<()> {
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
fn test_async_charset_writer_accessors_empty_chunks_and_finished_state() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(4, false);
    let mut writer =
        AsyncCharsetTextWriter::new(output, Utf8Codec, CharsetEncodePolicy::report()).with_line_ending(LineEnding::Cr);

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

    let error = complete(writer.write_char_async('x')).expect_err("character write after finish should fail");
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
    let error = complete(writer.write_chars_async(&['x'])).expect_err("character slice write after finish should fail");
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
    let error = complete(writer.write_str_async("x")).expect_err("string write after finish should fail");
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
    Ok(())
}

#[test]
fn test_async_charset_writer_grows_across_need_output_and_pending_writes() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(1, true);
    let mut writer =
        AsyncCharsetTextWriter::new_with_buffer_capacity(output, Utf8Codec, CharsetEncodePolicy::report(), 1);

    complete(writer.write_chars_fully_async(&['a', 'b', 'c', 'd', 'e']))?;
    complete(writer.finish_async())?;
    let (output, pending) = writer.into_parts();
    assert!(pending.is_empty());
    assert_eq!(b"abcde", output.bytes.as_slice());
    Ok(())
}

#[test]
fn test_async_charset_writer_retains_bytes_after_zero_and_write_errors() -> io::Result<()> {
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
fn test_async_charset_writer_propagates_flush_errors() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(4, false).with_flush_error(io::ErrorKind::BrokenPipe);
    let mut writer = AsyncCharsetTextWriter::new(output, Utf8Codec, CharsetEncodePolicy::report());

    let error = complete(writer.flush_async()).expect_err("scripted flush error should propagate");
    assert_eq!(io::ErrorKind::BrokenPipe, error.kind());
    complete(writer.flush_async())?;
    Ok(())
}

#[test]
fn test_async_charset_writer_finish_error_leaves_writer_available_for_retry() -> io::Result<()> {
    let output = ChunkedAsyncOutput::new(4, false).with_flush_error(io::ErrorKind::BrokenPipe);
    let mut writer = AsyncCharsetTextWriter::new(output, Utf8Codec, CharsetEncodePolicy::report());

    let error = complete(writer.finish_async()).expect_err("finish should report the scripted flush error");
    assert_eq!(io::ErrorKind::BrokenPipe, error.kind());
    complete(writer.finish_async())?;
    let (output, pending) = writer.into_parts();

    assert!(pending.is_empty());
    assert!(output.flushed);
    Ok(())
}

#[test]
fn test_async_charset_writer_grows_and_emits_codec_lifecycle_output() -> io::Result<()> {
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
fn test_async_charset_writer_propagates_codec_lifecycle_errors() {
    let output = ChunkedAsyncOutput::new(64, false);
    let mut writer = AsyncCharsetTextWriter::new(
        output,
        ScriptedCodec::new(ScriptedCodecMode::EncodeResetError),
        CharsetEncodePolicy::report(),
    );
    let error = complete(writer.write_char_async('A')).expect_err("encoder reset error should propagate");
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());

    let output = ChunkedAsyncOutput::new(64, false);
    let mut writer = AsyncCharsetTextWriter::new(
        output,
        ScriptedCodec::new(ScriptedCodecMode::EncodeValueError),
        CharsetEncodePolicy::report(),
    );
    let error = complete(writer.write_char_async('A')).expect_err("encoder value error should propagate");
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
    assert!(writer.output().bytes.is_empty());
    let (_, pending) = writer.into_parts();
    assert_eq!(b"^", pending.as_slice());

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
fn test_async_charset_reader_limited_read_rolls_back_appended_text() {
    let input = ChunkedAsyncInput::new("A中".as_bytes().to_vec(), 1, false);
    let mut reader = AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
    let mut output = String::from("prefix:");

    let error = complete(reader.read_to_string_limited_async(&mut output, 3))
        .expect_err("decoded text beyond the append limit must fail");

    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert_eq!("prefix:", output);
}

/// Checks actual codecs against independent byte fixtures at async boundaries.
fn assert_async_multibyte<C>(codec: C, expected: &[u8], unit_width: usize) -> io::Result<()>
where
    C: CharsetCodec<Unit = u8> + Copy,
{
    let text = "A中🙂\r\nZ";
    for capacity in [1, 2, 3, 4, 8] {
        let mut writer = AsyncCharsetTextWriter::new_with_buffer_capacity(
            ChunkedAsyncOutput::new(1, true),
            codec,
            CharsetEncodePolicy::report(),
            capacity,
        );
        let mut offset = 0;
        while offset < text.len() {
            let consumed = complete(writer.write_str_async(&text[offset..]))?;
            assert!(consumed > 0 && consumed <= text.len() - offset);
            assert!(text.is_char_boundary(offset + consumed));
            offset += consumed;
        }
        complete(writer.finish_async())?;
        let (output, pending) = writer.into_parts();
        assert!(pending.is_empty());
        assert_eq!(expected, output.bytes.as_slice());
        let mut reader = AsyncCharsetTextReader::new_with_buffer_capacity(
            ChunkedAsyncInput::new(expected.to_vec(), 1, true),
            codec,
            CharsetDecodePolicy::report(),
            capacity,
        );
        let mut decoded = String::new();
        assert_eq!(
            text.chars().count(),
            complete(reader.read_to_string_async(&mut decoded))?
        );
        assert_eq!(text, decoded);
    }

    for cut in 1..expected.len() {
        let progress = Rc::new(Cell::new(0));
        let input = ChunkedAsyncInput::new(expected.to_vec(), 1, false).with_progress(progress.clone());
        let mut reader =
            AsyncCharsetTextReader::new_with_buffer_capacity(input, codec, CharsetDecodePolicy::report(), 1);
        let mut decoded = String::new();
        let mut cx = test_context();
        let mut future = Box::pin(reader.read_to_string_async(&mut decoded));
        assert!(poll_until_position(future.as_mut(), &mut cx, &progress, cut));
        drop(future);
        complete(reader.read_to_string_async(&mut decoded))?;
        assert_eq!(text, decoded);
    }

    let progress = Rc::new(Cell::new(0));
    let input = ChunkedAsyncInput::new(expected.to_vec(), 1, false).with_progress(progress.clone());
    let mut reader = AsyncCharsetTextReader::new_with_buffer_capacity(input, codec, CharsetDecodePolicy::report(), 1);
    let mut line = String::from("seed:");
    let mut cx = test_context();
    let mut future = Box::pin(reader.read_line_limited_async(&mut line, 4));
    let after_cr = expected.len() - 2 * unit_width;
    assert!(poll_until_position(future.as_mut(), &mut cx, &progress, after_cr));
    drop(future);
    assert_eq!("seed:", line);
    assert_eq!(
        io::ErrorKind::InvalidData,
        complete(reader.read_line_limited_async(&mut line, 99))
            .unwrap_err()
            .kind(),
    );
    assert_eq!("seed:", line);
    line.clear();
    assert!(complete(reader.read_line_limited_async(&mut line, 4))?);
    assert_eq!("Z", line);
    line.clear();
    assert!(!complete(reader.read_line_limited_async(&mut line, 4))?);

    let mut reader = AsyncCharsetTextReader::new_with_buffer_capacity(
        ChunkedAsyncInput::new(expected[..expected.len() - 1].to_vec(), 1, true),
        codec,
        CharsetDecodePolicy::report(),
        1,
    );
    let error = complete(reader.read_to_string_async(&mut String::new())).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    Ok(())
}

#[test]
fn test_async_multibyte_utf16() -> io::Result<()> {
    let text = "A中🙂\r\nZ";
    let le: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let be: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
    assert_async_multibyte(Utf16ByteCodec::new(ByteOrder::LittleEndian), &le, 2)?;
    assert_async_multibyte(Utf16ByteCodec::new(ByteOrder::BigEndian), &be, 2)
}

#[test]
fn test_async_multibyte_utf32() -> io::Result<()> {
    let text = "A中🙂\r\nZ";
    let le: Vec<u8> = text.chars().flat_map(|ch| u32::from(ch).to_le_bytes()).collect();
    let be: Vec<u8> = text.chars().flat_map(|ch| u32::from(ch).to_be_bytes()).collect();
    assert_async_multibyte(Utf32ByteCodec::new(ByteOrder::LittleEndian), &le, 4)?;
    assert_async_multibyte(Utf32ByteCodec::new(ByteOrder::BigEndian), &be, 4)
}
