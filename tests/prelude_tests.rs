use std::convert::Infallible;

use qubit_io_text::prelude::{
    CharsetReadExt,
    CharsetWriteExt,
    InputTextReader,
    OutputTextWriter,
    StrTextReader,
    StringInput,
    StringOutput,
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

#[test]
fn test_prelude_exports_char_io_adapters() -> std::io::Result<()> {
    let input = StringInput::new("中".to_owned());
    let mut reader = InputTextReader::new(input);
    assert_eq!(Some('中'), reader.read_char()?);

    let mut text = String::new();
    {
        let output = StringOutput::new(&mut text);
        let mut writer = OutputTextWriter::new(output);
        writer.write_str("value")?;
    }
    assert_eq!("value", text);
    Ok(())
}

#[test]
fn test_prelude_exports_charset_ext_traits() -> std::io::Result<()> {
    use qubit_codec_text::{
        CharsetDecodePolicy,
        CharsetDecoder,
        CharsetEncodePolicy,
        CharsetEncoder,
    };
    use qubit_io_text::prelude::{
        AsciiCodec,
        BufferedReader,
        BufferedWriter,
        CharsetStringDecoder,
        CharsetStringEncoder,
        CodingErrorPolicy,
        Utf8Codec,
    };
    use std::io::Cursor;

    let mut reader = Cursor::new(b"text".to_vec())
        .charset_text_reader(Utf8Codec, CodingErrorPolicy::Strict);
    let mut text = String::new();
    reader.read_to_string(&mut text)?;
    assert_eq!("text", text);

    let mut bytes = Vec::new();
    bytes.write_str_with_charset("A", AsciiCodec, CodingErrorPolicy::Strict)?;
    assert_eq!(b"A", bytes.as_slice());

    let mut string_encoder = CharsetStringEncoder::new(Utf8Codec);
    let encoded = string_encoder
        .encode_str("D")
        .expect("prelude string encoder should encode UTF-8");
    let mut string_decoder = CharsetStringDecoder::new(Utf8Codec);
    let decoded = string_decoder
        .decode_to_string(&encoded)
        .expect("prelude string decoder should decode UTF-8");
    assert_eq!("D", decoded);

    let decoder =
        CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
    let mut reader = BufferedReader::new(
        Cursor::new(b"B".to_vec()),
        decoder,
        CodingErrorPolicy::Strict,
    );
    assert_eq!(Some('B'), reader.read_char()?);

    let encoder =
        CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
            .expect("UTF-8 strict encoder should be constructible");
    let mut writer = BufferedWriter::new(Vec::new(), encoder);
    writer.write_str("C")?;
    assert_eq!(b"C", writer.into_inner()?.as_slice());
    Ok(())
}
