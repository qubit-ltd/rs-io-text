// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_codec::TranscodeEncodeError;
use qubit_codec_text::AsciiCodec;
use qubit_codec_text::CharsetEncodeError;
use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::Latin1Codec;
use qubit_codec_text::Utf8Codec;
use qubit_io::AsyncOutput;
use qubit_io_text::AsyncCharsetTextWriter;
use qubit_io_text::AsyncTextWriter;
use qubit_io_text::AsyncUtf8TextWriter;

/// Records delivery and injects one error after accepting a byte prefix.
#[derive(Default)]
struct RecoveryOutput {
    bytes: Vec<u8>,
    writes: usize,
    flushes: usize,
}

#[derive(Default)]
struct IndexOutput {
    bytes: Vec<u8>,
}

impl AsyncOutput for IndexOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        self.bytes.extend_from_slice(&input[index..index + count]);
        Poll::Ready(Ok(count))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn assert_complete_main_index(error: &io::Error, expected: usize) {
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
    let typed = error
        .get_ref()
        .and_then(|source| source.downcast_ref::<TranscodeEncodeError<CharsetEncodeError, char>>())
        .expect("transcode error remains downcastable");
    let context = typed.domain_error_ref().expect("charset main error");
    let source = typed.domain_ref().expect("charset source remains available");
    assert_eq!(Some('中' as u32), source.value());
    assert_eq!((Some(expected), expected), (context.input_index(), source.index()));
}

impl AsyncOutput for RecoveryOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        self.writes += 1;
        if self.writes == 2 {
            return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "scripted failure")));
        }
        let take = if self.writes == 1 { count.min(1) } else { count };
        self.bytes.extend_from_slice(&input[index..index + take]);
        Poll::Ready(Ok(take))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.flushes += 1;
        Poll::Ready(Ok(()))
    }
}

/// Completes futures driven by the immediately ready scripted output.
fn complete<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut cx) {
        Poll::Ready(result) => result,
        Poll::Pending => panic!("scripted output must not suspend"),
    }
}

#[test]
fn test_complete_async_string_reports_call_character_index() {
    let mut writer = AsyncCharsetTextWriter::new(IndexOutput::default(), AsciiCodec, CharsetEncodePolicy::report());
    let text = format!("{}中", "A".repeat(300));
    let error = complete(writer.write_str_fully_async(&text)).expect_err("ASCII rejects 中");
    assert_complete_main_index(&error, 300);
}

#[test]
fn test_complete_async_string_reports_call_index_across_capacities() {
    for capacity in [1, 2, 7, 8192] {
        for index in [0, 255, 256, 300, 512] {
            let text = format!("{}中", "A".repeat(index));
            let mut writer = AsyncCharsetTextWriter::new_with_buffer_capacity(
                IndexOutput::default(),
                AsciiCodec,
                CharsetEncodePolicy::report(),
                capacity,
            );
            let error = complete(writer.write_str_fully_async(&text)).expect_err("ASCII rejects 中");
            assert_complete_main_index(&error, index);
        }
    }

    let text = format!("{}中", "A".repeat(300));
    let mut writer = AsyncCharsetTextWriter::new(IndexOutput::default(), AsciiCodec, CharsetEncodePolicy::report());
    let error = complete(writer.write_str_fully_async(&text)).expect_err("ASCII rejects 中");
    assert_complete_main_index(&error, 300);
}

#[test]
fn test_complete_async_charset_writer_preserves_call_origin_for_other_entry_points() {
    let text = format!("{}中", "A".repeat(300));

    let mut chars = vec!['A'; 300];
    chars.push('中');
    let mut writer = AsyncCharsetTextWriter::new(IndexOutput::default(), AsciiCodec, CharsetEncodePolicy::report());
    let error = complete(writer.write_chars_fully_async(&chars)).expect_err("ASCII rejects 中");
    assert_complete_main_index(&error, 300);

    let mut writer = AsyncCharsetTextWriter::new(IndexOutput::default(), AsciiCodec, CharsetEncodePolicy::report());
    let error = complete(AsyncTextWriter::write_str_fully_async(&mut writer, &text))
        .expect_err("ASCII rejects 中 through the trait");
    assert_complete_main_index(&error, 300);

    let mut writer = AsyncCharsetTextWriter::new(IndexOutput::default(), AsciiCodec, CharsetEncodePolicy::report());
    complete(writer.write_str_fully_async("prefix")).expect("ASCII accepts prefix");
    let error = complete(writer.write_str_fully_async(&text)).expect_err("ASCII rejects 中");
    assert_complete_main_index(&error, 300);

    let mut writer = AsyncCharsetTextWriter::new(IndexOutput::default(), Latin1Codec, CharsetEncodePolicy::report());
    let latin1_text = format!("{}中", "é".repeat(300));
    let error = complete(writer.write_str_fully_async(&latin1_text)).expect_err("Latin-1 rejects 中");
    assert_complete_main_index(&error, 300);

    let mut writer = AsyncCharsetTextWriter::new(IndexOutput::default(), AsciiCodec, CharsetEncodePolicy::report());
    let error = complete(writer.write_line_fully_async(&text)).expect_err("ASCII rejects line body");
    assert_complete_main_index(&error, 300);
}

#[test]
fn test_async_string_single_step_error_uses_current_suffix_origin() {
    let mut writer = AsyncCharsetTextWriter::new(IndexOutput::default(), AsciiCodec, CharsetEncodePolicy::report());
    let text = format!("{}中", "A".repeat(300));
    let first = complete(writer.write_str_async(&text)).expect("first string step succeeds");
    assert_eq!(256, first);
    let error = complete(writer.write_str_async(&text[first..])).expect_err("ASCII rejects 中");
    assert_complete_main_index(&error, 44);

    let mut writer = AsyncCharsetTextWriter::new(IndexOutput::default(), AsciiCodec, CharsetEncodePolicy::report());
    let mut chars = vec!['A'; 300];
    chars.push('中');
    let first = complete(writer.write_chars_async(&chars[..256])).expect("first character step succeeds");
    assert_eq!(256, first);
    let error = complete(writer.write_chars_async(&chars[first..])).expect_err("ASCII rejects 中");
    assert_complete_main_index(&error, 44);
}

/// Checks that both complete-write trait methods reject a finalized writer.
async fn assert_finished_empty_trait<W>(writer: &mut W)
where
    W: AsyncTextWriter<Error = io::Error>,
{
    let error = writer
        .write_str_fully_async("")
        .await
        .expect_err("finished string write");
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
    let error = writer
        .write_chars_fully_async(&[])
        .await
        .expect_err("finished character write");
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_async_charset_writer_open_empty_writes_do_not_perform_io() -> io::Result<()> {
    let mut writer = AsyncCharsetTextWriter::new(RecoveryOutput::default(), Utf8Codec, CharsetEncodePolicy::report());

    complete(writer.write_str_fully_async(""))?;
    complete(writer.write_chars_fully_async(&[]))?;
    assert_eq!(0, writer.output_mut().writes);
    assert_eq!(0, writer.output_mut().flushes);
    Ok(())
}

#[test]
fn test_async_charset_writer_finished_empty_writes_are_rejected() -> io::Result<()> {
    let mut writer = AsyncCharsetTextWriter::new(RecoveryOutput::default(), Utf8Codec, CharsetEncodePolicy::report());
    complete(writer.finish_async())?;
    let writes = writer.output_mut().writes;
    let flushes = writer.output_mut().flushes;

    let error = complete(writer.write_str_async("")).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
    let error = complete(writer.write_chars_async(&[])).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
    let error = complete(writer.write_str_fully_async("")).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
    let error = complete(writer.write_chars_fully_async(&[])).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
    complete(assert_finished_empty_trait(&mut writer));

    assert_eq!(writes, writer.output_mut().writes);
    assert_eq!(flushes, writer.output_mut().flushes);
    Ok(())
}

#[test]
fn test_async_charset_writer_finished_empty_writes_after_delivery_failure() -> io::Result<()> {
    let mut writer = AsyncCharsetTextWriter::new(RecoveryOutput::default(), Utf8Codec, CharsetEncodePolicy::report());
    complete(writer.write_str_fully_async("abc"))?;
    assert_eq!(
        io::ErrorKind::BrokenPipe,
        complete(writer.finish_async()).unwrap_err().kind()
    );

    let error = complete(writer.write_str_fully_async("")).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());
    let error = complete(writer.write_chars_fully_async(&[])).unwrap_err();
    assert_eq!(io::ErrorKind::InvalidInput, error.kind());

    complete(assert_finished_empty_trait(&mut writer));

    complete(writer.finish_async())?;
    assert_eq!(b"abc", writer.output_mut().bytes.as_slice());
    Ok(())
}

#[test]
fn test_async_buffered_parts_preserves_unflushed_text_without_io() -> io::Result<()> {
    let mut writer = AsyncUtf8TextWriter::new(RecoveryOutput::default());
    complete(writer.write_str_fully_async("A中"))?;
    let (output, pending) = writer.into_inner().into_buffered_parts();
    assert_eq!(0, output.writes);
    assert_eq!(0, output.flushes);
    assert!(output.bytes.is_empty());
    assert_eq!("A中".as_bytes(), pending.readable());
    Ok(())
}

#[test]
fn test_async_buffered_parts_recovers_only_undelivered_suffix() -> io::Result<()> {
    let mut writer = AsyncCharsetTextWriter::new(RecoveryOutput::default(), Utf8Codec, CharsetEncodePolicy::report());
    complete(writer.write_str_fully_async("A中"))?;
    assert_eq!(
        io::ErrorKind::BrokenPipe,
        complete(writer.flush_async()).unwrap_err().kind()
    );
    let (output, pending) = writer.into_buffered_parts();
    assert_eq!(b"A", output.bytes.as_slice());
    assert_eq!("中".as_bytes(), pending.readable());
    assert_eq!(2, output.writes);
    assert_eq!(0, output.flushes);
    Ok(())
}

#[test]
fn test_async_buffered_parts_is_empty_after_finished_delivery() -> io::Result<()> {
    let mut writer = AsyncUtf8TextWriter::new(RecoveryOutput::default());
    complete(writer.write_str_fully_async("A中"))?;
    assert_eq!(
        io::ErrorKind::BrokenPipe,
        complete(writer.finish_async()).unwrap_err().kind()
    );
    complete(writer.finish_async())?;
    let (output, pending) = writer.into_inner().into_buffered_parts();
    assert_eq!("A中".as_bytes(), output.bytes.as_slice());
    assert_eq!(1, output.flushes);
    assert!(pending.is_empty());
    Ok(())
}

#[test]
fn test_async_buffered_parts_keeps_legacy_vec_recovery_compatible() -> io::Result<()> {
    let mut writer = AsyncUtf8TextWriter::new(RecoveryOutput::default());
    complete(writer.write_str_fully_async("A中"))?;
    let (output, pending): (RecoveryOutput, Vec<u8>) = writer.into_inner().into_parts();
    assert_eq!(0, output.writes);
    assert_eq!("A中".as_bytes(), pending.as_slice());
    Ok(())
}
