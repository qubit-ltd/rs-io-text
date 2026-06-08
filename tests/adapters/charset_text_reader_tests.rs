use std::io::{self, Cursor, ErrorKind, Read};

use qubit_io_text::{
    CharsetReadExt, CharsetTextReader, CodingErrorPolicy, TextLineRead, TextRead, Utf8Codec,
};

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("read failed"))
    }
}

#[test]
fn test_new_decodes_utf8_text() -> std::io::Result<()> {
    let bytes = "中文\nsecond".as_bytes().to_vec();
    let mut reader =
        CharsetTextReader::new(Cursor::new(bytes), Utf8Codec, CodingErrorPolicy::Strict);
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
    let mut reader = CharsetTextReader::new(
        Cursor::new("中文".as_bytes().to_vec()),
        Utf8Codec,
        CodingErrorPolicy::Strict,
    );

    assert_eq!(Some('中'), reader.read_char()?);

    let inner = reader.into_inner();
    assert_eq!(6, inner.position());
    Ok(())
}

#[test]
fn test_read_chars_after_decoding() -> std::io::Result<()> {
    let mut reader = CharsetTextReader::new(
        Cursor::new("中文".as_bytes().to_vec()),
        Utf8Codec,
        CodingErrorPolicy::Strict,
    );
    let mut chars = Vec::new();

    assert_eq!(2, reader.read_chars(&mut chars, 8)?);
    assert_eq!(vec!['中', '文'], chars);
    Ok(())
}

#[test]
fn test_new_propagates_reader_errors() {
    let mut reader = CharsetTextReader::new(FailingReader, Utf8Codec, CodingErrorPolicy::Strict);
    let error = reader
        .read_char()
        .expect_err("reader errors must be propagated");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_new_rejects_invalid_bytes_in_strict_mode() {
    let mut reader = CharsetTextReader::new(
        Cursor::new(vec![0xFF]),
        Utf8Codec,
        CodingErrorPolicy::Strict,
    );
    let error = reader
        .read_char()
        .expect_err("strict mode must reject invalid text");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_new_replaces_invalid_bytes_in_replace_mode() -> std::io::Result<()> {
    let mut reader = CharsetTextReader::new(
        Cursor::new(vec![0xFF]),
        Utf8Codec,
        CodingErrorPolicy::Replace,
    );
    let mut output = String::new();

    assert_eq!(1, reader.read_to_string(&mut output)?);
    assert_eq!("\u{FFFD}", output);
    Ok(())
}

#[test]
fn test_new_reports_incomplete_bytes_in_strict_mode() {
    let mut reader = CharsetTextReader::new(
        Cursor::new(vec![0xE4, 0xB8]),
        Utf8Codec,
        CodingErrorPolicy::Strict,
    );
    let error = reader
        .read_char()
        .expect_err("strict mode must reject incomplete text");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_new_replaces_incomplete_bytes_in_replace_mode() -> std::io::Result<()> {
    let mut reader = CharsetTextReader::new(
        Cursor::new(vec![0xE4, 0xB8]),
        Utf8Codec,
        CodingErrorPolicy::Replace,
    );
    let mut output = String::new();

    assert_eq!(1, reader.read_to_string(&mut output)?);
    assert_eq!("\u{FFFD}", output);
    Ok(())
}

#[test]
fn test_with_capacity_preserves_utf8_tail_across_refills() -> std::io::Result<()> {
    let input = Cursor::new("中🙂".as_bytes().to_vec());
    let mut reader =
        CharsetTextReader::with_capacity(input, Utf8Codec, CodingErrorPolicy::Strict, 1);

    assert_eq!(Some('中'), reader.read_char()?);
    assert_eq!(Some('🙂'), reader.read_char()?);
    assert_eq!(None, reader.read_char()?);
    Ok(())
}

#[test]
fn test_charset_read_ext_creates_stream_reader() -> std::io::Result<()> {
    let input = Cursor::new("ext中文".as_bytes().to_vec());
    let mut reader = input.charset_text_reader(Utf8Codec, CodingErrorPolicy::Strict);
    let mut output = String::new();

    assert_eq!(5, reader.read_to_string(&mut output)?);
    assert_eq!("ext中文", output);
    Ok(())
}
