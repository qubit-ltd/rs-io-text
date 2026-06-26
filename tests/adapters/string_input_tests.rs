use qubit_io::{Input, InputExt};
use qubit_io_text::StringInput;

#[test]
fn test_read_reads_chars_from_owned_string() -> std::io::Result<()> {
    let mut input = StringInput::new("a中🙂".to_owned());
    let mut output = ['\0'; 4];

    assert_eq!(0, input.position());
    assert_eq!(2, input.read(&mut output[..2])?);
    assert_eq!(['a', '中'], output[..2]);
    assert_eq!("a中".len(), input.position());

    assert_eq!(1, input.read(&mut output[..2])?);
    assert_eq!('🙂', output[0]);
    assert_eq!("a中🙂".len(), input.position());
    assert_eq!(0, input.read(&mut output[..2])?);
    Ok(())
}

#[test]
fn test_read_unchecked_writes_into_indexed_range() -> std::io::Result<()> {
    let mut input = StringInput::new("ab中".to_owned());
    let mut output = ['.'; 5];

    let read = unsafe { input.read_unchecked(&mut output, 1, 3)? };

    assert_eq!(3, read);
    assert_eq!(['.', 'a', 'b', '中', '.'], output);
    assert_eq!("ab中".len(), input.position());
    Ok(())
}

#[test]
fn test_read_exact_or_eof_reads_remaining_chars() -> std::io::Result<()> {
    let mut input = StringInput::new("中🙂".to_owned());
    let mut output = ['\0'; 4];

    let read = input.read_exact_or_eof(&mut output)?;

    assert_eq!(2, read);
    assert_eq!(['中', '🙂'], output[..2]);
    assert_eq!("中🙂", input.into_inner());
    Ok(())
}
