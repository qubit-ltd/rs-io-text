use std::convert::Infallible;

use qubit_io_text::{
    StrTextReader,
    TextLineRead,
    TextRead,
};

#[test]
fn test_read_char_returns_unicode_scalars() -> Result<(), Infallible> {
    let mut reader = StrTextReader::new("a中🙂");

    assert_eq!(0, reader.position());
    assert_eq!(Some('a'), reader.read_char()?);
    assert_eq!(1, reader.position());
    assert_eq!(Some('中'), reader.read_char()?);
    assert_eq!(Some('🙂'), reader.read_char()?);
    assert_eq!(None, reader.read_char()?);
    Ok(())
}

#[test]
fn test_read_chars_reads_at_most_requested_count() -> Result<(), Infallible> {
    let mut reader = StrTextReader::new("ab中🙂");
    let mut chars = Vec::new();

    assert_eq!(0, reader.read_chars(&mut chars, 0)?);
    assert_eq!(3, reader.read_chars(&mut chars, 3)?);
    assert_eq!(vec!['a', 'b', '中'], chars);
    assert_eq!(1, reader.read_chars(&mut chars, 8)?);
    assert_eq!(vec!['a', 'b', '中', '🙂'], chars);
    assert_eq!(0, reader.read_chars(&mut chars, 8)?);
    Ok(())
}

#[test]
fn test_read_line_appends_line_with_terminator() -> Result<(), Infallible> {
    let mut reader = StrTextReader::new("first\r\nsecond\nlast");
    let mut line = String::from("prefix:");

    assert!(reader.read_line(&mut line)?);
    assert_eq!("prefix:first\r\n", line);

    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("second\n", line);

    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("last", line);

    line.clear();
    assert!(!reader.read_line(&mut line)?);
    assert!(line.is_empty());
    Ok(())
}

#[test]
fn test_read_to_string_appends_remaining_text() -> Result<(), Infallible> {
    let mut reader = StrTextReader::new("a中🙂");
    assert_eq!(Some('a'), reader.read_char()?);

    let mut output = String::from("prefix:");
    assert_eq!(2, reader.read_to_string(&mut output)?);
    assert_eq!("prefix:中🙂", output);
    assert_eq!(0, reader.read_to_string(&mut output)?);
    Ok(())
}
