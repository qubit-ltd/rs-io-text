use std::io::{
    self,
    ErrorKind,
    Write,
};

use qubit_text_io::{
    LineEnding,
    TextWrite,
    Utf8TextWriter,
};

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("write failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("flush failed"))
    }
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
    let output = Vec::new();
    let mut writer = Utf8TextWriter::new(output);

    assert!(writer.get_ref().is_empty());
    writer.get_mut().extend_from_slice(b"prefix:");
    assert_eq!(LineEnding::Lf, writer.line_ending());
    writer.write_line("done")?;
    writer.flush()?;

    assert_eq!(b"prefix:done\n", writer.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_write_methods_propagate_underlying_errors() {
    let mut writer = Utf8TextWriter::new(FailingWriter);

    assert_eq!(
        ErrorKind::Other,
        writer
            .write_char('x')
            .expect_err("write_char must fail")
            .kind(),
    );
    assert_eq!(
        ErrorKind::Other,
        writer
            .write_chars(&['x'])
            .expect_err("write_chars must fail")
            .kind(),
    );
    assert_eq!(
        ErrorKind::Other,
        writer
            .write_line("x")
            .expect_err("write_line must fail")
            .kind(),
    );
    assert_eq!(
        ErrorKind::Other,
        writer.flush().expect_err("flush must fail").kind(),
    );
}
