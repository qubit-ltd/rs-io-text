use qubit_io_text::{LineEnding, OutputTextWriter, StringOutput, TextWrite};

#[test]
fn test_write_text_to_char_output() -> std::io::Result<()> {
    let mut text = String::new();
    {
        let output = StringOutput::new(&mut text);
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
        let output = StringOutput::new(&mut text);
        let mut writer = OutputTextWriter::new(output);

        assert_eq!("prefix", writer.get_ref().get_ref());
        writer.get_mut().get_mut().push(':');
        writer.write_str("value")?;
        assert_eq!(LineEnding::Lf, writer.line_ending());
    }

    assert_eq!("prefix:value", text);
    Ok(())
}
