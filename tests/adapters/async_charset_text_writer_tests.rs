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

use qubit_codec_text::CharsetEncodePolicy;
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
