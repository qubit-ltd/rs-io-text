# Qubit IO Text User Guide

[中文](user_guide.zh_CN.md) · [README](../README.md) ·
[API reference](https://docs.rs/qubit-io-text)

This guide is for Rust applications that should work with Unicode text and
lines instead of encoded bytes. It covers `qubit-io-text` 0.9, whose charset
algorithms come from `qubit-codec-text` and whose byte and character streams
come from `qubit-io`.

## Conceptual Model

The synchronous `TextReader`, `TextLineReader`, and `TextWriter` traits deal in
Rust `char` and `str`. Adapters then connect those traits to strings, character
streams, UTF-8 byte streams, or configurable character sets.

| Need | Main API | Input or output |
| --- | --- | --- |
| In-memory Unicode text | `StrTextReader`, `StringTextReader`, `StringTextWriter` | strings |
| Complete in-memory charset conversion | `CharsetStringEncoder`, `CharsetStringDecoder` | complete input; `Vec<C::Unit>` |
| Character streams | `InputTextReader`, `OutputTextWriter` | `CharInput`, `CharOutput` |
| Strict UTF-8 bytes | `Utf8TextReader`, `Utf8TextWriter` | `ByteInput`, `ByteOutput` |
| Synchronous charset conversion | `CharsetTextReader`, `CharsetTextWriter` | `ByteInput`, `ByteOutput` |
| Runtime-neutral async text | `AsyncTextReader`, `AsyncTextLineReader`, `AsyncTextWriter` | Unicode text |
| Runtime-neutral async conversion | `AsyncCharsetTextReader`, `AsyncCharsetTextWriter`, `AsyncUtf8TextReader`, `AsyncUtf8TextWriter` | `AsyncByteInput`, `AsyncByteOutput` |

`TextReader::read_to_string` appends to its destination and returns the number of
Unicode scalar values appended, not UTF-8 bytes. `TextWriter::write_line` adds
the configured `LineEnding`.

## Scenario: Preserve Unicode Text on a Byte Stream

Suppose an application receives a UTF-8 message, adds a CRLF-terminated header,
and must produce valid bytes for another component. The workflow selects a
codec and error policy once, writes text, finishes the codec lifecycle, then
reads the same bytes back as text.

## Installation

```toml
[dependencies]
qubit-io-text = "0.9"
qubit-codec-text = "^0.9"
qubit-io = "0.19"
```

This guide uses `Utf8Codec` and `AsyncByteOutput`, which are owned by
`qubit-codec-text` and `qubit-io`; Rust consumers must declare those crates as
direct dependencies.

## Complete In-Memory Conversion

`CharsetStringEncoder` and `CharsetStringDecoder` convert a complete input held
in memory. Use them for closed inputs; use the stream adapters for incremental
input, ongoing streams, or when codec state must remain available between
chunks. The codec determines the encoded unit type `C::Unit`, so the encoder
returns `Vec<C::Unit>`; only a byte codec such as `Utf8Codec` returns `Vec<u8>`.
The convenience `new` constructors use replacement policies by default. This
example selects `report()` explicitly so malformed or unmappable text returns
an error. The encoder's `with_policy` can fail during construction, while the
decoder's returns directly.
Each conversion runs a complete codec lifecycle, including reset and finish.
If `encode_str` returns an error, its locally accumulated output is not returned;
this does not promise rollback of encoder state. `decode_to_string_into` restores
the destination string's original length on `Err`, but does not roll back decoder
state or cover panics. The slice-writing `encode_str_into` may leave an encoded
prefix in the caller's output when it fails.

```rust
use qubit_codec_text::CharsetDecodePolicy;
use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::Utf8Codec;
use qubit_io_text::CharsetStringDecoder;
use qubit_io_text::CharsetStringEncoder;

pub fn convert_memory() -> Result<(), Box<dyn std::error::Error>> {
    let mut encoder = CharsetStringEncoder::with_policy(
        Utf8Codec, CharsetEncodePolicy::report(),
    )?;
    let bytes = encoder.encode_str("A中🙂")?;
    assert_eq!("A中🙂".as_bytes(), bytes.as_slice());
    let mut decoder = CharsetStringDecoder::with_policy(
        Utf8Codec, CharsetDecodePolicy::report(),
    );
    assert_eq!("A中🙂", decoder.decode_to_string(&bytes)?);
    Ok(())
}
```

## Core Workflow

```rust
use std::io::Cursor;

use qubit_codec_text::CharsetDecodePolicy;
use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::Utf8Codec;
use qubit_io_text::CharsetTextReader;
use qubit_io_text::CharsetTextWriter;
use qubit_io_text::LineEnding;
use qubit_io_text::TextReader;
use qubit_io_text::TextWriter;

fn main() -> std::io::Result<()> {
    let mut bytes = Vec::new();
    let mut writer = CharsetTextWriter::new(&mut bytes, Utf8Codec, CharsetEncodePolicy::report())
        .with_line_ending(LineEnding::CrLf);
    writer.write_line("subject: status")?;
    writer.write_str("中文")?;
    writer.finish()?;
    drop(writer);
    let mut reader = CharsetTextReader::new(Cursor::new(bytes), Utf8Codec, CharsetDecodePolicy::report());
    let mut text = String::new();
    let chars = reader.read_to_string(&mut text)?;
    assert_eq!(19, chars);
    assert_eq!("subject: status\r\n中文", text);
    Ok(())
}
```

Once encoder finalization succeeds, later text writes are rejected even if
delivery or flush fails. Retry `finish()` on the same writer to complete
delivery; do not replay the original text. An encoder-finalization error does
not imply a generally safe retry protocol. `CharsetReadExt` and `CharsetWriteExt` offer
construction and one-shot helpers when a persistent adapter is unnecessary.

## Policies, Lines, and UTF-8

Choose `CharsetDecodePolicy` for malformed input and `CharsetEncodePolicy` for
unencodable output. Each policy can report, ignore, or replace with its own
configured replacement value, and is chosen during adapter construction.
For configuration supplied at runtime, use a fallible constructor such as
`CharsetTextWriter::try_new`; for example, `AsciiCodec` cannot encode
`CharsetEncodePolicy::replace('中')`, so this policy is rejected during
construction.

```rust
use qubit_codec_text::{AsciiCodec, CharsetEncodePolicy};
use qubit_io_text::CharsetTextWriter;

let result = CharsetTextWriter::try_new(
    Vec::<u8>::new(),
    AsciiCodec,
    CharsetEncodePolicy::replace('中'),
);
assert!(result.is_err());
```

`LineEnding::Lf`, `LineEnding::CrLf`, and `LineEnding::Cr` control what
`write_line` appends. Built-in text readers accept LF, CRLF, and CR by default
and preserve the complete terminator. Use `LineEndingSet` with a reader's
`with_line_endings` method to select a custom set. If the stream is known to be
UTF-8, `Utf8TextReader` and `Utf8TextWriter` provide strict convenience
wrappers over the same byte-stream boundary.

## Async Workflow

The runtime-neutral `AsyncTextReader`, `AsyncTextLineReader`, and
`AsyncTextWriter` traits define asynchronous text operations. Charset and UTF-8
adapters implement the applicable traits, and constructing an adapter performs
no I/O.

```rust
use qubit_io::AsyncByteOutput;
use qubit_codec_text::{CharsetEncodePolicy, Utf8Codec};
use qubit_io_text::{
    AsyncCharsetTextWriter,
    LineEnding,
};

async fn write_document<O>(output: O) -> std::io::Result<O>
where
    O: AsyncByteOutput + Unpin,
{
    let mut writer = AsyncCharsetTextWriter::new(
        output,
        Utf8Codec,
        CharsetEncodePolicy::report(),
    )
    .with_line_ending(LineEnding::CrLf);
    writer.write_line_fully_async("subject: status").await?;
    writer.finish_async().await?;
    let (output, pending) = writer.into_parts();
    debug_assert!(pending.is_empty());
    Ok(output)
}
```

The asynchronous reader retains incomplete encoded characters across suspension
and cancellation. `write_chars_async` and `write_str_async` commit one
reportable prefix and return its count; resume with the suffix. The
`*_fully_async` convenience loops are not cancellation-safe: after
cancellation their source position cannot be recovered reliably. Use the
single-step APIs for cancellation-sensitive protocols.

On successful return, `write_str_async` counts consumed UTF-8 bytes, while
`write_chars_async` counts consumed `char` elements. Advance only the
corresponding source suffix. The [AsyncTextWriter example](https://docs.rs/qubit-io-text/latest/qubit_io_text/trait.AsyncTextWriter.html#examples)
shows both loops and explicit finalization; the [AsyncTextLineReader example](https://docs.rs/qubit-io-text/latest/qubit_io_text/trait.AsyncTextLineReader.html#examples)
shows line reads that preserve terminators.

## Send Futures and Threaded Tasks

Runtime-neutral APIs do not select an executor, and runtime neutrality does not
mean every generic future can move between threads. The
[`AsyncTextReader`](https://docs.rs/qubit-io-text/latest/qubit_io_text/trait.AsyncTextReader.html)
and
[`AsyncTextWriter`](https://docs.rs/qubit-io-text/latest/qubit_io_text/trait.AsyncTextWriter.html)
traits document that their method futures are not required to be `Send`.
Adding `Send` to a generic reader or writer bound therefore does not establish
a `Send` future. Keep generic code local to a task unless its concrete future
bounds are known.

A concrete `AsyncUtf8TextReader<I>` can provide a `Send` future when its owned
input satisfies the needed bounds. Here `async move` owns the input, and the
`'static` bound is needed for this example's task handoff; borrowed local async
calls do not universally require `'static`. `Send` does not imply cancellation
safety: consult the concrete adapter's cancellation behavior before retrying.

```rust
use std::future::Future;
use std::io;
use qubit_io::AsyncByteInput;
use qubit_io_text::AsyncUtf8TextReader;

pub fn read_owned<I>(input: I)
    -> impl Future<Output = io::Result<String>> + Send + 'static
where
    I: AsyncByteInput + Unpin + Send + 'static,
{
    async move {
        let mut reader = AsyncUtf8TextReader::new(input);
        let mut text = String::new();
        reader.read_to_string_async(&mut text).await?;
        Ok(text)
    }
}
```

## Errors and Diagnostics
- To identify this crate's bounded-append error, inspect the source of an existing `io::Error` and downcast it to `TextAppendLimitError`; do not classify every `InvalidData` error as an append limit. The getter reports the original configured UTF-8 byte limit. A rejected line is discarded before its limit error is reported; if discarding is interrupted by an input error, resume a line read on the same reader, which reports that original limit once after recovery. This type does not authorize retrying arbitrary decode failures or roll back consumed input. Character reads and one-shot conversions retain their own documented behavior.

```rust
use qubit_io_text::TextAppendLimitError;

fn append_limit(error: &std::io::Error) -> Option<usize> {
    error.get_ref()?
        .downcast_ref::<TextAppendLimitError>()
        .map(TextAppendLimitError::max_append_len)
}
```

- `CharsetDecodePolicy::report()` reports malformed encoded input and incomplete
  encoded tails at EOF. `CharsetEncodePolicy::report()` reports Unicode
  characters that the selected codec cannot encode. Use the corresponding
  `ignore()` or `replace(...)` policy only when intentional.
- Once encoder finalization succeeds, the writer rejects new text even if
  output delivery or flush returns an error. Retain that writer and retry
  `finish` to continue delivery without finalizing again; do not replay the
  original text. An encoder-finalization error does not imply transactional
  rollback or general retry safety.
  This retry guidance concerns normally returned I/O errors. After a caught
  inner I/O panic, async recovery has no indeterminate-delivery marker; see
  [Recovering pending bytes](#recovering-pending-bytes) before retrying or replaying.
- `CharsetStringDecoder::decode_to_string_into` appends on success and retains
  the destination's original contents on `Err`. This does not roll back decoder
  state or cover panics; streaming readers have separate prefix-retention rules.
- Synchronous text operations are not transactional. A failing read may leave an
  appended prefix in the destination and may already have consumed input. A
  failing write may already have accepted or delivered a prefix; retrying the
  complete string or line can duplicate text. These errors do not report the
  consumed prefix. Concrete APIs can provide stronger guarantees; in
  particular, the append-limit rollback rules below still apply. Handle flush
  errors explicitly when buffered delivery matters.
- Buffered readers can decode ahead of the requested character or line. With a
  reporting policy, malformed bytes later in the same decode window can make a
  read fail before an earlier valid character or complete line is returned.
  For example, with `ok\n` followed by `0xff`, a read may return `ok\n` or
  report `InvalidData` first, depending on how the input is chunked. Text already
  appended to the destination and decoded characters that have not been
  registered as readable are separate states. An error guarantees neither
  delivery of every valid prefix nor input rollback or safe retry; a smaller
  buffer is not a recovery protocol. Applications that need record isolation
  should frame encoded records according to their encoding and protocol before
  decoding them. Do not split arbitrary encoded input on LF bytes.
- One-shot `read_to_string_with_charset` and
  `read_to_string_with_charset_limited` return no decoded prefix or temporary
  decoder/unread state on failure, even though input may have advanced or been
  read ahead. One-shot `write_str_with_charset` may already have delivered a
  prefix; retrying the complete string can duplicate it. Its temporary writer
  may also attempt to drain pending output on drop, without reporting drop-time
  errors. Hold a `CharsetTextReader` or `CharsetTextWriter` explicitly when
  recovery state is needed, and apply the [reader](#recovering-unread-text) or
  [writer](#recovering-pending-bytes) `into_parts` rules below; neither
  these methods nor the decoded-output limit provide input rollback.
- Synchronous `BufferedTextWriter::write_str` and
  `CharsetTextWriter::write_str` report zero-based main-stage input positions in
  Rust `char` units, relative to that call, not UTF-8 bytes or the whole stream.
  The charset adapter also adjusts the charset source's input-character index;
  a generic encoder's opaque domain source keeps its own index semantics.
  `write_line` encodes its body and line ending as separate calls, so each has
  its own position origin.
- For `AsyncCharsetTextWriter`, single-step encoding errors use zero-based Rust
  `char` positions relative to the source slice or string suffix passed to that
  call. Complete-write errors use positions relative to the original complete
  call, regardless of internal chunks. A complete line encodes its body and
  appended line ending separately, each starting at zero. These indices are
  diagnostics, not safe retry cursors; complete writes can commit a prefix
  before returning an error or being cancelled.
- For this crate's `AsyncCharsetTextWriter` and `AsyncUtf8TextWriter`, successful
  encoder finalization also makes empty string and character-slice writes return
  `InvalidInput`, through both inherent and `AsyncTextWriter` methods. Open
  writers accept empty input without I/O. After finalization, omit empty writes
  or check for empty input before calling; retry `finish_async` for pending
  delivery instead of replaying text.
- `input()` can be positioned beyond text already buffered by an async reader.
- Synchronous buffered-writer `into_parts()` performs no I/O and returns `BufferedOutputRecovery`. `Pending` bytes were not reported as written; `Indeterminate` means a panicking inner write may have produced a prefix and must not be replayed automatically. Recovery does not call finish, and encoder lifecycle output not yet emitted may not be present in the returned buffer. Async writer `into_parts()` still returns its raw pending byte vector; `into_buffered_parts()` returns its existing buffer without copying, and `pending.readable()` is the pending window. Async recovery assumes normal inner I/O return contracts and has no indeterminate-delivery marker after a panic; see [Recovering pending bytes](#recovering-pending-bytes) before retrying or replaying pending data.
- Use `read_to_string_limited`, `read_to_string_limited_async`, or
  `read_to_string_with_charset_limited` when decoded output needs a bound.
  The limit applies to UTF-8 bytes appended by that call; exceeding it
  returns `InvalidData` and restores the destination string to its original
  length. The reader can still consume or read ahead in the underlying input,
  and the limit does not impose a raw input-byte bound.

  For `read_line_limited` and `read_line_limited_async`, the rejected line is
  discarded through its configured line ending or EOF before the original
  `InvalidData` limit error is returned. If an input error interrupts discarding,
  it is returned first and the reader retains recovery state. Continue with a
  line-reading method on the same reader; it resumes discarding with the original
  limit and line-ending configuration, then reports the original `InvalidData`
  once. The next line read starts at the next record.

- An asynchronous line read cancelled before the append limit is exceeded can
  leave a successfully decoded prefix in its destination. Resume that read on
  the same reader without appending the prefix again; the limit counts bytes
  appended by each call, not the whole logical line. Once a line exceeds its
  limit, the destination is rolled back. Cancellation while discarding the
  rejected record also leaves recovery state for the next line-reading call. For
  input `abcdef\nok\n` with limit `4`, the rejected `abcdef\n` record is
  discarded, `InvalidData` is reported, and the next line read returns `ok\n`.
  This recovery does not apply to character reads or reader `into_parts`; see
  [Recovering unread text](#recovering-unread-text) for that separate handoff.

### Recovering pending bytes

This consumes a writer already held by the caller when it must stop using it.
It performs no finish or I/O. Prefer finishing normally; after an error, only
handle bytes from `pending.readable()` when the output protocol makes that safe.
Never replay a prefix already delivered. For `AsyncUtf8TextWriter`, call
`writer.into_inner().into_buffered_parts()`. Callers that prefer a `Vec<u8>` can
keep using `into_parts()`.

Recovery assumes that inner I/O calls obey their normal return contracts. The
asynchronous buffer does not track poisoning or indeterminate delivery after
an inner I/O panic. If such a panic is caught, the returned pending bytes may
overlap a prefix already delivered. Reconcile external side effects before
retrying or replaying them. `pending.readable()` identifies the logical window
in the buffer, but its presence alone does not prove that these bytes were not
delivered before a panic. These panic limits do not change retry behavior after
normally returned I/O errors or the reported progress retained across
cancellation.

```rust
use qubit_codec_text::CharsetCodec;
use qubit_io::AsyncByteOutput;
use qubit_io::Buffer;
use qubit_io_text::AsyncCharsetTextWriter;

fn recover_pending<O, C>(writer: AsyncCharsetTextWriter<O, C>) -> (O, Buffer<u8>)
where
    O: AsyncByteOutput,
    C: CharsetCodec<Unit = u8>,
{
    writer.into_buffered_parts()
}
```

### Recovering unread text

Use reader `into_parts()` when the application must take ownership of text that
the reader has already read ahead. The character already returned (`a` below)
belongs to the caller and is not part of the remaining state. Process
`pending_chars` first, then `unread_bytes.readable()` followed by bytes still in
the underlying input; a multibyte character can cross that boundary, so keep
those byte fragments together and decode them as one stream.

This example is limited to known valid UTF-8 with no additional codec state.
For a general charset, retain the returned decoder and continue driving it only
while its lifecycle permits; it may already be started, and a finalized decoder
cannot be reopened by decomposition. Do not put the fragments in a new buffered
reader that may lazily reset a codec. Decomposition does not roll back decoding
errors or resume the discard state for a rejected line. Keep the original
reader for those cases.

```rust
use std::io;
use std::io::Cursor;
use std::io::Read;

use qubit_io_text::TextReader;
use qubit_io_text::Utf8TextReader;

fn main() -> io::Result<()> {
    let mut reader = Utf8TextReader::with_capacity(
        Cursor::new("abc🙂tail".as_bytes().to_vec()), 4,
    );
    assert_eq!(Some('a'), reader.read_char()?);
    let mut parts = reader.into_parts();
    assert_eq!(['b', 'c'], parts.pending_chars.as_slice());
    assert_eq!([0xf0], parts.unread_bytes.readable());
    let mut remaining: String = parts.pending_chars.into_iter().collect();
    let mut bytes = parts.unread_bytes.readable().to_vec();
    parts.input.read_to_end(&mut bytes)?;
    let tail = std::str::from_utf8(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    remaining.push_str(tail);
    assert_eq!("bc🙂tail", remaining);
    Ok(())
}
```

## Troubleshooting and Best Practices

| Symptom | Check first |
| --- | --- |
| A line ending is unexpected | Configure `LineEndingSet` on the reader, or `LineEnding` on the writer, before calling the line operation. |
| Decoding rejects data | Confirm the codec and intentionally choose `CharsetDecodePolicy::report()` or `CharsetDecodePolicy::replace(...)`, including its replacement value. |
| Output seems incomplete | Finish the charset writer; flushing alone does not finish the codec. |
| Retrying after a failed write duplicates text | A synchronous error does not report its accepted prefix; an async single-step method reports successful progress. Resume async writes from the returned suffix, and use framing or idempotency where needed. |

Import `qubit_io_text::prelude::*` when its grouped text traits and adapters
are convenient. Import codecs from `qubit-codec-text`, because this crate does
not own charset algorithms.

## Further Reading

- [README](../README.md) and [中文 README](../README.zh_CN.md)
- [中文用户指南](user_guide.zh_CN.md)
- [API reference](https://docs.rs/qubit-io-text)

## Resuming an interrupted line read

Use this pattern only after the caller has decided that resuming the read is
safe. `line` must contain only the prefix of the current record that has not
yet been delivered; it must not contain arbitrary initial text or a record
already returned to the caller. A false return describes this call's progress,
so EOF can still leave a retained prefix to deliver. The example propagates
input and decoding errors unchanged and does not retry them. Once `Some` is
returned, the destination has been taken and is empty; another call at EOF
returns `None`.

```rust
use std::io;

use qubit_io::AsyncByteInput;
use qubit_io_text::AsyncUtf8TextReader;

pub async fn resume_line<I>(
    reader: &mut AsyncUtf8TextReader<I>,
    line: &mut String,
) -> io::Result<Option<String>>
where
    I: AsyncByteInput + Unpin,
{
    let appended = reader.read_line_async(line).await?;
    if appended || !line.is_empty() {
        Ok(Some(std::mem::take(line)))
    } else {
        Ok(None)
    }
}
```
