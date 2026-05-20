use std::io::{
    self,
    BufReader,
    Cursor,
    ErrorKind,
    Read,
};

use qubit_text_io::{
    TextLineRead,
    TextRead,
    Utf8TextReader,
};

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("read failed"))
    }
}

#[test]
fn test_read_char_and_line_from_utf8_reader() -> std::io::Result<()> {
    let input = Cursor::new("a中\nβeta".as_bytes().to_vec());
    let mut reader = Utf8TextReader::from_read(input);
    let mut line = String::new();

    assert_eq!(Some('a'), reader.read_char()?);
    assert!(reader.read_line(&mut line)?);
    assert_eq!("中\n", line);

    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("βeta", line);
    assert!(!reader.read_line(&mut line)?);
    Ok(())
}

#[test]
fn test_new_accessors_and_into_inner() {
    let input = Cursor::new("abc".as_bytes().to_vec());
    let mut reader = Utf8TextReader::new(BufReader::new(input));

    assert_eq!(3, reader.get_ref().get_ref().get_ref().len());
    assert_eq!(0, reader.get_mut().get_mut().position());

    let inner = reader.into_inner();
    assert_eq!(0, inner.into_inner().position());
}

#[test]
fn test_read_char_covers_utf8_widths_and_eof() -> std::io::Result<()> {
    let input = Cursor::new("aé中🙂".as_bytes().to_vec());
    let mut reader = Utf8TextReader::from_read(input);

    assert_eq!(Some('a'), reader.read_char()?);
    assert_eq!(Some('é'), reader.read_char()?);
    assert_eq!(Some('中'), reader.read_char()?);
    assert_eq!(Some('🙂'), reader.read_char()?);
    assert_eq!(None, reader.read_char()?);
    Ok(())
}

#[test]
fn test_read_chars_reads_utf8_scalars() -> std::io::Result<()> {
    let input = Cursor::new("a中🙂".as_bytes().to_vec());
    let mut reader = Utf8TextReader::from_read(input);
    let mut chars = Vec::new();

    assert_eq!(2, reader.read_chars(&mut chars, 2)?);
    assert_eq!(vec!['a', '中'], chars);
    assert_eq!(1, reader.read_chars(&mut chars, 4)?);
    assert_eq!(vec!['a', '中', '🙂'], chars);
    Ok(())
}

#[test]
fn test_read_chars_propagates_utf8_errors() {
    let input = Cursor::new(vec![0xFF]);
    let mut reader = Utf8TextReader::from_read(input);
    let mut chars = Vec::new();

    let error = reader
        .read_chars(&mut chars, 1)
        .expect_err("invalid UTF-8 must be rejected while reading chars");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_to_string_appends_valid_utf8() -> std::io::Result<()> {
    let input = Cursor::new("中🙂".as_bytes().to_vec());
    let mut reader = Utf8TextReader::from_read(input);
    let mut output = String::from("prefix:");

    assert_eq!(2, reader.read_to_string(&mut output)?);
    assert_eq!("prefix:中🙂", output);
    Ok(())
}

#[test]
fn test_read_to_string_reports_invalid_utf8() {
    let input = Cursor::new(vec![0xFF]);
    let mut reader = Utf8TextReader::from_read(input);
    let mut output = String::new();

    let error = reader
        .read_to_string(&mut output)
        .expect_err("invalid UTF-8 must be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_line_reports_invalid_utf8() {
    let input = Cursor::new(vec![0xFF]);
    let mut reader = Utf8TextReader::from_read(input);
    let mut line = String::new();

    let error = reader
        .read_line(&mut line)
        .expect_err("invalid UTF-8 line must be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_char_propagates_io_errors() {
    let mut reader = Utf8TextReader::from_read(FailingReader);

    let error = reader
        .read_char()
        .expect_err("reader I/O errors must be propagated");
    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_read_char_reports_invalid_utf8() {
    let input = Cursor::new(vec![0xE4, 0xFF, 0xAD]);
    let mut reader = Utf8TextReader::from_read(input);

    let error = reader
        .read_char()
        .expect_err("invalid UTF-8 scalar must be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_char_reports_invalid_leading_byte() {
    let input = Cursor::new(vec![0xFF]);
    let mut reader = Utf8TextReader::from_read(input);

    let error = reader
        .read_char()
        .expect_err("invalid UTF-8 leading byte must be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_char_reports_unexpected_eof_in_utf8_sequence() {
    let input = Cursor::new(vec![0xE4, 0xB8]);
    let mut reader = Utf8TextReader::from_read(input);

    let error = reader
        .read_char()
        .expect_err("truncated UTF-8 scalar must be rejected");
    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
}

#[test]
fn test_read_char_reports_unexpected_eof_for_two_byte_sequence() {
    let input = Cursor::new(vec![0xC2]);
    let mut reader = Utf8TextReader::from_read(input);

    let error = reader
        .read_char()
        .expect_err("truncated two-byte UTF-8 scalar must be rejected");
    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
}

#[test]
fn test_read_char_reports_unexpected_eof_for_four_byte_sequence() {
    let input = Cursor::new(vec![0xF0, 0x9F]);
    let mut reader = Utf8TextReader::from_read(input);

    let error = reader
        .read_char()
        .expect_err("truncated four-byte UTF-8 scalar must be rejected");
    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
}
