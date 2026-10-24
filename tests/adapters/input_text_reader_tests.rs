// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Error;
use std::io::ErrorKind;

use qubit_io::BufferedInput;
use qubit_io::CharInput;
use qubit_io::Input;
use qubit_io_text::InputTextReader;
use qubit_io_text::LineEnding;
use qubit_io_text::LineEndingSet;
use qubit_io_text::StringCharInput;
use qubit_io_text::TextLineReader;
use qubit_io_text::TextReader;

#[derive(Debug)]
struct FailingCharInput;

impl Input for FailingCharInput {
    type Item = char;

    unsafe fn read_unchecked(&mut self, _output: &mut [char], _index: usize, _count: usize) -> std::io::Result<usize> {
        Err(Error::other("read failed"))
    }
}

#[derive(Debug)]
struct CrThenErrorThenChar {
    state: u8,
}

impl Input for CrThenErrorThenChar {
    type Item = char;

    unsafe fn read_unchecked(&mut self, output: &mut [char], index: usize, _count: usize) -> std::io::Result<usize> {
        match self.state {
            0 => {
                output[index] = '\r';
                self.state = 1;
                Ok(1)
            }
            1 => {
                self.state = 2;
                Err(Error::other("transient read failure"))
            }
            2 => {
                output[index] = 'X';
                self.state = 3;
                Ok(1)
            }
            _ => Ok(0),
        }
    }
}

#[derive(Debug)]
struct ChunkedCharInput {
    chars: Vec<char>,
    position: usize,
    max_chunk: usize,
}

#[derive(Debug)]
struct ShortThenErrorInput {
    position: usize,
    failed: bool,
    first: char,
}

#[derive(Debug)]
struct PendingThenErrorInput {
    state: u8,
}

#[derive(Debug)]
struct InterruptedThenShortInput {
    interrupted: bool,
    inner: ShortThenErrorInput,
}

impl Input for InterruptedThenShortInput {
    type Item = char;

    unsafe fn read_unchecked(&mut self, output: &mut [char], index: usize, count: usize) -> std::io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(Error::from(ErrorKind::Interrupted));
        }
        unsafe { self.inner.read_unchecked(output, index, count) }
    }
}

impl Input for ShortThenErrorInput {
    type Item = char;

    unsafe fn read_unchecked(&mut self, output: &mut [char], index: usize, count: usize) -> std::io::Result<usize> {
        if count == 0 {
            return Ok(0);
        }
        if self.position >= 2 && !self.failed {
            self.failed = true;
            return Err(Error::other("error after confirmed prefix"));
        }
        let chars = [self.first, '\n', 'z'];
        if self.position == chars.len() {
            return Ok(0);
        }
        output[index] = chars[self.position];
        self.position += 1;
        Ok(1)
    }
}

impl Input for PendingThenErrorInput {
    type Item = char;

    unsafe fn read_unchecked(&mut self, output: &mut [char], index: usize, count: usize) -> std::io::Result<usize> {
        if count == 0 {
            return Ok(0);
        }
        match self.state {
            0 => {
                output[index..index + 3].copy_from_slice(&['a', '\n', 'x']);
                self.state = 1;
                Ok(3)
            }
            1 => {
                self.state = 2;
                Err(Error::other("error after pending character"))
            }
            2 => {
                output[index] = 'z';
                self.state = 3;
                Ok(1)
            }
            _ => Ok(0),
        }
    }
}

impl ChunkedCharInput {
    fn new(text: &str, max_chunk: usize) -> Self {
        Self {
            chars: text.chars().collect(),
            position: 0,
            max_chunk,
        }
    }
}

impl Input for ChunkedCharInput {
    type Item = char;

    unsafe fn read_unchecked(&mut self, output: &mut [char], index: usize, count: usize) -> std::io::Result<usize> {
        let available = self.chars.len().saturating_sub(self.position);
        let read = available.min(count).min(self.max_chunk);
        output[index..index + read].copy_from_slice(&self.chars[self.position..self.position + read]);
        self.position += read;
        Ok(read)
    }
}

#[test]
fn test_read_line_accepts_crlf_and_cr_by_default() -> std::io::Result<()> {
    let input = StringCharInput::new("first\r\nsecond\rthird".to_owned());
    let mut reader = InputTextReader::new(input);
    let mut line = String::new();

    assert_eq!(LineEndingSet::ALL, reader.line_endings());
    assert!(reader.read_line(&mut line)?);
    assert_eq!("first\r\n", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("second\r", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("third", line);
    Ok(())
}

#[test]
fn test_read_line_can_restrict_accepted_endings() -> std::io::Result<()> {
    let input = StringCharInput::new("first\rsecond\nthird".to_owned());
    let mut reader = InputTextReader::new(input).with_line_endings(LineEndingSet::only(LineEnding::Lf));
    let mut line = String::new();

    assert_eq!(LineEndingSet::LF, reader.line_endings());
    assert!(reader.read_line(&mut line)?);
    assert_eq!("first\rsecond\n", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("third", line);
    Ok(())
}

#[test]
fn test_read_line_handles_chunked_crlf_and_cr_paths() -> std::io::Result<()> {
    let mut reader = InputTextReader::new(ChunkedCharInput::new("first\r\nnext", 1));
    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("first\r\n", line);

    let mut reader = InputTextReader::new(ChunkedCharInput::new("first\rnext", 1));
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("first\r", line);

    let mut reader = InputTextReader::new(ChunkedCharInput::new("tail\r", 1)).with_line_endings(LineEndingSet::CRLF);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("tail\r", line);
    Ok(())
}

#[test]
fn test_read_line_preserves_cr_when_crlf_lookahead_fails() -> std::io::Result<()> {
    let mut reader = InputTextReader::new(CrThenErrorThenChar { state: 0 });
    let mut line = String::new();

    let error = reader.read_line(&mut line).expect_err("lookahead fails");
    assert_eq!(ErrorKind::Other, error.kind());
    assert!(line.is_empty());

    assert!(reader.read_line(&mut line)?);
    assert_eq!("\r", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("X", line);
    Ok(())
}

#[test]
fn test_short_line_returns_before_next_source_error() -> std::io::Result<()> {
    let input = ShortThenErrorInput {
        position: 0,
        failed: false,
        first: 'a',
    };
    let mut reader = InputTextReader::new(input);
    let mut output = String::new();
    assert!(reader.read_line(&mut output)?);
    assert_eq!("a\n", output);
    assert_eq!(
        ErrorKind::Other,
        reader.read_line(&mut String::new()).unwrap_err().kind()
    );
    assert_eq!(Some('z'), reader.read_char()?);
    Ok(())
}

#[test]
fn test_bulk_read_retains_confirmed_prefix_on_error() -> std::io::Result<()> {
    let input = ShortThenErrorInput {
        position: 0,
        failed: false,
        first: 'a',
    };
    let mut reader = InputTextReader::new(input);
    let mut output = String::from("seed:");
    assert_eq!(ErrorKind::Other, reader.read_to_string(&mut output).unwrap_err().kind());
    assert_eq!("seed:a\n", output);
    assert_eq!(1, reader.read_to_string(&mut output)?);
    assert_eq!("seed:a\nz", output);
    Ok(())
}

#[test]
fn test_read_chars_retains_confirmed_prefix_on_error() -> std::io::Result<()> {
    let input = ShortThenErrorInput {
        position: 0,
        failed: false,
        first: 'a',
    };
    let mut reader = InputTextReader::new(input);
    let mut output = vec!['s'];
    assert_eq!(ErrorKind::Other, reader.read_chars(&mut output, 8).unwrap_err().kind());
    assert_eq!(vec!['s', 'a', '\n'], output);
    assert_eq!(1, reader.read_chars(&mut output, 8)?);
    assert_eq!(vec!['s', 'a', '\n', 'z'], output);
    Ok(())
}

#[test]
fn test_read_chars_counts_unicode_characters_on_short_reads() -> std::io::Result<()> {
    let input = ShortThenErrorInput {
        position: 0,
        failed: false,
        first: '中',
    };
    let mut reader = InputTextReader::new(input);
    let mut output = Vec::new();
    assert_eq!(2, reader.read_chars(&mut output, 2)?);
    assert_eq!(vec!['中', '\n'], output);
    assert_eq!(ErrorKind::Other, reader.read_chars(&mut output, 8).unwrap_err().kind());
    assert_eq!(1, reader.read_chars(&mut output, 8)?);
    assert_eq!(vec!['中', '\n', 'z'], output);
    Ok(())
}

#[test]
fn test_read_chars_preserves_pending_prefix_when_source_errors() -> std::io::Result<()> {
    let mut reader = InputTextReader::new(PendingThenErrorInput { state: 0 });
    let mut line = String::new();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("a\n", line);

    let mut chars = vec!['s'];
    assert_eq!(ErrorKind::Other, reader.read_chars(&mut chars, 8).unwrap_err().kind());
    assert_eq!(vec!['s', 'x'], chars);
    assert_eq!(1, reader.read_chars(&mut chars, 8)?);
    assert_eq!(vec!['s', 'x', 'z'], chars);
    Ok(())
}

#[test]
fn test_short_line_retries_interrupted_source_read() -> std::io::Result<()> {
    let input = InterruptedThenShortInput {
        interrupted: false,
        inner: ShortThenErrorInput {
            position: 0,
            failed: false,
            first: 'a',
        },
    };
    let mut reader = InputTextReader::new(input);
    let mut output = String::new();
    assert!(reader.read_line(&mut output)?);
    assert_eq!("a\n", output);
    Ok(())
}

fn assert_other_error<T>(result: std::io::Result<T>)
where
    T: std::fmt::Debug,
{
    let error = result.expect_err("operation should propagate input error");
    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_read_char_reads_from_char_input() -> std::io::Result<()> {
    let input = StringCharInput::new("a中".to_owned());
    let mut reader = InputTextReader::new(input);

    assert_eq!(Some('a'), reader.read_char()?);
    assert_eq!(Some('中'), reader.read_char()?);
    assert_eq!(None, reader.read_char()?);
    Ok(())
}

#[test]
fn test_accessors_expose_buffered_input() -> std::io::Result<()> {
    let input = StringCharInput::new("abc".to_owned());
    let mut reader = InputTextReader::new(input);

    assert!(reader.get_ref().is_buffered());
    assert!(reader.get_mut().is_buffered());
    assert_eq!(Some('a'), reader.read_char()?);
    assert!(reader.into_inner().is_buffered());
    Ok(())
}

#[test]
fn test_new_accepts_already_buffered_input() -> std::io::Result<()> {
    let input = BufferedInput::new(StringCharInput::new("xy".to_owned()));
    let mut reader = InputTextReader::new(input);

    assert!(reader.get_ref().is_buffered());
    assert_eq!(Some('x'), reader.read_char()?);
    assert_eq!(Some('y'), reader.read_char()?);
    assert_eq!(None, reader.read_char()?);
    Ok(())
}

#[test]
fn test_from_boxed_wraps_unbuffered_input() -> std::io::Result<()> {
    let input: Box<dyn CharInput> = Box::new(StringCharInput::new("boxed".to_owned()));
    let mut reader = InputTextReader::from_boxed(input);
    let mut output = String::new();

    assert!(reader.get_ref().is_buffered());
    assert_eq!(5, reader.read_to_string(&mut output)?);
    assert_eq!("boxed", output);
    Ok(())
}

#[test]
fn test_from_boxed_keeps_buffered_input() -> std::io::Result<()> {
    let input: Box<dyn CharInput> = Box::new(BufferedInput::new(StringCharInput::new("buf".to_owned())));
    let mut reader = InputTextReader::from_boxed(input);
    let debug = format!("{reader:?}");

    assert!(debug.contains("InputTextReader"));
    assert!(debug.contains("is_buffered"));
    assert_eq!(Some('b'), reader.read_char()?);
    assert!(reader.into_inner().is_buffered());
    Ok(())
}

#[test]
fn test_read_chars_appends_up_to_max() -> std::io::Result<()> {
    let input = StringCharInput::new("ab中".to_owned());
    let mut reader = InputTextReader::new(input);
    let mut output = vec!['x'];

    assert_eq!(2, reader.read_chars(&mut output, 2)?);
    assert_eq!(vec!['x', 'a', 'b'], output);
    assert_eq!(1, reader.read_chars(&mut output, 8)?);
    assert_eq!(vec!['x', 'a', 'b', '中'], output);
    Ok(())
}

#[test]
fn test_read_chars_reads_across_internal_chunks() -> std::io::Result<()> {
    let input = StringCharInput::new("a".repeat(300));
    let mut reader = InputTextReader::new(input);
    let mut output = Vec::new();

    assert_eq!(300, reader.read_chars(&mut output, 300)?);
    assert_eq!(300, output.len());
    assert!(output.iter().all(|ch| *ch == 'a'));
    Ok(())
}

#[test]
fn test_read_to_string_appends_remaining_chars() -> std::io::Result<()> {
    let input = StringCharInput::new("a中".to_owned());
    let mut reader = InputTextReader::new(input);
    let mut output = String::from("seed:");

    assert_eq!(2, reader.read_to_string(&mut output)?);
    assert_eq!("seed:a中", output);
    assert_eq!(0, reader.read_to_string(&mut output)?);
    Ok(())
}

#[test]
fn test_read_to_string_drains_pending_chars() -> std::io::Result<()> {
    let input = StringCharInput::new("a\nbc".to_owned());
    let mut reader = InputTextReader::new(input);
    let mut line = String::new();
    let mut rest = String::from("tail:");

    assert!(reader.read_line(&mut line)?);
    assert_eq!("a\n", line);
    assert_eq!(Some('b'), reader.read_char()?);
    assert_eq!(1, reader.read_to_string(&mut rest)?);
    assert_eq!("tail:c", rest);
    Ok(())
}

#[test]
fn test_read_chars_drains_pending_chars() -> std::io::Result<()> {
    let input = StringCharInput::new("a\nbc".to_owned());
    let mut reader = InputTextReader::new(input);
    let mut line = String::new();
    let mut chars = Vec::new();

    assert!(reader.read_line(&mut line)?);
    assert_eq!("a\n", line);
    assert_eq!(1, reader.read_chars(&mut chars, 1)?);
    assert_eq!(vec!['b'], chars);
    assert_eq!(Some('c'), reader.read_char()?);
    Ok(())
}

#[test]
fn test_read_chars_with_zero_max_does_not_read() -> std::io::Result<()> {
    let mut reader = InputTextReader::new(FailingCharInput);
    let mut output = vec!['x'];

    assert_eq!(0, reader.read_chars(&mut output, 0)?);
    assert_eq!(vec!['x'], output);
    Ok(())
}

#[test]
fn test_read_methods_propagate_input_errors() {
    assert_other_error(InputTextReader::new(FailingCharInput).read_char());

    let mut chars = vec!['x'];
    assert_other_error(InputTextReader::new(FailingCharInput).read_chars(&mut chars, 1));
    assert_eq!(vec!['x'], chars);

    let mut text = String::from("seed:");
    assert_other_error(InputTextReader::new(FailingCharInput).read_to_string(&mut text));
    assert_eq!("seed:", text);

    let mut line = String::from("seed:");
    assert_other_error(InputTextReader::new(FailingCharInput).read_line(&mut line));
    assert_eq!("seed:", line);
}

#[test]
fn test_read_line_keeps_line_ending() -> std::io::Result<()> {
    let input = StringCharInput::new("alpha\nβeta".to_owned());
    let mut reader = InputTextReader::new(input);
    let mut line = String::new();

    assert!(reader.read_line(&mut line)?);
    assert_eq!("alpha\n", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("βeta", line);
    line.clear();
    assert!(!reader.read_line(&mut line)?);
    Ok(())
}

#[test]
fn test_read_line_without_newline_reads_direct_chunk() -> std::io::Result<()> {
    let input = StringCharInput::new("plain".to_owned());
    let mut reader = InputTextReader::new(input);
    let mut line = String::from("seed:");

    assert!(reader.read_line(&mut line)?);
    assert_eq!("seed:plain", line);
    line.clear();
    assert!(!reader.read_line(&mut line)?);
    Ok(())
}

#[test]
fn test_read_line_preserves_batched_tail_for_next_read() -> std::io::Result<()> {
    let input = StringCharInput::new("a\nb\nc".to_owned());
    let mut reader = InputTextReader::new(input);
    let mut line = String::new();

    assert!(reader.read_line(&mut line)?);
    assert_eq!("a\n", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("b\n", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("c", line);
    Ok(())
}

#[test]
fn test_into_parts_preserves_batched_line_tail() -> std::io::Result<()> {
    let input = StringCharInput::new("a\nbc".to_owned());
    let mut reader = InputTextReader::new(input);
    let mut line = String::new();

    assert!(reader.read_line(&mut line)?);
    assert_eq!("a\n", line);
    let (mut input, pending) = reader.into_parts();
    assert_eq!(vec!['b', 'c'], pending);

    let mut output = ['\0'];
    assert_eq!(0, input.read_fully(&mut output)?);
    Ok(())
}
