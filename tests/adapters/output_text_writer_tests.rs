use std::io::{Error, ErrorKind};

use qubit_io::{BufferedOutput, Output};
use qubit_io_text::{LineEnding, OutputTextWriter, StringCharOutput, TextWrite};

#[derive(Debug)]
struct FailingCharOutput;

impl Output for FailingCharOutput {
    type Item = char;

    unsafe fn write_unchecked(
        &mut self,
        _input: &[char],
        _index: usize,
        _count: usize,
    ) -> std::io::Result<usize> {
        Err(Error::other("write failed"))
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
        let output: Box<dyn Output<Item = char> + '_> = Box::new(StringCharOutput::new(&mut text));
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
        let output: Box<dyn Output<Item = char> + '_> =
            Box::new(BufferedOutput::new(StringCharOutput::new(&mut text)));
        let mut writer = OutputTextWriter::from_boxed(output).with_line_ending(LineEnding::Cr);

        assert!(writer.get_ref().is_buffered());
        writer.write_line("box")?;
        let output = writer.into_inner()?;
        drop(output);
    }

    assert_eq!("box\r", text);
    Ok(())
}

#[test]
fn test_into_inner_propagates_flush_error() {
    let mut writer = OutputTextWriter::new(FailingCharOutput);

    writer
        .write_str("pending")
        .expect("buffered write may succeed before flush");
    let error = match writer.into_inner() {
        Ok(_) => panic!("into_inner should propagate flush error"),
        Err(error) => error,
    };
    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_into_inner_flushes_wrapped_output() -> std::io::Result<()> {
    let mut text = String::new();
    {
        let output = StringCharOutput::new(&mut text);
        let mut writer = OutputTextWriter::new(output);

        writer.write_str("value")?;
        let output = writer.into_inner()?;
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
