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
use std::pin::Pin;
use std::rc::Rc;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_codec_text::CharsetDecodePolicy;
use qubit_io::AsyncInput;
use qubit_io_text::AsyncUtf8TextReader;
use qubit_io_text::LineEnding;
use qubit_io_text::LineEndingSet;
use qubit_io_text::TextAppendLimitError;

/// Reads a fixed byte sequence without suspending.
struct ReadyInput {
    bytes: Vec<u8>,
    position: usize,
}

impl ReadyInput {
    /// Creates an input over `text`.
    fn new(text: &str) -> Self {
        Self {
            bytes: text.as_bytes().to_vec(),
            position: 0,
        }
    }
}

impl AsyncInput for ReadyInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        output: &mut [u8],
        output_index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        let count = count.min(self.bytes.len() - self.position);
        let input_end = self.position + count;
        let output_end = output_index + count;
        output[output_index..output_end].copy_from_slice(&self.bytes[self.position..input_end]);
        self.position = input_end;
        Poll::Ready(Ok(count))
    }
}

/// Returns `Pending` between one-byte reads to control cancellation points.
struct PendingAfterPrefixInput {
    bytes: Vec<u8>,
    position: usize,
    pending: bool,
    progress: Rc<Cell<usize>>,
}

impl PendingAfterPrefixInput {
    fn new(text: &str, progress: Rc<Cell<usize>>) -> Self {
        Self {
            bytes: text.as_bytes().to_vec(),
            position: 0,
            pending: true,
            progress,
        }
    }
}

impl AsyncInput for PendingAfterPrefixInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [u8],
        output_index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        if self.pending {
            self.pending = false;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.pending = true;
        let count = count.min(1).min(self.bytes.len() - self.position);
        let input_end = self.position + count;
        let output_end = output_index + count;
        output[output_index..output_end].copy_from_slice(&self.bytes[self.position..input_end]);
        self.position = input_end;
        self.progress.set(self.position);
        Poll::Ready(Ok(count))
    }
}

/// Polls a ready test future to completion.
fn complete<F>(future: F) -> F::Output
where
    F: Future,
{
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("ready input must not suspend"),
    }
}

fn complete_allowing_pending<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    for _ in 0..128 {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
    }
    panic!("scripted input did not become ready");
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
fn test_async_utf8_text_reader_decodes_text_and_exposes_inner_reader() -> io::Result<()> {
    let mut reader = AsyncUtf8TextReader::with_capacity(ReadyInput::new("A中"), CharsetDecodePolicy::report(), 1);
    let mut text = String::new();

    assert_eq!(2, complete(reader.read_to_string_async(&mut text))?);
    assert_eq!("A中", text);

    let reader = reader.into_inner();
    assert_eq!(4, reader.input().position);
    Ok(())
}

#[test]
fn test_async_utf8_text_reader_configuration_and_deref_accessors() {
    let mut reader = AsyncUtf8TextReader::new(ReadyInput::new("payload"));
    assert_eq!(LineEndingSet::ALL, reader.line_endings());
    assert_eq!(0, reader.input().position);
    reader.input_mut().position = 1;

    let reader = reader.with_line_endings(LineEndingSet::only(LineEnding::Cr));
    assert_eq!(LineEndingSet::CR, reader.line_endings());
    let reader = reader.into_inner();
    assert_eq!(1, reader.input().position);
}

#[test]
fn test_async_utf8_text_reader_forwards_all_async_read_methods() -> io::Result<()> {
    let mut reader = AsyncUtf8TextReader::with_policy(ReadyInput::new("a\nb"), CharsetDecodePolicy::report());
    assert_eq!(Some('a'), complete(reader.read_char_async())?);

    let mut chars = Vec::new();
    assert_eq!(1, complete(reader.read_chars_async(&mut chars, 1))?);
    assert_eq!(vec!['\n'], chars);

    let mut line = String::new();
    assert!(complete(reader.read_line_async(&mut line))?);
    assert_eq!("b", line);
    assert!(!complete(reader.read_line_async(&mut line))?);
    Ok(())
}

#[test]
fn test_async_utf8_text_reader_parts_preserve_chars_bytes_and_source() -> io::Result<()> {
    let mut reader = AsyncUtf8TextReader::with_capacity(ReadyInput::new("abc🙂tail"), CharsetDecodePolicy::report(), 4);
    assert_eq!(Some('a'), complete(reader.read_char_async())?);
    let parts = reader.into_inner().into_parts();
    assert_eq!(4, parts.input.position);
    assert_eq!(['b', 'c'], parts.pending_chars.as_slice());
    assert_eq!([0xf0], parts.unread_bytes.readable());

    let mut bytes = parts.unread_bytes.readable().to_vec();
    bytes.extend_from_slice(&parts.input.bytes[parts.input.position..]);
    let mut restored: String = parts.pending_chars.into_iter().collect();
    restored.push_str(std::str::from_utf8(&bytes).expect("valid UTF-8 remainder"));
    assert_eq!("bc🙂tail", restored);
    Ok(())
}

#[test]
fn test_async_utf8_text_reader_resumes_rejected_line_before_next_record() -> io::Result<()> {
    let mut reader = AsyncUtf8TextReader::new(ReadyInput::new("abcdef\nok\n"));
    let mut output = String::from("seed:");
    let error = complete(reader.read_line_limited_async(&mut output, 4)).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert_eq!("seed:", output);
    assert!(complete(reader.read_line_limited_async(&mut output, 4))?);
    assert_eq!("seed:ok\n", output);
    Ok(())
}

#[test]
fn test_async_utf8_text_reader_deref_exposes_per_call_string_limit() -> io::Result<()> {
    let mut reader = AsyncUtf8TextReader::new(ReadyInput::new("中文"));
    let mut output = String::from("seed:");
    let message_start = output.len();
    assert_eq!(2, complete(reader.read_to_string_limited_async(&mut output, 6))?);
    assert_eq!("seed:中文", output);
    assert_eq!(6, output.len() - message_start);

    let mut reader = AsyncUtf8TextReader::new(ReadyInput::new("中文"));
    let mut output = String::from("seed:");
    let error = complete(reader.read_to_string_limited_async(&mut output, 5)).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert_eq!("seed:", output);

    let mut reader = AsyncUtf8TextReader::new(ReadyInput::new(""));
    let mut output = String::new();
    assert_eq!(0, complete(reader.read_to_string_limited_async(&mut output, 0))?);

    let mut reader = AsyncUtf8TextReader::new(ReadyInput::new("a"));
    let mut output = String::new();
    assert_eq!(
        io::ErrorKind::InvalidData,
        complete(reader.read_to_string_limited_async(&mut output, 0))
            .unwrap_err()
            .kind()
    );
    assert_eq!("", output);
    Ok(())
}

#[test]
fn test_async_utf8_text_reader_deref_preserves_per_call_budget_after_cancel() -> io::Result<()> {
    let progress = Rc::new(Cell::new(0));
    let mut reader = AsyncUtf8TextReader::new(PendingAfterPrefixInput::new("中文", progress.clone()));
    let mut output = String::from("seed:");
    let message_start = output.len();
    let mut context = Context::from_waker(Waker::noop());
    let mut future = Box::pin(reader.read_to_string_limited_async(&mut output, 4));
    let pending = poll_until_position(future.as_mut(), &mut context, &progress, 3);
    drop(future);
    assert_eq!("seed:中", output);
    assert!(pending);

    assert_eq!(
        1,
        complete_allowing_pending(reader.read_to_string_limited_async(&mut output, 4))?
    );
    assert_eq!("seed:中文", output);
    assert_eq!(6, output.len() - message_start);

    let progress = Rc::new(Cell::new(0));
    let mut reader = AsyncUtf8TextReader::new(PendingAfterPrefixInput::new("中文", progress.clone()));
    let mut output = String::from("seed:");
    let mut future = Box::pin(reader.read_to_string_limited_async(&mut output, 4));
    assert!(poll_until_position(future.as_mut(), &mut context, &progress, 3));
    drop(future);
    let error = complete_allowing_pending(reader.read_to_string_limited_async(&mut output, 1)).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidData, error.kind());
    assert_eq!("seed:中", output);
    assert_eq!(
        Some(1),
        error
            .get_ref()
            .and_then(|source| source.downcast_ref::<TextAppendLimitError>())
            .map(TextAppendLimitError::max_append_len)
    );
    Ok(())
}
