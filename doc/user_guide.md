# Qubit IO Text User Guide

Use `qubit-io-text` when application code wants to work with Unicode text rather
than raw bytes. The crate provides small text traits plus adapters for strings,
UTF-8 byte streams, and byte-oriented `qubit-codec-text` charsets.

## Capability Map

| Area | API | Purpose |
| --- | --- | --- |
| Traits | `TextRead`, `TextLineRead`, `TextWrite` | abstract Unicode text sources and sinks |
| In-memory readers | `StrTextReader`, `StringTextReader` | read text from borrowed or owned strings |
| In-memory writer | `StringTextWriter` | write text into a borrowed `String` with line-ending policy |
| UTF-8 streams | `Utf8TextReader`, `Utf8TextWriter` | adapt `Read` and `Write` byte streams as UTF-8 text |
| Charset streams | `CharsetTextReader`, `CharsetTextWriter` | adapt byte-oriented `qubit-codec-text` codecs |
| Policy values | `LineEnding`, `CodingErrorPolicy` | configure line endings and malformed/unmappable handling |

## Installation

```toml
[dependencies]
qubit-io-text = "0.1"
```

## Writing Text

`TextWrite` is implemented for `String`, `StringTextWriter`, `Utf8TextWriter`,
and `CharsetTextWriter`.

```rust
use qubit_io_text::TextWrite;

let mut output = String::new();
output.write_line("first")?;
output.write_str("second")?;

assert_eq!("first\nsecond", output);
# Ok::<(), std::convert::Infallible>(())
```

Use `StringTextWriter` when line endings must be explicit:

```rust
use qubit_io_text::{
    LineEnding,
    StringTextWriter,
    TextWrite,
};

let mut output = String::new();
let mut writer = StringTextWriter::new(&mut output).with_line_ending(LineEnding::CrLf);

writer.write_line("first")?;

assert_eq!("first\r\n", output);
# Ok::<(), std::convert::Infallible>(())
```

## UTF-8 Byte Streams

Use `Utf8TextWriter` and `Utf8TextReader` when the byte stream is always UTF-8.

```rust
use std::io::Cursor;

use qubit_io_text::{
    TextRead,
    TextWrite,
    Utf8TextReader,
    Utf8TextWriter,
};

let mut bytes = Vec::new();
Utf8TextWriter::new(&mut bytes).write_str("a中")?;

let mut reader = Utf8TextReader::from_read(Cursor::new(bytes));
let mut text = String::new();
reader.read_to_string(&mut text)?;

assert_eq!("a中", text);
# Ok::<(), std::io::Error>(())
```

## Charset Adapters

`CharsetTextWriter` and `CharsetTextReader` accept byte-oriented codecs from
`qubit-codec-text`, such as `AsciiCodec`, `Latin1Codec`, `Utf8Codec`,
`Utf16ByteCodec`, and `Utf32ByteCodec`.

```rust
use qubit_io_text::{
    CharsetTextWriter,
    CodingErrorPolicy,
    TextWrite,
    Utf8Codec,
};

let mut bytes = Vec::new();
let mut writer = CharsetTextWriter::new(&mut bytes, Utf8Codec, CodingErrorPolicy::Strict);

writer.write_str("hello")?;
writer.flush()?;

assert_eq!(b"hello", bytes.as_slice());
# Ok::<(), std::io::Error>(())
```

Strict mode reports malformed input or unmappable output. Replacement mode uses
the replacement behavior provided by `qubit-codec-text`.

```rust
use std::io::Cursor;

use qubit_io_text::{
    CharsetTextReader,
    CodingErrorPolicy,
    TextRead,
    Utf8Codec,
};

let mut reader = CharsetTextReader::new(
    Cursor::new(vec![0xFF]),
    Utf8Codec,
    CodingErrorPolicy::Replace,
)?;
let mut output = String::new();

reader.read_to_string(&mut output)?;
assert_eq!("\u{FFFD}", output);
# Ok::<(), std::io::Error>(())
```

## Reading Lines

Readers that implement `TextLineRead` append the line terminator when one is
present, matching standard Rust line-reading behavior.

```rust
use qubit_io_text::{
    StrTextReader,
    TextLineRead,
};

let mut reader = StrTextReader::new("first\nsecond");
let mut line = String::new();

assert!(reader.read_line(&mut line)?);
assert_eq!("first\n", line);
# Ok::<(), std::convert::Infallible>(())
```

## Choosing the Right Layer

- Use `qubit-codec-text` for buffer-level charset conversion.
- Use `qubit-io-text` when text needs to flow through reader or writer traits.
- Use `qubit-io` for generic byte stream helpers that are not text-specific.
