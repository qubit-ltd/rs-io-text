// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::Utf8Codec;
use qubit_io::AsyncByteOutput;
use qubit_io_text::AsyncCharsetTextWriter;
use qubit_io_text::LineEnding;
use qubit_io_text::TextWriter;
use qubit_io_text::Utf8TextWriter;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

struct CompileOnlyOutput;

impl qubit_io::AsyncOutput for CompileOnlyOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _input: &[Self::Item],
        _index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(count))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

async fn write_message<O>(output: O) -> std::io::Result<O>
where
    O: AsyncByteOutput + Unpin,
{
    let mut writer = AsyncCharsetTextWriter::new(output, Utf8Codec, CharsetEncodePolicy::report());
    writer.write_line_fully_async("hello").await?;
    writer.finish_async().await?;
    let (output, pending) = writer.into_parts();
    assert!(pending.is_empty());
    Ok(output)
}

/// Compiles the documented synchronous and asynchronous usage examples.
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
    drop(write_message(CompileOnlyOutput));
    Ok(())
}
