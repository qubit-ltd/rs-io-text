use qubit_io_text::{InputTextReader, StringInput, TextLineRead, TextRead};

#[test]
fn test_read_char_reads_from_char_input() -> std::io::Result<()> {
    let input = StringInput::new("a中".to_owned());
    let mut reader = InputTextReader::new(input);

    assert_eq!(Some('a'), reader.read_char()?);
    assert_eq!(Some('中'), reader.read_char()?);
    assert_eq!(None, reader.read_char()?);
    assert_eq!("a中", reader.into_inner().into_inner());
    Ok(())
}

#[test]
fn test_read_chars_appends_up_to_max() -> std::io::Result<()> {
    let input = StringInput::new("ab中".to_owned());
    let mut reader = InputTextReader::new(input);
    let mut output = vec!['x'];

    assert_eq!(2, reader.read_chars(&mut output, 2)?);
    assert_eq!(vec!['x', 'a', 'b'], output);
    assert_eq!(1, reader.read_chars(&mut output, 8)?);
    assert_eq!(vec!['x', 'a', 'b', '中'], output);
    Ok(())
}

#[test]
fn test_read_line_keeps_line_ending() -> std::io::Result<()> {
    let input = StringInput::new("alpha\nβeta".to_owned());
    let mut reader = InputTextReader::new(input);
    let mut line = String::new();

    assert!(reader.read_line(&mut line)?);
    assert_eq!("alpha\n", line);
    line.clear();
    assert!(reader.read_line(&mut line)?);
    assert_eq!("βeta", line);
    line.clear();
    assert!(!reader.read_line(&mut line)?);
    Ok(())
}
