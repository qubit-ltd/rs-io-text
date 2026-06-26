use qubit_io_text::{
    StringTextReader,
    TextLineRead,
    TextRead,
};

#[test]
fn test_from_string_reads_owned_text() -> std::io::Result<()> {
    let mut reader = StringTextReader::new("alpha\nβeta".to_owned());
    let mut line = String::new();

    assert_eq!(0, reader.position());
    assert!(reader.read_line(&mut line)?);
    assert_eq!("alpha\n", line);
    assert_eq!(6, reader.position());
    assert_eq!(Some('β'), reader.read_char()?);
    Ok(())
}

#[test]
fn test_read_chars_reads_owned_text() -> std::io::Result<()> {
    let mut reader = StringTextReader::new("ab中".to_owned());
    let mut chars = Vec::new();

    assert_eq!(2, reader.read_chars(&mut chars, 2)?);
    assert_eq!(vec!['a', 'b'], chars);
    assert_eq!(1, reader.read_chars(&mut chars, 8)?);
    assert_eq!(vec!['a', 'b', '中'], chars);
    Ok(())
}

#[test]
fn test_read_to_string_appends_remaining_owned_text() -> std::io::Result<()> {
    let mut reader = StringTextReader::new("ab中".to_owned());
    let mut output = String::from("prefix:");

    assert_eq!(3, reader.read_to_string(&mut output)?);
    assert_eq!("prefix:ab中", output);
    assert_eq!(5, reader.position());
    Ok(())
}

#[test]
fn test_into_inner_returns_original_text() {
    let reader = StringTextReader::new("payload".to_owned());

    assert_eq!("payload", reader.into_inner());
}
