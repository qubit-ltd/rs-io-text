use std::convert::Infallible;

use qubit_io_text::{LineEnding, StringTextWriter, TextWrite};

#[test]
fn test_string_implements_text_write() -> Result<(), Infallible> {
    let mut output = String::new();

    output.write_chars(&[])?;
    output.write_char('中')?;
    output.write_chars(&['a', '🙂'])?;
    output.write_str("raw")?;
    output.write_line("done")?;
    output.flush()?;

    assert_eq!("中a🙂rawdone\n", output);
    Ok(())
}

#[test]
fn test_string_text_writer_uses_configured_line_ending() -> std::io::Result<()> {
    let mut output = String::new();
    {
        let mut writer = StringTextWriter::new(&mut output).with_line_ending(LineEnding::CrLf);

        writer.write_line("first")?;
        writer.write_str("second")?;
        writer.flush()?;
    }

    assert_eq!("first\r\nsecond", output);
    Ok(())
}

#[test]
fn test_string_text_writer_accessors() -> std::io::Result<()> {
    let mut output = String::from("prefix");
    {
        let mut writer = StringTextWriter::new(&mut output);

        assert_eq!("prefix", writer.get_ref());
        writer.get_mut().push(':');
        assert_eq!(LineEnding::Lf, writer.line_ending());
        writer.write_char('中')?;
        writer.write_chars(&['a', 'b'])?;
        writer.write_line("value")?;
    }

    assert_eq!("prefix:中abvalue\n", output);
    Ok(())
}
