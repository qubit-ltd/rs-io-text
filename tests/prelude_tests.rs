use std::convert::Infallible;

use qubit_text_io::prelude::{
    StrTextReader,
    TextRead,
    TextWrite,
};

#[test]
fn test_prelude_exports_text_traits_and_adapters() -> Result<(), Infallible> {
    let mut reader = StrTextReader::new("text");
    let mut output = String::new();

    assert_eq!(4, reader.read_to_string(&mut output)?);
    output.write_line("!")?;

    assert_eq!("text!\n", output);
    Ok(())
}
