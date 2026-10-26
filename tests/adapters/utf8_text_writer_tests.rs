// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io;
use std::io::ErrorKind;
use std::io::Write;

use qubit_io::Output;
use qubit_io_text::LineEnding;
use qubit_io_text::TextWriter;
use qubit_io_text::Utf8TextWriter;

#[derive(Debug, Default)]
struct OutputOnlyWriter {
    bytes: Vec<u8>,
}

impl Output for OutputOnlyWriter {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, input: &[u8], index: usize, count: usize) -> io::Result<usize> {
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

#[derive(Default)]
struct PanicOutput {
    bytes: Vec<u8>,
}

#[derive(Debug)]
struct FailOnByteOnceOutput {
    bytes: Vec<u8>,
    fail_on: u8,
    failed: bool,
}

impl Output for FailOnByteOnceOutput {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, input: &[u8], index: usize, count: usize) -> io::Result<usize> {
        let bytes = &input[index..index + count];
        if !self.failed && bytes.contains(&self.fail_on) {
            self.failed = true;
            return Err(io::Error::new(ErrorKind::BrokenPipe, "scripted write failure"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Output for PanicOutput {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, input: &[u8], index: usize, count: usize) -> std::io::Result<usize> {
        let committed = count.min(1);
        self.bytes.extend_from_slice(&input[index..index + committed]);
        panic!("scripted output panic after committing a prefix");
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_new_accepts_qubit_output_without_std_write() -> std::io::Result<()> {
    let mut writer = Utf8TextWriter::new(OutputOnlyWriter::default());

    writer.write_line("output中文")?;
    writer.finish()?;
    let (output, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!("output中文\n".as_bytes(), output.bytes.as_slice());
    Ok(())
}

#[test]
fn test_write_utf8_text_to_byte_writer() -> std::io::Result<()> {
    let mut output = Vec::new();
    {
        let mut writer = Utf8TextWriter::new(&mut output).with_line_ending(LineEnding::CrLf);

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
    let output = b"prefix:".to_vec();
    let mut writer = Utf8TextWriter::with_capacity(output, 1);

    assert_eq!(b"prefix:", writer.output().as_slice());
    assert_eq!(LineEnding::Lf, writer.line_ending());
    writer.write_line("done")?;
    writer.finish()?;

    let (output, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };
    assert!(pending.readable().is_empty());
    assert_eq!(b"prefix:done\n", output.as_slice());
    Ok(())
}

#[test]
fn test_finish_then_into_parts_returns_output() -> std::io::Result<()> {
    let mut writer = Utf8TextWriter::new(Vec::new());
    writer.write_str("recoverable")?;

    writer.finish()?;
    let (output, pending) = writer.into_parts();
    let pending = match pending {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => panic!("unexpected indeterminate recovery"),
    };

    assert!(pending.readable().is_empty());
    assert_eq!(b"recoverable", output.as_slice());
    Ok(())
}

#[test]
fn test_finish_error_leaves_utf8_writer_available_for_retry() {
    let mut writer = Utf8TextWriter::new(FailingWriter);

    let error = writer.finish().expect_err("finish should report write failure");

    assert_eq!(ErrorKind::Other, error.kind());
    let _ = writer.output();
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

#[test]
fn test_into_parts_preserves_indeterminate_after_inner_write_panics() {
    let mut writer = Utf8TextWriter::new(PanicOutput::default());
    writer.write_str("text").expect("the encoded value should be buffered");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| TextWriter::flush(&mut writer)));
    assert!(result.is_err());

    let (inner, recovery) = writer.into_parts();
    assert_eq!(b"t", inner.bytes.as_slice());
    assert!(matches!(recovery, qubit_io::BufferedOutputRecovery::Indeterminate(_)));
}

#[test]
fn test_write_line_endings_empty_lines_and_long_mixed_text() -> io::Result<()> {
    let long_line = "中文🙂é".repeat(100);
    assert!(long_line.chars().count() > 256);
    let expected = format!("\n\r\n\r{long_line}首🙂尾\n");
    let mut writer = Utf8TextWriter::with_capacity(Vec::new(), 8)
        .with_line_ending(LineEnding::Lf);
    writer.write_line("")?;
    writer = writer.with_line_ending(LineEnding::CrLf);
    writer.write_line("")?;
    writer = writer.with_line_ending(LineEnding::Cr);
    writer.write_line("")?;
    writer = writer.with_line_ending(LineEnding::Lf);
    writer.write_str(&long_line)?;
    writer.write_char('首')?;
    writer.write_str("🙂")?;
    writer.write_line("尾")?;
    writer.finish()?;

    let (output, recovery) = writer.into_parts();
    let qubit_io::BufferedOutputRecovery::Pending(pending) = recovery else {
        panic!("unexpected indeterminate recovery")
    };
    assert!(pending.readable().is_empty());
    assert_eq!(expected.as_bytes(), output.as_slice());
    Ok(())
}

#[test]
fn test_write_line_after_finish_rejects_empty_and_non_empty_lines() -> io::Result<()> {
    let mut writer = Utf8TextWriter::new(Vec::new());
    writer.write_str("done")?;
    writer.finish()?;

    assert_eq!(ErrorKind::InvalidInput, writer.write_line("").unwrap_err().kind());
    assert_eq!(ErrorKind::InvalidInput, writer.write_line("later").unwrap_err().kind());
    Ok(())
}

#[test]
fn test_write_line_body_error_does_not_append_line_ending() {
    let output = FailOnByteOnceOutput { bytes: Vec::new(), fail_on: b'!', failed: false };
    let mut writer = Utf8TextWriter::with_capacity(output, 1);
    let body = "!".repeat(8 * 1024 + 1);
    let error = writer.write_line(&body).expect_err("body delivery should fail");
    assert_eq!(ErrorKind::BrokenPipe, error.kind());
    let (output, recovery) = writer.into_parts();
    let qubit_io::BufferedOutputRecovery::Pending(pending) = recovery else {
        panic!("unexpected indeterminate recovery")
    };
    let mut recovered = output.bytes;
    recovered.extend_from_slice(pending.readable());
    assert!(!recovered.contains(&b'\n'));
    assert!(recovered.iter().all(|byte| *byte == b'!'));
}

#[test]
fn test_write_line_ending_flush_error_keeps_body_bytes() {
    let output = FailOnByteOnceOutput { bytes: Vec::new(), fail_on: b'\n', failed: false };
    let mut writer = Utf8TextWriter::with_capacity(output, 1);
    let body = "b".repeat(8 * 1024);
    writer.write_line(&body).expect("body should be buffered successfully");
    let error = writer.flush().expect_err("line-ending delivery should fail");
    assert_eq!(ErrorKind::BrokenPipe, error.kind());
    let (output, recovery) = writer.into_parts();
    let qubit_io::BufferedOutputRecovery::Pending(pending) = recovery else {
        panic!("unexpected indeterminate recovery")
    };
    let mut recovered = output.bytes;
    recovered.extend_from_slice(pending.readable());
    assert_eq!(format!("{body}\n").as_bytes(), recovered.as_slice());
}
