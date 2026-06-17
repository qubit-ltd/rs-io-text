# Qubit IO Text

[![Rust CI](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io-text/coverage-badge.json)](https://qubit-ltd.github.io/rs-io-text/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io-text.svg?color=blue)](https://crates.io/crates/qubit-io-text)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Chinese Document](https://img.shields.io/badge/Document-Chinese-blue.svg)](README.zh_CN.md)

Text-oriented I/O traits and adapters for Rust.

## Overview

`qubit-io-text` provides:

- `TextRead`, `TextLineRead`, and `TextWrite` traits for Unicode text sources and sinks;
- in-memory adapters such as `StrTextReader`, `StringTextReader`, and `StringTextWriter`;
- UTF-8 byte stream adapters: `Utf8TextReader` and `Utf8TextWriter`;
- charset adapters backed by `qubit-codec-text`: `CharsetTextReader` and
  `CharsetTextWriter`;
- charset read/write extension traits: `CharsetReadExt` and `CharsetWriteExt`;
- configurable line endings through `LineEnding`;
- strict or replacement error policy through `CodingErrorPolicy`.

Detailed usage is documented in the [user guide](doc/user_guide.md). API
reference documentation is available on [docs.rs](https://docs.rs/qubit-io-text).

## Design Goals

- **Text I/O Layer**: provide stream-facing text abstractions without owning
  charset algorithms.
- **Unicode-Oriented Traits**: expose `char` and `str` operations instead of raw
  byte operations.
- **Codec Integration**: adapt `qubit-codec-text` codecs to byte streams.
- **Explicit Error Policy**: keep strict and replacement behavior visible at the
  adapter boundary.
- **Composable Adapters**: support in-memory, UTF-8, and generic charset-backed
  text sources and sinks.

## Features

### Text Traits

- **`TextRead`**: reads Unicode scalar values from a text source.
- **`TextLineRead`**: reads line-oriented text while preserving line terminators.
- **`TextWrite`**: writes text, characters, and lines to a text sink.

### In-Memory Adapters

- **`StrTextReader`**: reads from borrowed `str`.
- **`StringTextReader`**: reads from owned `String`.
- **`StringTextWriter`**: writes into an owned or borrowed `String` buffer.

### Byte Stream Adapters

- **`Utf8TextReader` / `Utf8TextWriter`**: direct UTF-8 byte stream adapters.
- **`CharsetTextReader` / `CharsetTextWriter`**: adapters backed by
  `qubit-codec-text` charset codecs.
- **`CharsetReadExt` / `CharsetWriteExt`**: extension traits that wrap
  `Read` and `Write` streams as charset text readers and writers.

### Configuration

- **`LineEnding`**: controls line endings written by text writers.
- **`CodingErrorPolicy`**: maps strict or replacement behavior to codec policies.

## Documentation

- [User Guide](doc/user_guide.md)
- [API Reference](https://docs.rs/qubit-io-text)
- [Chinese README](README.zh_CN.md)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
qubit-io-text = "0.1"
```

## Quick Start

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

### `TextRead::read_to_string` Count

`TextRead::read_to_string` appends the remaining text and returns the number of
Unicode scalar values appended. This is different from `std::io::Read`, whose
string helper reports bytes.

```rust
use qubit_io_text::{StrTextReader, TextRead};

let mut reader = StrTextReader::new("中🙂");
let mut text = String::new();

let count = reader.read_to_string(&mut text)?;

assert_eq!(2, count);
assert_eq!(7, text.len());
assert_eq!("中🙂", text);
# Ok::<(), core::convert::Infallible>(())
```

## API Reference

### Text Traits

| Trait | Purpose |
|-------|---------|
| `TextRead` | Read Unicode scalar values and strings |
| `TextLineRead` | Read text lines |
| `TextWrite` | Write strings, characters, and lines |

### Adapter Types

| Type | Purpose |
|------|---------|
| `StrTextReader` | Borrowed `str` reader |
| `StringTextReader` | Owned `String` reader |
| `StringTextWriter` | `String`-backed writer |
| `Utf8TextReader` / `Utf8TextWriter` | UTF-8 byte stream adapters |
| `CharsetTextReader` / `CharsetTextWriter` | Charset codec backed byte stream adapters |
| `BufferedCharsetTextReader` / `BufferedCharsetTextWriter` | Buffered charset stream aliases |
| `CharsetReadExt` / `CharsetWriteExt` | Extension traits for `Read` and `Write` |

### Configuration Types

| Type | Purpose |
|------|---------|
| `LineEnding` | Select `\n`, `\r\n`, or custom line endings |
| `CodingErrorPolicy` | Select strict or replacement behavior |

## Relationship to Text Codecs

`qubit-io-text` depends on `qubit-codec-text` for charset algorithms. Use
`qubit-codec-text` directly when you only need buffer-level encoding and
decoding; use `qubit-io-text` when those codecs should be adapted to text I/O
traits or byte streams.

## Performance Considerations

Text trait default methods build on `read_char` and `write_str` for predictable
behavior. Byte stream adapters delegate charset work to `qubit-codec-text`, while
in-memory adapters avoid byte decoding entirely.

## Testing & Code Coverage

This project keeps text I/O behavior covered by integration tests under
`tests/`.

### Running Tests

```bash
# Run all tests
cargo test

# Run with coverage report
./coverage.sh

# Generate text format report
./coverage.sh text

# Align code with CI requirements
./align-ci.sh

# Run CI checks (format, clippy, test, coverage, audit)
RS_CI_SKIP_TOOLCHAIN_UPDATE=1 ./ci-check.sh
```

## Dependencies

Runtime dependencies are intentionally small:

- `qubit-codec-text` provides charset algorithms and codec policy types.
- `qubit-io` provides generic stream helpers.

## License

Copyright (c) 2026. Haixing Hu.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

See [LICENSE](LICENSE) for the full license text.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development Guidelines

- Keep text I/O adapters separate from charset codec algorithms.
- Cover strict, replacement, EOF, and partial-read behavior.
- Keep README examples aligned with the user guide.
- Ensure all checks pass before submitting a PR.

## Author

**Haixing Hu**

## Related Projects

- [qubit-codec-text](https://github.com/qubit-ltd/rs-codec-text): buffer-level
  charset codecs.
- [qubit-io](https://github.com/qubit-ltd/rs-io): generic `std::io` helpers.
- More Rust libraries from Qubit are available under the
  [qubit-ltd](https://github.com/qubit-ltd) GitHub organization.

---

Repository: [https://github.com/qubit-ltd/rs-io-text](https://github.com/qubit-ltd/rs-io-text)
