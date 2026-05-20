# Qubit Text IO User Guide

Qubit Text IO is the text-oriented I/O crate in the Qubit Rust family. It
focuses on small text traits and adapters that let business logic operate on
Unicode text while adapter code handles byte encodings and concrete sinks.

For byte-level stream utilities, see
[qubit-io](https://github.com/qubit-ltd/rs-io).

## When to Use This Crate

Use `qubit-text-io` when your code works with Unicode text rather than raw
bytes. Typical examples include formatters, report generators, line-oriented
processors, configuration readers, and components that should write to multiple
text destinations without knowing their encodings.

Good fits:

- A formatter writes strings to a UTF-8 file, a GBK file, a `String`, or a
  custom logging sink.
- A parser reads lines from text sources that may have different external
  encodings.
- A component needs explicit line-ending behavior.
- A boundary should reject malformed input in strict mode.
- A migration tool needs replacement-mode decoding for legacy input.

Not a fit:

- Binary protocols and byte-level framing.
- Filesystem path management.
- Recursive directory operations.
- Full Unicode segmentation, normalization, collation, or display width.
- Database-specific persistence logic.

## Import Patterns

Import the traits and adapters you use directly:

```rust
use qubit_text_io::{
    TextWrite,
    Utf8TextWriter,
};
```

Use the prelude when a module primarily works with text I/O:

```rust
use qubit_text_io::prelude::*;
```

## TextWrite

`TextWrite` is the main abstraction for text producers. A producer can emit
Unicode text without choosing the final byte encoding.

```rust
use qubit_text_io::TextWrite;

fn write_report<W>(writer: &mut W) -> Result<(), W::Error>
where
    W: TextWrite,
{
    writer.write_line("Report")?;
    writer.write_str("status: ok")?;
    writer.flush()
}
```

The caller chooses the sink. It may be a `String`, a UTF-8 byte writer, an
encoded writer, a logger, or a database field adapter.

### Writing to a String

`String` implements `TextWrite` directly with LF line endings:

```rust
use qubit_text_io::TextWrite;

let mut output = String::new();

output.write_line("hello")?;
output.write_char('中')?;

assert_eq!("hello\n中", output);

# Ok::<(), std::convert::Infallible>(())
```

Use `StringTextWriter` when you need explicit line-ending configuration:

```rust
use qubit_text_io::{
    LineEnding,
    StringTextWriter,
    TextWrite,
};

let mut output = String::new();
let mut writer = StringTextWriter::new(&mut output).with_line_ending(LineEnding::CrLf);

writer.write_line("hello")?;
drop(writer);

assert_eq!("hello\r\n", output);

# Ok::<(), std::convert::Infallible>(())
```

### Writing UTF-8 Bytes

Use `Utf8TextWriter` when the destination is a `std::io::Write` sink that should
receive UTF-8 bytes:

```rust
use qubit_text_io::{
    TextWrite,
    Utf8TextWriter,
};

let mut bytes = Vec::new();
let mut writer = Utf8TextWriter::new(&mut bytes);

writer.write_line("hello")?;
writer.flush()?;

assert_eq!(b"hello\n", bytes.as_slice());

# Ok::<(), std::io::Error>(())
```

### Writing an Explicit Encoding

Use `EncodedTextWriter` when the destination byte stream needs a specific
encoding:

```rust
use encoding_rs::GBK;
use qubit_text_io::{
    CodingErrorPolicy,
    EncodedTextWriter,
    TextWrite,
};

let mut bytes = Vec::new();
let mut writer = EncodedTextWriter::new(&mut bytes, GBK, CodingErrorPolicy::Strict);

writer.write_str("中文")?;
writer.flush()?;

let (decoded, _, had_errors) = GBK.decode(bytes.as_slice());
assert!(!had_errors);
assert_eq!("中文", decoded);

# Ok::<(), std::io::Error>(())
```

## TextRead and TextLineRead

`TextRead` exposes Unicode scalar values and remaining text. `TextLineRead`
adds line-oriented reading while preserving line terminators.

```rust
use qubit_text_io::{
    StrTextReader,
    TextRead,
};

let mut reader = StrTextReader::new("a中🙂");

assert_eq!(Some('a'), reader.read_char()?);
assert_eq!(Some('中'), reader.read_char()?);
assert_eq!(Some('🙂'), reader.read_char()?);
assert_eq!(None, reader.read_char()?);

# Ok::<(), std::convert::Infallible>(())
```

### Reading Lines

`read_line` appends to the destination string and returns `false` only when EOF
is reached with no text appended.

```rust
use qubit_text_io::{
    StrTextReader,
    TextLineRead,
};

let mut reader = StrTextReader::new("first\r\nsecond");
let mut line = String::new();

assert!(reader.read_line(&mut line)?);
assert_eq!("first\r\n", line);

line.clear();
assert!(reader.read_line(&mut line)?);
assert_eq!("second", line);

line.clear();
assert!(!reader.read_line(&mut line)?);

# Ok::<(), std::convert::Infallible>(())
```

### Reading UTF-8 Bytes

Use `Utf8TextReader` for streaming UTF-8 byte input:

```rust
use std::io::Cursor;

use qubit_text_io::{
    TextLineRead,
    Utf8TextReader,
};

let input = Cursor::new("hello\n世界".as_bytes());
let mut reader = Utf8TextReader::from_read(input);
let mut line = String::new();

assert!(reader.read_line(&mut line)?);
assert_eq!("hello\n", line);

# Ok::<(), std::io::Error>(())
```

### Reading an Explicit Encoding

`EncodedTextReader` decodes all input during construction and then serves text
from memory. It is intended for bounded inputs.

```rust
use std::io::Cursor;

use encoding_rs::GBK;
use qubit_text_io::{
    CodingErrorPolicy,
    EncodedTextReader,
    TextRead,
};

let (bytes, _, had_errors) = GBK.encode("中文");
assert!(!had_errors);

let mut reader = EncodedTextReader::new(
    Cursor::new(bytes.into_owned()),
    GBK,
    CodingErrorPolicy::Strict,
)?;
let mut output = String::new();

reader.read_to_string(&mut output)?;
assert_eq!("中文", output);

# Ok::<(), std::io::Error>(())
```

## Coding Error Policies

`CodingErrorPolicy::Strict` rejects malformed input bytes and unencodable
output text:

```rust
use std::io::Cursor;

use encoding_rs::UTF_8;
use qubit_text_io::{
    CodingErrorPolicy,
    EncodedTextReader,
};

let error = EncodedTextReader::new(
    Cursor::new(vec![0xFF]),
    UTF_8,
    CodingErrorPolicy::Strict,
)
.expect_err("strict mode rejects malformed UTF-8");

assert_eq!(std::io::ErrorKind::InvalidData, error.kind());
```

`CodingErrorPolicy::Replace` accepts the same input and uses the replacement
behavior provided by `encoding_rs`.

## API Boundaries

This crate treats Rust `char` as a Unicode scalar value. That is not the same as
a user-perceived character. A single grapheme cluster can contain multiple
Unicode scalar values.

Use a specialized Unicode crate when you need:

- grapheme cluster boundaries;
- normalization;
- locale-aware case mapping;
- display width;
- collation.

Keep `qubit-text-io` for the I/O boundary: adapting external encodings and
text sinks/sources into Rust Unicode text.
