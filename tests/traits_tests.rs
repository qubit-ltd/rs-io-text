// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io_text::{
    AsyncTextRead,
    AsyncTextWrite,
    LineEnding,
    TextLineRead,
    TextRead,
    TextWrite,
};

use std::future::Future;
use std::task::{
    Context,
    Poll,
    Waker,
};

fn complete<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test future must not suspend"),
    }
}

struct AsyncReader(std::vec::IntoIter<char>);

impl AsyncTextRead for AsyncReader {
    type Error = ReadError;

    async fn read_char_async(&mut self) -> Result<Option<char>, Self::Error> {
        Ok(self.0.next())
    }
}

struct FailingAsyncReader;

impl AsyncTextRead for FailingAsyncReader {
    type Error = ReadError;

    async fn read_char_async(&mut self) -> Result<Option<char>, Self::Error> {
        Err(ReadError)
    }
}

struct AsyncWriter;

impl AsyncTextWrite for AsyncWriter {
    type Error = WriteError;

    async fn write_char_async(&mut self, _ch: char) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn write_chars_async(
        &mut self,
        chars: &[char],
    ) -> Result<usize, Self::Error> {
        Ok(chars.len())
    }

    async fn write_str_async(
        &mut self,
        text: &str,
    ) -> Result<usize, Self::Error> {
        Ok(text.len())
    }

    async fn write_line_fully_async(
        &mut self,
        _line: &str,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn flush_async(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn finish_async(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct PartialAsyncWriter {
    chars: String,
    text: String,
}

impl AsyncTextWrite for PartialAsyncWriter {
    type Error = WriteError;

    async fn write_char_async(&mut self, ch: char) -> Result<(), Self::Error> {
        self.chars.push(ch);
        Ok(())
    }

    async fn write_chars_async(
        &mut self,
        chars: &[char],
    ) -> Result<usize, Self::Error> {
        let Some(ch) = chars.first() else {
            return Ok(0);
        };
        self.chars.push(*ch);
        Ok(1)
    }

    async fn write_str_async(
        &mut self,
        text: &str,
    ) -> Result<usize, Self::Error> {
        let Some(ch) = text.chars().next() else {
            return Ok(0);
        };
        self.text.push(ch);
        Ok(ch.len_utf8())
    }

    async fn write_line_fully_async(
        &mut self,
        _line: &str,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn flush_async(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn finish_async(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct FailingAsyncWriter;

impl AsyncTextWrite for FailingAsyncWriter {
    type Error = WriteError;

    async fn write_char_async(&mut self, _ch: char) -> Result<(), Self::Error> {
        Err(WriteError)
    }

    async fn write_chars_async(
        &mut self,
        _chars: &[char],
    ) -> Result<usize, Self::Error> {
        Err(WriteError)
    }

    async fn write_str_async(
        &mut self,
        _text: &str,
    ) -> Result<usize, Self::Error> {
        Err(WriteError)
    }

    async fn write_line_fully_async(
        &mut self,
        _line: &str,
    ) -> Result<(), Self::Error> {
        Err(WriteError)
    }

    async fn flush_async(&mut self) -> Result<(), Self::Error> {
        Err(WriteError)
    }

    async fn finish_async(&mut self) -> Result<(), Self::Error> {
        Err(WriteError)
    }
}

struct ZeroProgressAsyncWriter;

impl AsyncTextWrite for ZeroProgressAsyncWriter {
    type Error = WriteError;

    async fn write_char_async(&mut self, _ch: char) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn write_chars_async(
        &mut self,
        _chars: &[char],
    ) -> Result<usize, Self::Error> {
        Ok(0)
    }

    async fn write_str_async(
        &mut self,
        _text: &str,
    ) -> Result<usize, Self::Error> {
        Ok(0)
    }

    async fn write_line_fully_async(
        &mut self,
        _line: &str,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn flush_async(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn finish_async(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn test_async_text_read_defaults_cover_bulk_and_string_reads() {
    let mut reader = AsyncReader("ab".chars().collect::<Vec<_>>().into_iter());
    let mut chars = vec![':'];
    assert_eq!(Ok(1), complete(reader.read_chars_async(&mut chars, 1)));
    assert_eq!(&[':', 'a'], chars.as_slice());
    let mut text = String::from("prefix:");
    assert_eq!(Ok(1), complete(reader.read_to_string_async(&mut text)));
    assert_eq!("prefix:b", text);
}

#[test]
fn test_async_text_read_defaults_cover_limits_and_errors() {
    let mut reader = AsyncReader("a".chars().collect::<Vec<_>>().into_iter());
    let mut chars = Vec::new();
    assert_eq!(Ok(0), complete(reader.read_chars_async(&mut chars, 0)));
    assert_eq!(Ok(1), complete(reader.read_chars_async(&mut chars, 2)));
    assert_eq!(&['a'], chars.as_slice());

    let mut reader = FailingAsyncReader;
    assert_eq!(
        Err(ReadError),
        complete(reader.read_chars_async(&mut Vec::new(), 1)),
    );
    assert_eq!(
        Err(ReadError),
        complete(reader.read_to_string_async(&mut String::new())),
    );
}

#[test]
fn test_async_text_write_default_line_ending() {
    assert_eq!(LineEnding::Lf, AsyncWriter.line_ending());
}

#[test]
fn test_async_text_write_defaults_complete_partial_writes() {
    let mut writer = PartialAsyncWriter {
        chars: String::new(),
        text: String::new(),
    };

    assert_eq!(
        Ok(()),
        complete(writer.write_chars_fully_async(&['A', '中', '🙂'])),
    );
    assert_eq!(Ok(()), complete(writer.write_str_fully_async("A中🙂")));
    assert_eq!("A中🙂", writer.chars);
    assert_eq!("A中🙂", writer.text);
}

#[test]
fn test_async_text_write_defaults_propagate_errors() {
    let mut writer = FailingAsyncWriter;

    assert_eq!(
        Err(WriteError),
        complete(writer.write_chars_fully_async(&['A'])),
    );
    assert_eq!(Err(WriteError), complete(writer.write_str_fully_async("A")),);
}

#[test]
#[should_panic(
    expected = "AsyncTextWrite::write_chars_async returned zero for nonempty input"
)]
fn test_async_text_write_default_character_write_rejects_zero_progress() {
    let mut writer = ZeroProgressAsyncWriter;
    let _ = complete(writer.write_chars_fully_async(&['A']));
}

#[test]
#[should_panic(
    expected = "AsyncTextWrite::write_str_async returned zero for nonempty input"
)]
fn test_async_text_write_default_string_write_rejects_zero_progress() {
    let mut writer = ZeroProgressAsyncWriter;
    let _ = complete(writer.write_str_fully_async("A"));
}

#[derive(Debug, Eq, PartialEq)]
struct ReadError;

struct FailingTextReader;

impl TextRead for FailingTextReader {
    type Error = ReadError;

    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        Err(ReadError)
    }
}

struct EmptyTextReader;

impl TextRead for EmptyTextReader {
    type Error = ReadError;

    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        Ok(None)
    }
}

struct FailAfterCharReader {
    first: char,
    emitted: bool,
    fail_after_first: bool,
}

impl TextRead for FailAfterCharReader {
    type Error = ReadError;

    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        if self.emitted {
            if self.fail_after_first {
                Err(ReadError)
            } else {
                Ok(None)
            }
        } else {
            self.emitted = true;
            Ok(Some(self.first))
        }
    }
}

impl TextLineRead for FailAfterCharReader {}

struct SequenceTextReader {
    chars: std::vec::IntoIter<char>,
}

impl SequenceTextReader {
    fn new(text: &str) -> Self {
        Self {
            chars: text.chars().collect::<Vec<_>>().into_iter(),
        }
    }
}

impl TextRead for SequenceTextReader {
    type Error = ReadError;

    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        Ok(self.chars.next())
    }
}

impl TextLineRead for SequenceTextReader {}

impl TextLineRead for FailingTextReader {}

#[derive(Debug, Eq, PartialEq)]
struct WriteError;

struct FailingTextWriter {
    calls: usize,
    fail_first: bool,
}

impl TextWrite for FailingTextWriter {
    type Error = WriteError;

    fn write_str(&mut self, _text: &str) -> Result<(), Self::Error> {
        let fail = self.fail_first || self.calls > 0;
        self.calls += 1;
        if fail { Err(WriteError) } else { Ok(()) }
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct FailOnSecondWrite {
    calls: usize,
}

#[derive(Default)]
struct MinimalTextWriter {
    output: String,
}

impl TextWrite for MinimalTextWriter {
    type Error = WriteError;

    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        self.output.push_str(text);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl TextWrite for FailOnSecondWrite {
    type Error = WriteError;

    fn write_str(&mut self, _text: &str) -> Result<(), Self::Error> {
        self.calls += 1;
        if self.calls == 2 {
            return Err(WriteError);
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn test_default_read_methods_append_requested_text() {
    let mut reader = SequenceTextReader::new("ab中");
    let mut chars = vec![':'];
    let mut text = String::from("prefix:");

    assert_eq!(Ok(2), reader.read_chars(&mut chars, 2));
    assert_eq!(&[':', 'a', 'b'], chars.as_slice());
    assert_eq!(Ok(1), reader.read_to_string(&mut text));
    assert_eq!("prefix:中", text);
}

#[test]
fn test_default_read_methods_cover_zero_limit_and_errors() {
    let mut reader = SequenceTextReader::new("A");
    let mut chars = Vec::new();
    assert_eq!(Ok(0), reader.read_chars(&mut chars, 0));
    assert_eq!(Ok(Some('A')), reader.read_char());

    let mut reader = FailingTextReader;
    assert_eq!(Err(ReadError), reader.read_to_string(&mut String::new()));
    assert_eq!(Err(ReadError), reader.read_line(&mut String::new()));
}

#[test]
fn test_default_read_methods_cover_empty_and_repeated_eof_paths() {
    let mut reader = EmptyTextReader;
    let mut output = String::from("seed:");
    assert_eq!(Ok(0), reader.read_to_string(&mut output));
    assert_eq!("seed:", output);

    let mut reader = SequenceTextReader::new("A");
    assert_eq!(Ok(1), reader.read_to_string(&mut output));
    assert_eq!(Ok(0), reader.read_to_string(&mut output));
}

#[test]
fn test_default_read_line_preserves_terminator_and_eof() {
    let mut reader = SequenceTextReader::new("first\nlast");
    let mut output = String::new();

    assert_eq!(Ok(true), reader.read_line(&mut output));
    assert_eq!("first\n", output);
    output.clear();
    assert_eq!(Ok(true), reader.read_line(&mut output));
    assert_eq!("last", output);
    output.clear();
    assert_eq!(Ok(false), reader.read_line(&mut output));

    let mut reader = SequenceTextReader::new("");
    output.clear();
    assert_eq!(Ok(false), reader.read_line(&mut output));
}

#[test]
fn test_read_chars_propagates_read_errors() {
    let mut reader = FailingTextReader;
    let mut chars = Vec::new();

    assert_eq!(Err(ReadError), reader.read_chars(&mut chars, 1));
    assert!(chars.is_empty());

    let mut reader = FailAfterCharReader {
        first: 'a',
        emitted: false,
        fail_after_first: true,
    };
    assert_eq!(Err(ReadError), reader.read_chars(&mut chars, 2));

    let mut reader = FailAfterCharReader {
        first: 'a',
        emitted: false,
        fail_after_first: true,
    };
    assert_eq!(Err(ReadError), reader.read_to_string(&mut String::new()));

    let mut reader = FailAfterCharReader {
        first: 'a',
        emitted: false,
        fail_after_first: true,
    };
    assert_eq!(Err(ReadError), reader.read_line(&mut String::new()));

    let mut reader = FailAfterCharReader {
        first: '\n',
        emitted: false,
        fail_after_first: true,
    };
    let mut line = String::new();
    assert_eq!(Ok(true), reader.read_line(&mut line));
    assert_eq!("\n", line);

    let mut reader = FailAfterCharReader {
        first: 'a',
        emitted: false,
        fail_after_first: false,
    };
    let mut chars = Vec::new();
    assert_eq!(Ok(1), reader.read_chars(&mut chars, 2));
    assert_eq!(&['a'], chars.as_slice());

    let mut reader = FailAfterCharReader {
        first: 'a',
        emitted: false,
        fail_after_first: false,
    };
    let mut text = String::new();
    assert_eq!(Ok(1), reader.read_to_string(&mut text));
    assert_eq!("a", text);

    let mut reader = FailAfterCharReader {
        first: 'a',
        emitted: false,
        fail_after_first: false,
    };
    let mut line = String::new();
    assert_eq!(Ok(true), reader.read_line(&mut line));
    assert_eq!("a", line);
}

#[test]
fn test_read_chars_stops_at_eof_without_appending() {
    let mut reader = EmptyTextReader;
    let mut chars = Vec::new();

    assert_eq!(Ok(0), reader.read_chars(&mut chars, 1));
    assert!(chars.is_empty());
}

#[test]
fn test_write_char_and_chars_propagate_write_errors() {
    let mut writer = FailingTextWriter {
        calls: 0,
        fail_first: true,
    };

    assert_eq!(LineEnding::Lf, writer.line_ending());
    assert_eq!(Err(WriteError), writer.write_char('x'));
    assert_eq!(Err(WriteError), writer.write_chars(&['x']));
    assert_eq!(Err(WriteError), writer.write_line("line"));
    assert_eq!(Ok(()), writer.write_chars(&[]));

    let mut writer = FailingTextWriter {
        calls: 0,
        fail_first: false,
    };
    assert_eq!(Err(WriteError), writer.write_line("line"));
}

#[test]
fn test_write_line_propagates_line_ending_errors() {
    let mut writer = FailOnSecondWrite { calls: 0 };

    assert_eq!(Err(WriteError), writer.write_line("line"));
    assert_eq!(2, writer.calls);
}

#[test]
fn test_default_write_methods_write_complete_text() {
    let mut writer = MinimalTextWriter::default();

    assert_eq!(Ok(()), writer.write_char('中'));
    assert_eq!(Ok(()), writer.write_chars(&['A', '🙂']));
    assert_eq!(Ok(()), writer.write_chars(&[]));
    assert_eq!(Ok(()), writer.write_line("line"));
    assert_eq!(Ok(()), writer.flush());

    assert_eq!("中A🙂line\n", writer.output);
}
