// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{
    self,
    ErrorKind,
    Write,
};

use qubit_io::Output;
use qubit_io_text::{
    LineEnding,
    TextWrite,
    Utf8TextWriter,
};

#[derive(Debug, Default)]
struct OutputOnlyWriter {
    bytes: Vec<u8>,
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

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("write failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("flush failed"))
    }
}

#[test]
fn test_new_accepts_qubit_output_without_std_write() -> std::io::Result<()> {
    let mut writer = Utf8TextWriter::new(OutputOnlyWriter::default());

    writer.write_line("output中文")?;
    writer.finish()?;
    let output = writer.into_output().map_err(|error| error.into_error())?;

    assert_eq!("output中文\n".as_bytes(), output.bytes.as_slice());
    Ok(())
}

#[test]
fn test_write_utf8_text_to_byte_writer() -> std::io::Result<()> {
    let mut output = Vec::new();
    {
        let mut writer =
            Utf8TextWriter::new(&mut output).with_line_ending(LineEnding::CrLf);

        writer.write_char('中')?;
        writer.write_chars(&['x', 'y'])?;
        writer.write_str("abc")?;
        writer.write_line("done")?;
        writer.flush()?;
    }

    assert_eq!("中xyabcdone\r\n".as_bytes(), output.as_slice());
    Ok(())
}

#[test]
fn test_accessors_and_into_inner() -> std::io::Result<()> {
    let output = Vec::new();
    let mut writer = Utf8TextWriter::with_capacity(output, 1);

    assert!(writer.output().is_empty());
    writer.output_mut().extend_from_slice(b"prefix:");
    assert_eq!(LineEnding::Lf, writer.line_ending());
    writer.write_line("done")?;
    writer.finish()?;

    let output = writer.into_output().map_err(|error| error.into_error())?;
    assert_eq!(b"prefix:done\n", output.as_slice());
    Ok(())
}

#[test]
fn test_try_into_output_finishes_and_returns_output() -> std::io::Result<()> {
    let mut writer = Utf8TextWriter::new(Vec::new());
    writer.write_str("recoverable")?;

    let output = writer
        .try_into_output()
        .map_err(|error| error.into_error())?;

    assert_eq!(b"recoverable", output.as_slice());
    Ok(())
}

#[test]
fn test_into_output_returns_utf8_writer_after_failure() {
    let writer = Utf8TextWriter::new(FailingWriter);

    let error = match writer.into_output() {
        Ok(_) => {
            panic!("recoverable conversion should retain the UTF-8 writer")
        }
        Err(error) => error,
    };

    assert_eq!(ErrorKind::Other, error.error().kind());
    let _ = error.writer().output();
}

#[test]
fn test_write_methods_propagate_underlying_errors() {
    let mut writer = Utf8TextWriter::with_capacity(FailingWriter, 1);

    let error = match writer.write_str("buffered UTF-8") {
        Ok(()) => writer.finish().expect_err("finish must fail"),
        Err(error) => error,
    };
    assert_eq!(ErrorKind::Other, error.kind());
}
