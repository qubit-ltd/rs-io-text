use std::io::{
    self,
    ErrorKind,
    Write,
};

use encoding_rs::GBK;
use qubit_text_io::{
    CodingErrorPolicy,
    EncodedTextWriter,
    LineEnding,
    TextWrite,
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
fn test_write_gbk_text_to_byte_writer() -> std::io::Result<()> {
    let mut output = Vec::new();
    {
        let mut writer = EncodedTextWriter::new(&mut output, GBK, CodingErrorPolicy::Strict)
            .with_line_ending(LineEnding::CrLf);

        writer.write_char('A')?;
        writer.write_chars(&['B', 'C'])?;
        writer.write_line("中文")?;
        writer.flush()?;
    }

    let (decoded, _, had_errors) = GBK.decode(output.as_slice());
    assert!(!had_errors);
    assert_eq!("ABC中文\r\n", decoded);
    Ok(())
}

#[test]
fn test_write_rejects_unencodable_text_in_strict_mode() {
    let mut output = Vec::new();
    let mut writer = EncodedTextWriter::new(&mut output, GBK, CodingErrorPolicy::Strict);

    let error = writer
        .write_str("🙂")
        .expect_err("strict mode must reject unencodable text");
    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_write_chars_rejects_unencodable_text_in_strict_mode() {
    let mut output = Vec::new();
    let mut writer = EncodedTextWriter::new(&mut output, GBK, CodingErrorPolicy::Strict);

    let error = writer
        .write_chars(&['🙂'])
        .expect_err("strict mode must reject unencodable chars");
    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_write_replaces_unencodable_text_in_replace_mode() -> std::io::Result<()> {
    let mut output = Vec::new();
    {
        let mut writer = EncodedTextWriter::new(&mut output, GBK, CodingErrorPolicy::Replace);

        writer.write_str("🙂")?;
        writer.flush()?;
    }

    assert!(!output.is_empty());
    Ok(())
}

#[test]
fn test_accessors_and_into_inner() -> std::io::Result<()> {
    let output = Vec::new();
    let mut writer = EncodedTextWriter::new(output, GBK, CodingErrorPolicy::Strict);

    assert!(writer.get_ref().is_empty());
    writer.get_mut().extend_from_slice(b"prefix:");
    assert_eq!(LineEnding::Lf, writer.line_ending());
    writer.write_str("ascii")?;
    writer.flush()?;

    assert_eq!(b"prefix:ascii", writer.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_write_methods_propagate_underlying_errors() {
    let mut writer = EncodedTextWriter::new(FailingWriter, GBK, CodingErrorPolicy::Strict);

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
