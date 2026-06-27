use std::io::{Error, ErrorKind};

use qubit_io::Input;
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
    assert_eq!("a中", reader.into_inner().into_inner());
    Ok(())
}

#[test]
fn test_accessors_expose_wrapped_input() -> std::io::Result<()> {
    let input = StringCharInput::new("abc".to_owned());
    let mut reader = InputTextReader::new(input);

    assert_eq!("abc", reader.get_ref().get_ref());
    assert_eq!(0, reader.get_mut().position());
    assert_eq!(Some('a'), reader.read_char()?);
    assert_eq!(1, reader.get_ref().position());
    assert_eq!("abc", reader.into_inner().into_inner());
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
