use qubit_io::{Input, InputExt};
use qubit_io_text::StrCharInput;

#[test]
fn test_read_reads_chars_from_borrowed_str() -> std::io::Result<()> {
    let text = String::from("a中🙂");
    let mut input = StrCharInput::new(&text);
    let mut output = ['\0'; 4];

    assert_eq!(0, input.position());
    assert_eq!(2, input.read(&mut output[..2])?);
    assert_eq!(['a', '中'], output[..2]);
    assert_eq!("a中".len(), input.position());

    assert_eq!(1, input.read(&mut output[..2])?);
    assert_eq!('🙂', output[0]);
    assert_eq!("a中🙂".len(), input.position());
    assert_eq!(0, input.read(&mut output[..2])?);
    assert_eq!("a中🙂", text);
    Ok(())
}

#[test]
fn test_get_ref_returns_borrowed_str() {
    let text = String::from("a中");
    let input = StrCharInput::new(&text);

    assert_eq!("a中", input.get_ref());
}

#[test]
fn test_read_unchecked_writes_into_indexed_range() -> std::io::Result<()> {
    let mut input = StrCharInput::new("ab中");
    let mut output = ['.'; 5];

    let read = unsafe { input.read_unchecked(&mut output, 1, 3)? };

    assert_eq!(3, read);
    assert_eq!(['.', 'a', 'b', '中', '.'], output);
    assert_eq!("ab中".len(), input.position());
    Ok(())
}

#[test]
fn test_read_exact_or_eof_reads_remaining_chars() -> std::io::Result<()> {
    let mut input = StrCharInput::new("中🙂");
    let mut output = ['\0'; 4];

    let read = input.read_exact_or_eof(&mut output)?;

    assert_eq!(2, read);
    assert_eq!(['中', '🙂'], output[..2]);
    Ok(())
}
