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

use qubit_codec_text::CharsetDecodePolicy;
use qubit_io::AsyncInput;
use qubit_io_text::{
    AsyncUtf8TextReader,
    LineEnding,
    LineEndingSet,
};

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
        output[output_index..output_end]
            .copy_from_slice(&self.bytes[self.position..input_end]);
        self.position = input_end;
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

#[test]
fn test_async_utf8_text_reader_decodes_text_and_exposes_inner_reader()
-> io::Result<()> {
    let mut reader = AsyncUtf8TextReader::with_capacity(
        ReadyInput::new("A中"),
        CharsetDecodePolicy::report(),
        1,
    );
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
