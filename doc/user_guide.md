# Qubit IO Text User Guide

[中文](user_guide.zh_CN.md) · [README](../README.md) ·
[API reference](https://docs.rs/qubit-io-text)

This guide is for Rust applications that should work with Unicode text and
lines instead of encoded bytes. It covers `qubit-io-text` 0.3, whose charset
algorithms come from `qubit-codec-text` and whose byte and character streams
come from `qubit-io`.

## Conceptual Model

The synchronous `TextRead`, `TextLineRead`, and `TextWrite` traits deal in
Rust `char` and `str`. Adapters then connect those traits to strings, character
streams, UTF-8 byte streams, or configurable character sets.

| Need | Main API | Input or output |
| --- | --- | --- |
| In-memory Unicode text | `StrTextReader`, `StringTextReader`, `StringTextWriter` | strings |
| Character streams | `InputTextReader`, `OutputTextWriter` | `Input<Item = char>`, `Output<Item = char>` |
| Strict UTF-8 bytes | `Utf8TextReader`, `Utf8TextWriter` | `Input<Item = u8>`, `Output<Item = u8>` |
| Synchronous charset conversion | `CharsetTextReader`, `CharsetTextWriter` | `Input<Item = u8>`, `Output<Item = u8>` |
| Runtime-neutral async conversion | `AsyncCharsetTextReader`, `AsyncCharsetTextWriter` | `AsyncInput<Item = u8>`, `AsyncOutput<Item = u8>` |

`TextRead::read_to_string` appends to its destination and returns the number of
Unicode scalar values appended, not UTF-8 bytes. `TextWrite::write_line` adds
the configured `LineEnding`.

## Scenario: Preserve Unicode Text on a Byte Stream

Suppose an application receives a UTF-8 message, adds a CRLF-terminated header,
and must produce valid bytes for another component. The workflow selects a
codec and error policy once, writes text, finishes the codec lifecycle, then
reads the same bytes back as text.

## Installation

```toml
[dependencies]
qubit-io-text = "0.3"
```

## Core Workflow

```rust
use std::io::Cursor;

use qubit_codec_text::Utf8Codec;
use qubit_io_text::{
    CharsetTextReader,
    CharsetTextWriter,
    CodingErrorPolicy,
    LineEnding,
    TextRead,
    TextWrite,
};

let mut bytes = Vec::new();
let mut writer = CharsetTextWriter::new(
    &mut bytes,
    Utf8Codec,
    CodingErrorPolicy::Strict,
)
.with_line_ending(LineEnding::CrLf);
writer.write_line("subject: status")?;
writer.write_str("中文")?;
writer.finish()?;

let mut reader = CharsetTextReader::new(
    Cursor::new(bytes),
    Utf8Codec,
    CodingErrorPolicy::Strict,
);
let mut text = String::new();
let chars = reader.read_to_string(&mut text)?;
assert_eq!(19, chars);
assert_eq!("subject: status\r\n中文", text);
# Ok::<(), std::io::Error>(())
```

Call `finish()` before depending on codec-owned trailing output or a flushed
underlying sink. A failed finish retains the writer, allowing a caller to
inspect the state or retry. `CharsetReadExt` and `CharsetWriteExt` offer
construction and one-shot helpers when a persistent adapter is unnecessary.

## Policies, Lines, and UTF-8

Choose `CodingErrorPolicy::Strict` to report malformed input and unencodable
output, or `CodingErrorPolicy::Replace` to use the codec replacement value.
The policy is chosen during adapter construction, so it cannot change midway
through a stateful charset stream.

`LineEnding::Lf`, `LineEnding::CrLf`, and `LineEnding::Cr` control what
`write_line` appends. If the stream is known to be UTF-8, `Utf8TextReader` and
`Utf8TextWriter` provide strict convenience wrappers over the same byte-stream
boundary.

## Async Workflow

Async APIs are limited to charset adapters; there is not an async counterpart
for every synchronous text trait. Construction performs no I/O.

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
    writer.write_line_async("subject: status").await?;
    writer.finish_async().await?;
    let (output, pending) = writer.into_parts();
    debug_assert!(pending.is_empty());
    Ok(output)
}
```

The asynchronous reader retains incomplete encoded characters across suspension
and cancellation. Async writers retain pending encoded bytes, but cancellation
of a high-level write can still leave a text prefix in the output; do not retry
the entire string unless the surrounding protocol permits duplicate prefixes.

## Errors and Diagnostics

- `Strict` reports malformed encoded input, incomplete encoded EOF tails, and
  Unicode text that the selected codec cannot encode.
- A failing `finish` or `finish_async` leaves the writer available for retry.
- `input()` can be positioned beyond text already buffered by an async reader;
  `into_input()` discards that buffered text state.
- `into_parts()` performs no I/O. Calling it before a successful finish
  explicitly abandons codec lifecycle output that has not yet been emitted.

## Troubleshooting and Best Practices

| Symptom | Check first |
| --- | --- |
| A line ending is unexpected | Configure `LineEnding` on the writer before calling `write_line`. |
| Decoding rejects data | Confirm the codec and use `Strict` or `Replace` intentionally. |
| Output seems incomplete | Finish the charset writer; flushing alone does not finish the codec. |
| An async retry duplicates text | Treat the previous write as partially applied and use framing or idempotency. |

Import `qubit_io_text::prelude::*` when its grouped text traits and adapters
are convenient. Import codecs from `qubit-codec-text`, because this crate does
not own charset algorithms.

## Further Reading

- [README](../README.md) and [中文 README](../README.zh_CN.md)
- [中文用户指南](user_guide.zh_CN.md)
- [API reference](https://docs.rs/qubit-io-text)
