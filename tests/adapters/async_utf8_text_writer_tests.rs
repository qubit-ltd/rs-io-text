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
    pin::Pin,
    task::{
        Context,
        Poll,
        Waker,
    },
};

use qubit_io::AsyncOutput;
use qubit_io_text::{
    AsyncUtf8TextWriter,
    LineEnding,
};

/// Collects written bytes without suspending.
#[derive(Default)]
struct ReadyOutput(Vec<u8>);

impl AsyncOutput for ReadyOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        input: &[u8],
        input_index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        let input_end = input_index + count;
        self.0.extend_from_slice(&input[input_index..input_end]);
        Poll::Ready(Ok(count))
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
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
        Poll::Pending => panic!("ready output must not suspend"),
    }
}

#[test]
fn test_async_utf8_text_writer_encodes_text_and_exposes_inner_writer()
-> io::Result<()> {
    let mut writer =
        AsyncUtf8TextWriter::with_capacity(ReadyOutput::default(), 1);

    complete(writer.write_str_fully_async("A中"))?;
    complete(writer.finish_async())?;

    let (output, pending) = writer.into_inner().into_parts();
    assert!(pending.is_empty());
    assert_eq!("A中".as_bytes(), output.0.as_slice());
    Ok(())
}

#[test]
fn test_async_utf8_text_writer_configuration_and_deref_accessors() {
    let mut writer = AsyncUtf8TextWriter::new(ReadyOutput::default());
    assert!(writer.output().0.is_empty());
    writer.output_mut().0.push(b'!');
    assert_eq!(vec![b'!'], writer.output().0);
    let writer = writer.into_inner();
    assert_eq!(vec![b'!'], writer.output().0);
}

#[test]
fn test_async_utf8_text_writer_configures_line_ending() {
    let writer = AsyncUtf8TextWriter::new(ReadyOutput::default())
        .with_line_ending(LineEnding::CrLf);
    assert_eq!(LineEnding::CrLf, writer.configured_line_ending());
}
