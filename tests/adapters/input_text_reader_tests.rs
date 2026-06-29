use std::io::{Error, ErrorKind};

use qubit_io::{BufferedInput, Input};
use qubit_io_text::{InputTextReader, StringCharInput, TextLineRead, TextRead};

#[derive(Debug)]
struct FailingCharInput;

impl Input for FailingCharInput {
    type Item = char;

    unsafe fn read_unchecked(
        &mut self,
        _output: &mut [char],
        _index: usize,
        _count: usize,
    ) -> std::io::Result<usize> {
        Err(Error::other("read failed"))
    }
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
    let input: Box<dyn Input<Item = char>> = Box::new(StringCharInput::new("boxed".to_owned()));
    let mut reader = InputTextReader::from_boxed(input);
    let mut output = String::new();

    assert!(reader.get_ref().is_buffered());
    assert_eq!(5, reader.read_to_string(&mut output)?);
    assert_eq!("boxed", output);
    Ok(())
}

#[test]
fn test_from_boxed_keeps_buffered_input() -> std::io::Result<()> {
    let input: Box<dyn Input<Item = char>> =
        Box::new(BufferedInput::new(StringCharInput::new("buf".to_owned())));
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
