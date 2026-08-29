# Qubit IO Text

[![Rust CI](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io-text/coverage-badge.json)](https://qubit-ltd.github.io/rs-io-text/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io-text.svg?color=blue)](https://crates.io/crates/qubit-io-text)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

`qubit-io-text` lets Rust applications handle Unicode text and lines while
making byte encodings explicit at the stream boundary. It provides:

- `TextRead`, `TextLineRead`, and `TextWrite` for synchronous Unicode text;
- `AsyncTextRead`, `AsyncTextLineRead`, and `AsyncTextWrite` for runtime-neutral
  asynchronous text operations;
- string and character-stream adapters such as `StrTextReader`,
  `StringCharInput`, `StringCharOutput`, `InputTextReader`, and
  `OutputTextWriter`;
- strict UTF-8 convenience adapters over Qubit byte inputs and outputs;
- synchronous charset adapters over `qubit_io::Input<Item = u8>` and
  `qubit_io::Output<Item = u8>`;
- runtime-neutral `AsyncCharsetTextReader` and `AsyncCharsetTextWriter` over
  `AsyncInput<Item = u8>` and `AsyncOutput<Item = u8>`;
- charset policies from `qubit-codec-text`, `LineEnding` writer configuration,
  and `LineEndingSet` reader configuration (LF, CRLF, and CR by default).

Charset algorithms remain in `qubit-codec-text`; this crate drives them over
streams without selecting an async runtime.

## Installation

```toml
[dependencies]
qubit-io-text = "0.4"
```

## Quick Start: Encode a UTF-8 Text Message

```rust
use qubit_io_text::{
    LineEnding,
    TextWrite,
    Utf8TextWriter,
};

let mut bytes = Vec::new();
let mut writer = Utf8TextWriter::new(&mut bytes)
    .with_line_ending(LineEnding::CrLf);

writer.write_line("hello")?;
writer.write_str("中文")?;
writer.finish()?;

let (output, pending) = writer.into_parts();
assert!(pending.readable().is_empty());
assert_eq!("hello\r\n中文".as_bytes(), output.as_slice());
# Ok::<(), std::io::Error>(())
```

## Charset and Async Dependencies

Charset and async examples use types owned by these crates, so declare them
directly in the consuming package:

```toml
[dependencies]
qubit-io-text = "0.4"
qubit-codec-text = "0.4"
qubit-io = "0.15"
```

## Runtime-Neutral Async

```rust
use qubit_io::AsyncOutput;
use qubit_codec_text::{CharsetEncodePolicy, Utf8Codec};
use qubit_io_text::{
    AsyncCharsetTextWriter,
};

async fn write_message<O>(output: O) -> std::io::Result<O>
where
    O: AsyncOutput<Item = u8> + Unpin,
{
    let mut writer = AsyncCharsetTextWriter::new(
        output,
        Utf8Codec,
        CharsetEncodePolicy::report(),
    );
    writer.write_line_fully_async("hello").await?;
    writer.finish_async().await?;
    let (output, pending) = writer.into_parts();
    debug_assert!(pending.is_empty());
    Ok(output)
}
```

Call `finish()` or `finish_async()` before depending on codec trailers or the
underlying output flush. A failed finish retains the writer, so callers can
inspect or retry it. After a successful finish, `into_parts()` recovers the
owned output without performing further I/O.

## What It Provides

| Area | API |
| --- | --- |
| Unicode text traits | `TextRead`, `TextLineRead`, `TextWrite` |
| In-memory text | `StrTextReader`, `StringTextReader`, `StringTextWriter` |
| Character streams | `StringCharInput`, `StringCharOutput`, `InputTextReader`, `OutputTextWriter` |
| UTF-8 byte streams | `Utf8TextReader`, `Utf8TextWriter` over `Input`/`Output` |
| Synchronous charsets | `CharsetTextReader`, `CharsetTextWriter`, `CharsetReadExt`, `CharsetWriteExt` |
| Asynchronous text | `AsyncTextRead`, `AsyncTextLineRead`, `AsyncTextWrite` |
| Asynchronous charsets | `AsyncCharsetTextReader`, `AsyncCharsetTextWriter` |
| Policy | `CharsetDecodePolicy`, `CharsetEncodePolicy`, `LineEnding`, `LineEndingSet` |

Async charset readers retain an incomplete encoded character across suspension
and cancellation. `write_chars_async` and `write_str_async` commit one
reportable prefix and return its count; advance the source cursor before the
next call. The `*_fully_async` convenience loops are not cancellation-safe:
after cancellation their source position cannot be recovered reliably. Use
the single-step APIs for cancellation-sensitive protocols.

The crate does not own charset algorithms or select an async runtime. For a
scenario-led tutorial, see the [user guide](doc/user_guide.md) or
[中文用户指南](doc/user_guide.zh_CN.md); for every public item, see the
[API reference](https://docs.rs/qubit-io-text).

## Testing

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

# Project CI checks
./ci-check.sh

# Check code coverage
./coverage.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the
full license text.

## Contributing

Contributions are welcome. Please follow the Rust API guidelines, keep public
API documentation and tests current, and run `./align-ci.sh` to format code and
`./ci-check.sh` to satisfy CI requirements before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-io-text](https://github.com/qubit-ltd/rs-io-text)
