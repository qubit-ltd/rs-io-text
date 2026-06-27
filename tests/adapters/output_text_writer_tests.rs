use std::io::{Error, ErrorKind};

use qubit_io::Output;
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
fn test_accessors_expose_wrapped_output() -> std::io::Result<()> {
    let mut text = String::from("prefix");
    {
        let output = StringCharOutput::new(&mut text);
        let mut writer = OutputTextWriter::new(output);

        assert_eq!("prefix", writer.get_ref().get_ref());
        writer.get_mut().get_mut().push(':');
        writer.write_str("value")?;
        assert_eq!(LineEnding::Lf, writer.line_ending());
    }

    assert_eq!("prefix:value", text);
    Ok(())
}

#[test]
fn test_into_inner_returns_wrapped_output() -> std::io::Result<()> {
    let mut text = String::new();
    {
        let output = StringCharOutput::new(&mut text);
        let mut writer = OutputTextWriter::new(output);

        writer.write_str("value")?;
        let output = writer.into_inner();
        assert_eq!("value", output.get_ref());
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
    assert_other_error(OutputTextWriter::new(FailingCharOutput).write_str(&"a".repeat(256)));
    assert_other_error(OutputTextWriter::new(FailingCharOutput).write_line("x"));
    assert_other_error(OutputTextWriter::new(FailingCharOutput).write_line(""));
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
