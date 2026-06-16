use std::{
    io::{self, ErrorKind, Write},
    num::NonZeroUsize,
};

use qubit_codec_text::{
    Charset, CharsetDecodeError, CharsetDecodeResult, CharsetEncodeError, CharsetEncodeResult,
    Codec,
};
use qubit_io_text::{
    AsciiCodec, CharsetCodec, CharsetTextWriter, CharsetWriteExt, CodingErrorPolicy, LineEnding,
    TextWrite, Utf8Codec,
};

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("write failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("flush failed"))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct NeedOutputCodec;

impl CharsetCodec for NeedOutputCodec {
    fn charset(&self) -> Charset {
        Charset::ASCII
    }
}

unsafe impl Codec for NeedOutputCodec {
    type Value = char;
    type Unit = u8;
    type DecodeError = CharsetDecodeError;
    type EncodeError = CharsetEncodeError;

    fn min_units_per_value(&self) -> NonZeroUsize {
        NonZeroUsize::new(1).expect("unit width is non-zero")
    }

    fn max_units_per_value(&self) -> NonZeroUsize {
        NonZeroUsize::new(2).expect("unit width is non-zero")
    }

    fn encode_len(&self, value: &char) -> NonZeroUsize {
        if *value == 'B' {
            NonZeroUsize::new(2).expect("unit width is non-zero")
        } else {
            NonZeroUsize::MIN
        }
    }

    unsafe fn decode(
        &mut self,
        _input: &[u8],
        _index: usize,
    ) -> CharsetDecodeResult<(char, NonZeroUsize)> {
        unreachable!("writer tests do not decode with NeedOutputCodec")
    }

    unsafe fn encode(
        &mut self,
        value: &char,
        output: &mut [u8],
        index: usize,
    ) -> CharsetEncodeResult<NonZeroUsize> {
        let required = self.encode_len(value).get();
        debug_assert!(
            index
                .checked_add(required)
                .is_some_and(|end| end <= output.len())
        );
        unsafe {
            // SAFETY: The caller guarantees that `required` units are writable
            // from `index`.
            *output.as_mut_ptr().add(index) = *value as u8;
            if required == 2 {
                *output.as_mut_ptr().add(index + 1) = *value as u8;
            }
        }
        Ok(self.encode_len(value))
    }
}

#[test]
fn test_write_utf8_text_to_byte_writer() -> std::io::Result<()> {
    let mut output = Vec::new();
    {
        let mut writer = CharsetTextWriter::new(&mut output, Utf8Codec, CodingErrorPolicy::Strict)
            .with_line_ending(LineEnding::CrLf);

        writer.write_char('A')?;
        writer.write_chars(&['B', 'C'])?;
        writer.write_line("中文")?;
        writer.flush()?;
    }

    assert_eq!("ABC中文\r\n".as_bytes(), output.as_slice());
    Ok(())
}

#[test]
fn test_write_rejects_unencodable_text_in_strict_mode() {
    let mut output = Vec::new();
    let mut writer = CharsetTextWriter::new(&mut output, AsciiCodec, CodingErrorPolicy::Strict);

    let error = writer
        .write_str("🙂")
        .expect_err("strict mode must reject unencodable text");
    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_write_chars_rejects_unencodable_text_in_strict_mode() {
    let mut output = Vec::new();
    let mut writer = CharsetTextWriter::new(&mut output, AsciiCodec, CodingErrorPolicy::Strict);

    let error = writer
        .write_chars(&['🙂'])
        .expect_err("strict mode must reject unencodable chars");
    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_write_replaces_unencodable_text_in_replace_mode() -> std::io::Result<()> {
    let mut output = Vec::new();
    {
        let mut writer =
            CharsetTextWriter::new(&mut output, AsciiCodec, CodingErrorPolicy::Replace);

        writer.write_str("🙂")?;
        writer.flush()?;
    }

    assert_eq!(b"?", output.as_slice());
    Ok(())
}

#[test]
fn test_accessors_and_into_inner() -> std::io::Result<()> {
    let output = Vec::new();
    let mut writer = CharsetTextWriter::new(output, AsciiCodec, CodingErrorPolicy::Strict);

    assert!(writer.get_ref().is_empty());
    writer.get_mut().extend_from_slice(b"prefix:");
    assert_eq!(LineEnding::Lf, writer.line_ending());
    writer.write_str("ascii")?;
    writer.flush()?;

    let output = writer.into_inner()?;
    assert_eq!(b"prefix:ascii", output.as_slice());
    Ok(())
}

#[test]
fn test_write_methods_propagate_underlying_errors() {
    let mut writer =
        CharsetTextWriter::with_capacity(FailingWriter, AsciiCodec, CodingErrorPolicy::Strict, 1);

    writer
        .write_char('x')
        .expect("first single-byte write should stay buffered");
    assert_eq!(
        ErrorKind::Other,
        writer
            .write_chars(&['x'])
            .expect_err("write_chars should flush buffered bytes before writing")
            .kind(),
    );
    assert_eq!(
        ErrorKind::Other,
        writer
            .write_line("x")
            .expect_err("write_line should report pending buffered write errors")
            .kind(),
    );
    assert_eq!(
        ErrorKind::Other,
        writer.flush().expect_err("flush must fail").kind(),
    );
}

#[test]
fn test_write_raises_buffer_to_single_character_max_output() -> std::io::Result<()> {
    let mut writer =
        CharsetTextWriter::with_capacity(Vec::new(), NeedOutputCodec, CodingErrorPolicy::Strict, 1);

    writer.write_char('B')?;
    let output = writer.into_inner()?;

    assert_eq!(b"BB", output.as_slice());
    Ok(())
}

#[test]
fn test_with_capacity_buffers_until_flush() -> std::io::Result<()> {
    let mut output = Vec::new();
    {
        let mut writer =
            CharsetTextWriter::with_capacity(&mut output, Utf8Codec, CodingErrorPolicy::Strict, 64);

        writer.write_str("buffered")?;
        assert!(writer.get_ref().is_empty());
        writer.flush()?;
    }

    assert_eq!(b"buffered", output.as_slice());
    Ok(())
}

#[test]
fn test_charset_write_ext_creates_stream_writer() -> std::io::Result<()> {
    let output = Vec::new();
    let mut writer = output.charset_text_writer(AsciiCodec, CodingErrorPolicy::Replace);

    writer.write_line("A🙂")?;
    let output = writer.into_inner()?;

    assert_eq!(b"A?\n", output.as_slice());
    Ok(())
}

#[test]
fn test_charset_write_ext_writes_one_shot_text() -> std::io::Result<()> {
    let mut output = Vec::new();

    output.write_str_with_charset("A🙂", AsciiCodec, CodingErrorPolicy::Replace)?;

    assert_eq!(b"A?", output.as_slice());
    Ok(())
}
