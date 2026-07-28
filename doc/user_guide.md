# Qubit IO Text User Guide

Use `qubit-io-text` when application code should operate on Unicode scalar
values and lines instead of encoded bytes. Charset codecs are supplied by
`qubit-codec-text`; byte and character streams are supplied by `qubit-io`.

## Installation

```toml
[dependencies]
qubit-io-text = "0.3"
```

## API Layers

| Layer | Main API | Underlying I/O |
| --- | --- | --- |
| Synchronous text traits | `TextRead`, `TextLineRead`, `TextWrite` | Unicode `char` and `str` |
| In-memory adapters | `StrTextReader`, `StringTextReader`, `StringTextWriter`, `StringCharInput`, `StringCharOutput` | strings |
| Character bridges | `InputTextReader`, `OutputTextWriter` | `Input<Item = char>`, `Output<Item = char>` |
| UTF-8 convenience | `Utf8TextReader`, `Utf8TextWriter` | `Input<Item = u8>`, `Output<Item = u8>` |
| Synchronous charsets | `CharsetTextReader`, `CharsetTextWriter` | `Input<Item = u8>`, `Output<Item = u8>` |
| Asynchronous charsets | `AsyncCharsetTextReader`, `AsyncCharsetTextWriter` | `AsyncInput<Item = u8>`, `AsyncOutput<Item = u8>` |

The crate currently exposes asynchronous APIs on charset adapters rather than
an async counterpart to every text trait.

## Unicode Text Traits

`TextRead::read_to_string` appends to the destination and returns the number of
Unicode scalar values appended, not the number of UTF-8 bytes.

```rust
use qubit_io_text::{
    StrTextReader,
    TextRead,
};

let mut reader = StrTextReader::new("中🙂");
let mut text = String::new();
let count = reader.read_to_string(&mut text)?;

assert_eq!(2, count);
assert_eq!(7, text.len());
assert_eq!("中🙂", text);
# Ok::<(), core::convert::Infallible>(())
```

`TextLineRead::read_line` appends a trailing newline when the input contains
one. `TextWrite::write_line` adds the writer's configured `LineEnding`.

```rust
use qubit_io_text::{
    LineEnding,
    StringTextWriter,
    TextWrite,
};

let mut output = String::new();
let mut writer =
    StringTextWriter::new(&mut output).with_line_ending(LineEnding::CrLf);
writer.write_line("first")?;

assert_eq!("first\r\n", output);
# Ok::<(), std::io::Error>(())
```

## Synchronous Charset Streams

`CharsetTextReader<I, C>` decodes any `I: Input<Item = u8>`.
`CharsetTextWriter<O, C>` encodes into any `O: Output<Item = u8>`.

Built-in codec families include `AsciiCodec`, `Latin1Codec`, `Utf8Codec`,
`Utf16ByteCodec`, and `Utf32ByteCodec`.

```rust
use std::io::Cursor;

use qubit_codec_text::Utf8Codec;
use qubit_io_text::{
    CharsetTextReader,
    CharsetTextWriter,
    CodingErrorPolicy,
    TextRead,
    TextWrite,
};

let mut bytes = Vec::new();
let mut writer =
    CharsetTextWriter::new(&mut bytes, Utf8Codec, CodingErrorPolicy::Strict);
writer.write_str("hello 中")?;
writer.finish()?;

let mut reader = CharsetTextReader::new(
    Cursor::new(bytes),
    Utf8Codec,
    CodingErrorPolicy::Strict,
);
let mut text = String::new();
reader.read_to_string(&mut text)?;
assert_eq!("hello 中", text);
# Ok::<(), std::io::Error>(())
```

`CharsetReadExt` and `CharsetWriteExt` provide construction and one-shot
helpers on `Input` and `Output` values:

```rust
use std::io::Cursor;

use qubit_codec_text::Utf8Codec;
use qubit_io_text::{
    CharsetReadExt,
    CharsetWriteExt,
    CodingErrorPolicy,
};

let mut input = Cursor::new(b"hello".to_vec());
let text = input.read_to_string_with_charset(
    Utf8Codec,
    CodingErrorPolicy::Strict,
)?;

let mut output = Vec::new();
output.write_str_with_charset(
    &text,
    Utf8Codec,
    CodingErrorPolicy::Strict,
)?;
# Ok::<(), std::io::Error>(())
```

## Asynchronous Charset Reader

`AsyncCharsetTextReader` owns its `AsyncInput`, decoder, and retained byte and
character buffers. Construction performs no I/O.

```rust
use qubit_io::AsyncInput;
use qubit_codec_text::Utf8Codec;
use qubit_io_text::{
    AsyncCharsetTextReader,
    CodingErrorPolicy,
};

async fn read_document<I>(input: I) -> std::io::Result<String>
where
    I: AsyncInput<Item = u8> + Unpin,
{
    let mut reader = AsyncCharsetTextReader::new(
        input,
        Utf8Codec,
        CodingErrorPolicy::Strict,
    );
    let mut text = String::new();
    reader.read_to_string_async(&mut text).await?;
    Ok(text)
}
```

Its operations are:

- `read_char_async`;
- `read_chars_async`;
- `read_to_string_async`;
- `read_line_async`.

The reader commits received bytes to retained storage before another suspension
point. Cancelling a read therefore does not lose an incomplete encoded
character. `input()` may already be positioned beyond buffered bytes;
`into_input()` discards all buffered text state.

## Asynchronous Charset Writer

`AsyncCharsetTextWriter` owns its `AsyncOutput`, stateful encoder, and pending
encoded bytes.

```rust
use qubit_io::AsyncOutput;
use qubit_codec_text::Utf8Codec;
use qubit_io_text::{
    AsyncCharsetTextWriter,
    CodingErrorPolicy,
    LineEnding,
};

async fn write_document<O>(output: O) -> std::io::Result<O>
where
    O: AsyncOutput<Item = u8> + Unpin,
{
    let mut writer = AsyncCharsetTextWriter::new(
        output,
        Utf8Codec,
        CodingErrorPolicy::Strict,
    )
    .with_line_ending(LineEnding::CrLf);
    writer.write_line_async("header").await?;
    writer.write_str_async("body").await?;
    writer.finish_async().await?;
    let (output, pending) = writer.into_parts();
    debug_assert!(pending.is_empty());
    Ok(output)
}
```

The writer also provides `write_char_async`, `write_chars_async`, and
`flush_async`. Flushing drains encoded bytes but does not finish the encoder.
Finishing emits codec-owned trailing output, drains it, and flushes the
underlying output. A failed `finish_async()` retains pending state and can be
retried. After a successful `finish` or `finish_async`, text writers expose
`into_parts()` to recover the owned output and any encoded bytes still pending
without performing I/O. Calling `into_parts()` before finishing explicitly
abandons encoder lifecycle output that has not yet been emitted. `OutputTextWriter`
is a thin adapter: its `into_inner()` also performs no implicit flush.

Pending encoded bytes survive suspension and cancellation. However, a cancelled
high-level write may already have applied a text prefix. Do not retry the whole
string blindly unless the surrounding protocol makes duplicate prefixes safe.

## Error Policy and EOF

`CodingErrorPolicy::Strict` reports malformed input, incomplete encoded EOF
tails, and unencodable output. `CodingErrorPolicy::Replace` substitutes the
codec's replacement value.

Policy is part of adapter construction so error handling cannot change halfway
through a stateful charset stream.

## UTF-8 Convenience Adapters

`Utf8TextReader` and `Utf8TextWriter` are strict UTF-8 convenience wrappers
over `Input<Item = u8>` and `Output<Item = u8>`. They delegate to the same
buffered charset state machines as `CharsetTextReader` and
`CharsetTextWriter`, so standard-library streams work through Qubit's blanket
adapters without defining a second core I/O boundary.
