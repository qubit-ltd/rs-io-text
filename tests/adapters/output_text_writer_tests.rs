// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Error;
use std::io::ErrorKind;
use std::vec::IntoIter;

use qubit_io::BufferedOutput;
use qubit_io::CharOutput;
use qubit_io::Output;
use qubit_io_text::LineEnding;
use qubit_io_text::OutputTextWriter;
use qubit_io_text::StringCharOutput;
use qubit_io_text::TextWriter;

#[derive(Debug)]
struct FailingCharOutput;

impl Output for FailingCharOutput {
    type Item = char;

    unsafe fn write_unchecked(&mut self, _input: &[char], _index: usize, _count: usize) -> std::io::Result<usize> {
        Err(Error::other("write failed"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Injects short writes and errors without an automatic buffering wrapper.
#[derive(Debug)]
struct ScriptedCharOutput<'a> {
    accepted: &'a mut String,
    requests: &'a mut Vec<String>,
    actions: IntoIter<Option<usize>>,
}

impl Output for ScriptedCharOutput<'_> {
    type Item = char;

    fn is_buffered(&self) -> bool {
        true
    }

    unsafe fn write_unchecked(&mut self, input: &[char], index: usize, count: usize) -> std::io::Result<usize> {
        let requested = &input[index..index + count];
        self.requests.push(requested.iter().copied().collect());
        match self.actions.next().expect("unexpected extra output request") {
            Some(accepted) => {
                assert!(accepted > 0 && accepted <= count, "invalid scripted progress");
                self.accepted.extend(requested[..accepted].iter().copied());
                Ok(accepted)
            }
            None => Err(Error::other("scripted write failure")),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn assert_other_error<T>(result: std::io::Result<T>)
where
    T: std::fmt::Debug,
{
    let error = result.expect_err("operation should propagate output error");
    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_write_text_to_char_output() -> std::io::Result<()> {
    let mut text = String::new();
    {
        let output = StringCharOutput::new(&mut text);
        let mut writer = OutputTextWriter::new(output).with_line_ending(LineEnding::CrLf);

        writer.write_char('中')?;
        writer.write_chars(&['a', '🙂'])?;
        writer.write_str("raw")?;
        writer.write_line("done")?;
        writer.flush()?;
    }

    assert_eq!("中a🙂rawdone\r\n", text);
    Ok(())
}

#[test]
fn test_accessors_expose_buffered_output() -> std::io::Result<()> {
    let mut text = String::from("prefix");
    {
        let output = StringCharOutput::new(&mut text);
        let mut writer = OutputTextWriter::new(output);

        assert!(writer.get_ref().is_buffered());
        writer.get_mut().write_fully(&[':'])?;
        writer.write_str("value")?;
        assert_eq!(LineEnding::Lf, writer.line_ending());
    }

    assert_eq!("prefix:value", text);
    Ok(())
}

#[test]
fn test_new_accepts_already_buffered_output() -> std::io::Result<()> {
    let mut text = String::new();
    {
        let output = BufferedOutput::new(StringCharOutput::new(&mut text));
        let mut writer = OutputTextWriter::new(output);
        let debug = format!("{writer:?}");

        assert!(debug.contains("OutputTextWriter"));
        assert!(debug.contains("is_buffered"));
        writer.write_line("buffered")?;
        writer.flush()?;
    }

    assert_eq!("buffered\n", text);
    Ok(())
}

#[test]
fn test_from_boxed_wraps_unbuffered_output() -> std::io::Result<()> {
    let mut text = String::new();
    {
        let output: Box<dyn CharOutput + '_> = Box::new(StringCharOutput::new(&mut text));
        let mut writer = OutputTextWriter::from_boxed(output);

        assert!(writer.get_ref().is_buffered());
        writer.write_chars(&['b', 'o', 'x'])?;
        writer.flush()?;
    }

    assert_eq!("box", text);
    Ok(())
}

#[test]
fn test_from_boxed_keeps_buffered_output() -> std::io::Result<()> {
    let mut text = String::new();
    {
        let output: Box<dyn CharOutput + '_> = Box::new(BufferedOutput::new(StringCharOutput::new(&mut text)));
        let mut writer = OutputTextWriter::from_boxed(output).with_line_ending(LineEnding::Cr);

        assert!(writer.get_ref().is_buffered());
        writer.write_line("box")?;
        writer.flush()?;
        let output = writer.into_inner();
        drop(output);
    }

    assert_eq!("box\r", text);
    Ok(())
}

#[test]
fn test_into_inner_does_not_flush_wrapped_output() {
    let mut writer = OutputTextWriter::new(FailingCharOutput);

    writer
        .write_str("pending")
        .expect("buffered write may succeed before flush");
    let output = writer.into_inner();
    assert!(output.is_buffered());
}

#[test]
fn test_flush_then_into_inner_flushes_wrapped_output() -> std::io::Result<()> {
    let mut text = String::new();
    {
        let output = StringCharOutput::new(&mut text);
        let mut writer = OutputTextWriter::new(output);

        writer.write_str("value")?;
        writer.flush()?;
        let output = writer.into_inner();
        drop(output);
    }

    assert_eq!("value", text);
    Ok(())
}

#[test]
fn test_write_str_empty_does_not_write() -> std::io::Result<()> {
    let mut writer = OutputTextWriter::new(FailingCharOutput);

    writer.write_str("")?;
    Ok(())
}

#[test]
fn test_write_methods_propagate_output_errors() {
    let mut writer = OutputTextWriter::new(FailingCharOutput);
    writer
        .write_str(&"a".repeat(256))
        .expect("buffered write may succeed before flush");
    assert_other_error(writer.flush());

    let mut writer = OutputTextWriter::new(FailingCharOutput);
    writer
        .write_line("x")
        .expect("buffered line write may succeed before flush");
    assert_other_error(writer.flush());

    let mut writer = OutputTextWriter::new(FailingCharOutput);
    writer
        .write_line("")
        .expect("buffered empty line write may succeed before flush");
    assert_other_error(writer.flush());
}

#[test]
fn test_write_str_flushes_full_character_chunks() -> std::io::Result<()> {
    let long_text = "a".repeat(300);
    let mut text = String::new();
    {
        let output = StringCharOutput::new(&mut text);
        let mut writer = OutputTextWriter::new(output);

        writer.write_str(&long_text)?;
    }

    assert_eq!(long_text, text);
    Ok(())
}

#[test]
fn test_write_str_preserves_unicode_prefix_on_chunk_error() {
    let text = "中🙂a".repeat(100);
    let chars: Vec<char> = text.chars().collect();
    let mut accepted = String::new();
    let mut requests = Vec::new();
    {
        let output = ScriptedCharOutput {
            accepted: &mut accepted,
            requests: &mut requests,
            actions: vec![Some(100), Some(156), Some(7), None].into_iter(),
        };
        let mut writer = OutputTextWriter::new(output);
        let error = writer.write_str(&text).expect_err("second chunk must fail");
        assert_eq!(ErrorKind::Other, error.kind());
        assert_eq!("scripted write failure", error.to_string());
    }
    assert_eq!(chars[..263].iter().copied().collect::<String>(), accepted);
    let expected: Vec<String> = [0..256, 100..256, 256..300, 263..300]
        .into_iter()
        .map(|range| chars[range].iter().copied().collect())
        .collect();
    assert_eq!(expected, requests, "short writes must advance character offsets");
}

#[test]
fn test_write_line_does_not_append_ending_after_body_error() {
    let text = "中🙂a".repeat(100);
    let chars: Vec<char> = text.chars().collect();
    let mut accepted = String::new();
    let mut requests = Vec::new();
    {
        let output = ScriptedCharOutput {
            accepted: &mut accepted,
            requests: &mut requests,
            actions: vec![Some(100), None].into_iter(),
        };
        let mut writer = OutputTextWriter::new(output).with_line_ending(LineEnding::CrLf);
        let error = writer.write_line(&text).expect_err("body must fail before CRLF");
        assert_eq!(ErrorKind::Other, error.kind());
        assert_eq!("scripted write failure", error.to_string());
    }
    assert_eq!(chars[..100].iter().copied().collect::<String>(), accepted);
    let expected: Vec<String> = [0..256, 100..256]
        .into_iter()
        .map(|range| chars[range].iter().copied().collect())
        .collect();
    assert_eq!(expected, requests, "body failure must stop before the ending");
}

#[test]
fn test_write_line_preserves_body_and_partial_crlf_on_ending_error() {
    let text = "中🙂body";
    let mut accepted = String::new();
    let mut requests = Vec::new();
    {
        let output = ScriptedCharOutput {
            accepted: &mut accepted,
            requests: &mut requests,
            actions: vec![Some(text.chars().count()), Some(1), None].into_iter(),
        };
        let mut writer = OutputTextWriter::new(output).with_line_ending(LineEnding::CrLf);
        let error = writer.write_line(text).expect_err("LF must fail after accepted CR");
        assert_eq!(ErrorKind::Other, error.kind());
        assert_eq!("scripted write failure", error.to_string());
    }
    assert_eq!(format!("{text}\r"), accepted);
    assert_eq!(vec![text.to_owned(), "\r\n".to_owned(), "\n".to_owned()], requests);
}
