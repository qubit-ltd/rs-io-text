use qubit_text_io::{
    TextRead,
    TextWrite,
};

#[derive(Debug, Eq, PartialEq)]
struct ReadError;

struct FailingTextReader;

impl TextRead for FailingTextReader {
    type Error = ReadError;

    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        Err(ReadError)
    }

    fn read_chars(&mut self, _output: &mut Vec<char>, _max: usize) -> Result<usize, Self::Error> {
        Err(ReadError)
    }

    fn read_to_string(&mut self, _output: &mut String) -> Result<usize, Self::Error> {
        Err(ReadError)
    }
}

struct EmptyTextReader;

impl TextRead for EmptyTextReader {
    type Error = ReadError;

    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        Ok(None)
    }

    fn read_chars(&mut self, _output: &mut Vec<char>, _max: usize) -> Result<usize, Self::Error> {
        Ok(0)
    }

    fn read_to_string(&mut self, _output: &mut String) -> Result<usize, Self::Error> {
        Ok(0)
    }
}

#[derive(Debug, Eq, PartialEq)]
struct WriteError;

struct FailingTextWriter;

impl TextWrite for FailingTextWriter {
    type Error = WriteError;

    fn write_str(&mut self, _text: &str) -> Result<(), Self::Error> {
        Err(WriteError)
    }

    fn write_char(&mut self, _ch: char) -> Result<(), Self::Error> {
        Err(WriteError)
    }

    fn write_chars(&mut self, _chars: &[char]) -> Result<(), Self::Error> {
        Err(WriteError)
    }

    fn write_line(&mut self, _line: &str) -> Result<(), Self::Error> {
        Err(WriteError)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct FailOnSecondWrite {
    calls: usize,
}

impl TextWrite for FailOnSecondWrite {
    type Error = WriteError;

    fn write_str(&mut self, _text: &str) -> Result<(), Self::Error> {
        self.calls += 1;
        if self.calls == 2 {
            return Err(WriteError);
        }
        Ok(())
    }

    fn write_char(&mut self, ch: char) -> Result<(), Self::Error> {
        let mut buffer = [0_u8; 4];
        self.write_str(ch.encode_utf8(&mut buffer))
    }

    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        for ch in chars {
            self.write_char(*ch)?;
        }
        Ok(())
    }

    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.write_str(line)?;
        self.write_str(self.line_ending().as_str())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn test_read_chars_propagates_read_errors() {
    let mut reader = FailingTextReader;
    let mut chars = Vec::new();

    assert_eq!(Err(ReadError), reader.read_chars(&mut chars, 1));
    assert!(chars.is_empty());
}

#[test]
fn test_read_chars_stops_at_eof_without_appending() {
    let mut reader = EmptyTextReader;
    let mut chars = Vec::new();

    assert_eq!(Ok(0), reader.read_chars(&mut chars, 1));
    assert!(chars.is_empty());
}

#[test]
fn test_write_char_and_chars_propagate_write_errors() {
    let mut writer = FailingTextWriter;

    assert_eq!(Err(WriteError), writer.write_char('x'));
    assert_eq!(Err(WriteError), writer.write_chars(&['x']));
    assert_eq!(Err(WriteError), writer.write_line("line"));
}

#[test]
fn test_write_line_propagates_line_ending_errors() {
    let mut writer = FailOnSecondWrite { calls: 0 };

    assert_eq!(Err(WriteError), writer.write_line("line"));
    assert_eq!(2, writer.calls);
}
