use qubit_io::{Output, OutputExt};
use qubit_io_text::StringCharOutput;

#[test]
fn test_write_appends_chars_to_borrowed_string() -> std::io::Result<()> {
    let mut text = String::from("prefix:");
    {
        let mut output = StringCharOutput::new(&mut text);

        assert_eq!(2, output.write(&['中', '🙂'])?);
        output.flush()?;
    }

    assert_eq!("prefix:中🙂", text);
    Ok(())
}

#[test]
fn test_write_unchecked_reads_from_indexed_range() -> std::io::Result<()> {
    let mut text = String::new();
    let mut output = StringCharOutput::new(&mut text);
    let input = ['.', 'a', '中', '🙂', '.'];

    let written = unsafe { output.write_unchecked(&input, 1, 3)? };

    assert_eq!(3, written);
    assert_eq!("a中🙂", output.get_ref());
    Ok(())
}

#[test]
fn test_write_all_writes_complete_char_slice() -> std::io::Result<()> {
    let mut text = String::new();
    let mut output = StringCharOutput::new(&mut text);

    output.write_all(&['a', '中', '🙂'])?;

    assert_eq!("a中🙂", output.get_ref());
    Ok(())
}
