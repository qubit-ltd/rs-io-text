// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_io::AsyncInput;
use qubit_io::AsyncOutput;
use qubit_io_text::AsyncUtf8TextReader;
use qubit_io_text::LineEnding;
use qubit_io_text::TextWriter;
use qubit_io_text::Utf8TextWriter;
include!(concat!(env!("OUT_DIR"), "/user_guide_examples.rs"));

/// Supplies EOF to verify delivery of a previously retained line prefix.
struct EofAsyncInput;

impl AsyncInput for EofAsyncInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut [u8],
        _index: usize,
        _count: usize,
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(0))
    }
}

/// Supplies a fixed byte slice and then EOF to execute budget examples.
struct SliceAsyncInput {
    bytes: &'static [u8],
    position: usize,
}

impl SliceAsyncInput {
    fn new(bytes: &'static [u8]) -> Self {
        Self { bytes, position: 0 }
    }
}

impl AsyncInput for SliceAsyncInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        let read = count.min(self.bytes.len().saturating_sub(self.position));
        output[index..index + read].copy_from_slice(&self.bytes[self.position..self.position + read]);
        self.position += read;
        Poll::Ready(Ok(read))
    }
}

/// Records the output of the executable documentation examples.
#[derive(Default)]
pub(crate) struct RecordingAsyncOutput {
    bytes: Vec<u8>,
    flushed: bool,
}

impl AsyncOutput for RecordingAsyncOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        self.bytes.extend_from_slice(&input[index..index + count]);
        Poll::Ready(Ok(count))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.flushed = true;
        Poll::Ready(Ok(()))
    }
}

/// Polls examples using only the immediately ready documentation output.
fn complete<F: Future>(future: F) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    let mut future = std::pin::pin!(future);
    for _ in 0..512 {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
    }
    panic!("documentation example did not complete");
}

/// Compiles and executes the documented synchronous and asynchronous examples.
fn main() -> std::io::Result<()> {
    let mut bytes = Vec::new();
    let mut writer = Utf8TextWriter::new(&mut bytes).with_line_ending(LineEnding::CrLf);

    writer.write_line("hello")?;
    writer.write_str("中文")?;
    writer.finish()?;

    let (output, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => {
            panic!("unexpected indeterminate recovery")
        }
    };
    assert!(pending.readable().is_empty());
    assert_eq!("hello\r\n中文".as_bytes(), output.as_slice());
    user_guide_examples::readme_english::run()?;
    user_guide_examples::readme_chinese::run()?;
    user_guide_examples::guide_english::run()?;
    user_guide_examples::guide_chinese::run()?;
    user_guide_examples::reader_recovery_english::run()?;
    user_guide_examples::reader_recovery_chinese::run()?;
    user_guide_examples::memory_english::convert_memory().expect("English in-memory conversion example");
    user_guide_examples::memory_chinese::convert_memory().expect("Chinese in-memory conversion example");
    assert_eq!(
        "",
        complete(user_guide_examples::send_english::read_owned(EofAsyncInput))?
    );
    assert_eq!(
        "",
        complete(user_guide_examples::send_chinese::read_owned(EofAsyncInput))?
    );
    let output = complete(user_guide_examples::readme_async_english::run(
        RecordingAsyncOutput::default(),
    ))?;
    assert_eq!(b"hello\n", output.bytes.as_slice());
    assert!(output.flushed);
    let output = complete(user_guide_examples::readme_async_chinese::run(
        RecordingAsyncOutput::default(),
    ))?;
    assert_eq!(b"hello\n", output.bytes.as_slice());
    assert!(output.flushed);
    let output = complete(user_guide_examples::guide_async_english::run(
        RecordingAsyncOutput::default(),
    ))?;
    assert_eq!(b"subject: status\r\n", output.bytes.as_slice());
    assert!(output.flushed);
    let mut reader = AsyncUtf8TextReader::new(SliceAsyncInput::new("中文".as_bytes()));
    let mut text = String::from("seed:");
    assert_eq!(
        2,
        complete(user_guide_examples::budget_english::run(&mut reader, &mut text, 5, 6,))?
    );
    assert_eq!("seed:中文", text);
    for chinese in [false, true] {
        for (limit, succeeds) in [(6, true), (4, false)] {
            let mut reader = AsyncUtf8TextReader::new(SliceAsyncInput::new("文".as_bytes()));
            let mut text = String::from("seed:中");
            let result = if chinese {
                complete(user_guide_examples::budget_chinese::run(
                    &mut reader,
                    &mut text,
                    5,
                    limit,
                ))
            } else {
                complete(user_guide_examples::budget_english::run(
                    &mut reader,
                    &mut text,
                    5,
                    limit,
                ))
            };
            if succeeds {
                assert_eq!(1, result?);
                assert_eq!("seed:中文", text);
            } else {
                assert_eq!(
                    std::io::ErrorKind::InvalidData,
                    result.expect_err("remaining byte budget must reject the suffix").kind(),
                );
                assert_eq!("seed:中", text);
            }
        }
    }
    let mut reader = AsyncUtf8TextReader::new(SliceAsyncInput::new(b"x"));
    let mut text = String::from("z");
    let error = complete(user_guide_examples::budget_chinese::run(&mut reader, &mut text, 2, 8))
        .expect_err("an invalid message start must fail");
    assert_eq!(std::io::ErrorKind::InvalidInput, error.kind());
    assert_eq!(0, reader.input().position);
    let output = complete(user_guide_examples::guide_async_chinese::run(
        RecordingAsyncOutput::default(),
    ))?;
    assert_eq!(b"subject: status\r\n", output.bytes.as_slice());
    assert!(output.flushed);
    for prefix in ["", "tail"] {
        let expected = (!prefix.is_empty()).then(|| prefix.to_owned());
        let mut reader = AsyncUtf8TextReader::new(EofAsyncInput);
        let mut line = prefix.to_owned();
        let actual = complete(user_guide_examples::line_resume_english::resume_line(
            &mut reader,
            &mut line,
        ))?;
        assert_eq!(expected, actual, "English recovery example");
        assert!(line.is_empty(), "delivered prefix must be removed");
        assert_eq!(
            None,
            complete(user_guide_examples::line_resume_english::resume_line(
                &mut reader,
                &mut line,
            ))?
        );
    }
    for prefix in ["", "tail"] {
        let expected = (!prefix.is_empty()).then(|| prefix.to_owned());
        let mut reader = AsyncUtf8TextReader::new(EofAsyncInput);
        let mut line = prefix.to_owned();
        let actual = complete(user_guide_examples::line_resume_chinese::resume_line(
            &mut reader,
            &mut line,
        ))?;
        assert_eq!(expected, actual, "Chinese recovery example");
        assert!(line.is_empty(), "delivered prefix must be removed");
        assert_eq!(
            None,
            complete(user_guide_examples::line_resume_chinese::resume_line(
                &mut reader,
                &mut line,
            ))?
        );
    }
    Ok(())
}
