# Qubit IO Text

[![Rust CI](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io-text/coverage-badge.json)](https://qubit-ltd.github.io/rs-io-text/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io-text.svg?color=blue)](https://crates.io/crates/qubit-io-text)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Chinese Document](https://img.shields.io/badge/Document-Chinese-blue.svg)](README.zh_CN.md)

Unicode text traits and synchronous/asynchronous charset adapters for Rust.

`qubit-io-text` provides:

- `TextRead`, `TextLineRead`, and `TextWrite` for synchronous Unicode text;
- string and character-stream adapters such as `StrTextReader`,
  `StringCharInput`, `StringCharOutput`, `InputTextReader`, and
  `OutputTextWriter`;
- strict UTF-8 convenience adapters over Qubit byte inputs and outputs;
- synchronous charset adapters over `qubit_io::Input<Item = u8>` and
  `qubit_io::Output<Item = u8>`;
- runtime-neutral `AsyncCharsetTextReader` and `AsyncCharsetTextWriter` over
  `AsyncInput<Item = u8>` and `AsyncOutput<Item = u8>`;
- explicit `CodingErrorPolicy` and `LineEnding` configuration.

Charset algorithms remain in `qubit-codec-text`; this crate drives them over
streams without selecting an async runtime.

## Installation

```toml
[dependencies]
qubit-io-text = "0.3"
```

## Synchronous Example

```rust
use qubit_codec_text::Utf8Codec;
use qubit_io_text::{
    CharsetTextWriter,
    CodingErrorPolicy,
    LineEnding,
    TextWrite,
};

let mut bytes = Vec::new();
let mut writer = CharsetTextWriter::new(
    &mut bytes,
    Utf8Codec,
    CodingErrorPolicy::Strict,
)
.with_line_ending(LineEnding::CrLf);

writer.write_line("hello")?;
writer.write_str("中文")?;
writer.finish()?;

assert_eq!("hello\r\n中文".as_bytes(), bytes.as_slice());
# Ok::<(), std::io::Error>(())
```

## Asynchronous Example

```rust
use qubit_io::AsyncOutput;
use qubit_codec_text::Utf8Codec;
use qubit_io_text::{
    AsyncCharsetTextWriter,
    CodingErrorPolicy,
};

async fn write_message<O>(output: O) -> std::io::Result<O>
where
    O: AsyncOutput<Item = u8> + Unpin,
{
    let mut writer = AsyncCharsetTextWriter::new(
        output,
        Utf8Codec,
        CodingErrorPolicy::Strict,
    );
    writer.write_line_async("hello").await?;
    writer.into_output_async().await
}
```

Call `finish()` or `finish_async()` before depending on codec trailers or the
underlying output flush. `into_output_async()` consumes the writer even on
failure; call `finish_async()` first when retrying delivery matters.

## API Map

| Area | API |
| --- | --- |
| Unicode text traits | `TextRead`, `TextLineRead`, `TextWrite` |
| In-memory text | `StrTextReader`, `StringTextReader`, `StringTextWriter` |
| Character streams | `StringCharInput`, `StringCharOutput`, `InputTextReader`, `OutputTextWriter` |
| UTF-8 byte streams | `Utf8TextReader`, `Utf8TextWriter` over `Input`/`Output` |
| Synchronous charsets | `CharsetTextReader`, `CharsetTextWriter`, `CharsetReadExt`, `CharsetWriteExt` |
| Asynchronous charsets | `AsyncCharsetTextReader`, `AsyncCharsetTextWriter` |
| Policy | `CodingErrorPolicy`, `LineEnding` |

Async charset readers retain an incomplete encoded character across suspension
and cancellation. Async writers retain encoded bytes until the output accepts
them. A cancelled high-level write may nevertheless have applied a prefix, so
do not blindly retry the whole text unless the surrounding protocol permits it.

See the [user guide](doc/user_guide.md) and
[API reference](https://docs.rs/qubit-io-text).

## Development

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

./align-ci.sh
RS_CI_SKIP_TOOLCHAIN_UPDATE=1 ./ci-check.sh
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).

Copyright (c) 2026 Haixing Hu.
