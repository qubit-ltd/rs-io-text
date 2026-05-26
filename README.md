# Qubit IO Text

[![Rust CI](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io-text/coverage-badge.json)](https://qubit-ltd.github.io/rs-io-text/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io-text.svg?color=blue)](https://crates.io/crates/qubit-io-text)
[![License](https://img.shields.io/crates/l/qubit-io-text.svg)](LICENSE)

Text-oriented I/O traits and adapters for Rust.

`qubit-io-text` provides:

- `TextRead`, `TextLineRead`, and `TextWrite` traits for Unicode text sources and sinks;
- in-memory adapters such as `StrTextReader`, `StringTextReader`, and `StringTextWriter`;
- UTF-8 byte stream adapters: `Utf8TextReader` and `Utf8TextWriter`;
- charset adapters backed by `qubit-codec-text`: `CharsetTextReader` and
  `CharsetTextWriter`;
- configurable line endings through `LineEnding`;
- strict or replacement error policy through `CodingErrorPolicy`.

Detailed usage is documented in the [user guide](doc/user_guide.md). API
reference documentation is available on [docs.rs](https://docs.rs/qubit-io-text).

## Installation

```toml
[dependencies]
qubit-io-text = "0.1"
```

## Quick Example

```rust
use qubit_io_text::{
    CharsetTextWriter,
    CodingErrorPolicy,
    LineEnding,
    TextWrite,
    Utf8Codec,
};

let mut bytes = Vec::new();
let mut writer = CharsetTextWriter::new(&mut bytes, Utf8Codec, CodingErrorPolicy::Strict)
    .with_line_ending(LineEnding::CrLf);

writer.write_line("hello")?;
writer.write_str("中文")?;
writer.flush()?;

assert_eq!("hello\r\n中文".as_bytes(), bytes.as_slice());
# Ok::<(), std::io::Error>(())
```

## Relationship to Text Codecs

`qubit-io-text` depends on `qubit-codec-text` for charset algorithms. Use
`qubit-codec-text` directly when you only need buffer-level encoding and
decoding; use `qubit-io-text` when those codecs should be adapted to text I/O
traits or byte streams.

Repository: [https://github.com/qubit-ltd/rs-io-text](https://github.com/qubit-ltd/rs-io-text)
