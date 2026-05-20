use std::io::{
    self,
    Cursor,
    ErrorKind,
    Read,
};

use encoding_rs::{
    GBK,
    UTF_8,
};
use qubit_text_io::{
    CodingErrorPolicy,
    EncodedTextReader,
    TextLineRead,
    TextRead,
};

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("read failed"))
    }
}

#[test]
fn test_new_decodes_gbk_text() -> std::io::Result<()> {
    let (bytes, _, had_errors) = GBK.encode("中文\nsecond");
    assert!(!had_errors);

    let mut reader = EncodedTextReader::new(
        Cursor::new(bytes.into_owned()),
        GBK,
        CodingErrorPolicy::Strict,
    )?;
    let mut line = String::new();

    assert!(reader.read_line(&mut line)?);
    assert_eq!("中文\n", line);

    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("second", line);
    Ok(())
}

#[test]
fn test_read_char_and_into_inner_after_decoding() -> std::io::Result<()> {
    let (bytes, _, had_errors) = GBK.encode("中文");
    assert!(!had_errors);
    let mut reader = EncodedTextReader::new(
        Cursor::new(bytes.into_owned()),
        GBK,
        CodingErrorPolicy::Strict,
    )?;

    assert_eq!(Some('中'), reader.read_char()?);

    let inner = reader.into_inner();
    assert_eq!(3, inner.position());
    Ok(())
}

#[test]
fn test_read_chars_after_decoding() -> std::io::Result<()> {
    let (bytes, _, had_errors) = GBK.encode("中文");
    assert!(!had_errors);
    let mut reader = EncodedTextReader::new(
        Cursor::new(bytes.into_owned()),
        GBK,
        CodingErrorPolicy::Strict,
    )?;
    let mut chars = Vec::new();

    assert_eq!(2, reader.read_chars(&mut chars, 8)?);
    assert_eq!(vec!['中', '文'], chars);
    Ok(())
}

#[test]
fn test_new_propagates_reader_errors() {
    let error = EncodedTextReader::new(FailingReader, UTF_8, CodingErrorPolicy::Strict)
        .expect_err("reader errors must be propagated");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_new_rejects_invalid_bytes_in_strict_mode() {
    let error = EncodedTextReader::new(Cursor::new(vec![0xFF]), UTF_8, CodingErrorPolicy::Strict)
        .expect_err("strict mode must reject invalid text");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_new_replaces_invalid_bytes_in_replace_mode() -> std::io::Result<()> {
    let mut reader =
        EncodedTextReader::new(Cursor::new(vec![0xFF]), UTF_8, CodingErrorPolicy::Replace)?;
    let mut output = String::new();

    assert_eq!(1, reader.read_to_string(&mut output)?);
    assert_eq!("\u{FFFD}", output);
    Ok(())
}
