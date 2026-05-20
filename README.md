# Qubit Text IO

[![Rust CI](https://github.com/qubit-ltd/rs-text-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-text-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-text-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-text-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-text-io.svg?color=blue)](https://crates.io/crates/qubit-text-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Chinese Document](https://img.shields.io/badge/Document-Chinese-blue.svg)](README.zh_CN.md)

Text-oriented I/O traits and adapters for Rust.

## Overview

Qubit Text IO defines small traits for APIs that consume or produce Unicode
text without choosing the final byte encoding or storage destination. It is
intended to sit above byte-oriented `std::io::Read` / `std::io::Write` and
below higher-level Unicode processing such as segmentation, normalization,
locale-aware case folding, or display-width calculation.

Use this crate when you need:

- a `TextRead` abstraction for code that reads Unicode scalar values;
- a `TextLineRead` abstraction for line-oriented text consumers;
- a `TextWrite` abstraction for code that writes text to interchangeable sinks;
- configurable line endings for text writers;
- adapters for borrowed strings, owned strings, UTF-8 byte streams, and explicit
  `encoding_rs` encodings such as GBK or UTF-8;
- strict or replacement-based handling of malformed input and unencodable text.

For byte-stream helpers, binary codecs, and stream wrappers, use
[qubit-io](https://github.com/qubit-ltd/rs-io).

For detailed usage, examples, and API selection guidance, see the
[User Guide](doc/user_guide.md). API reference documentation is available on
[docs.rs](https://docs.rs/qubit-text-io).

## Installation

```toml
[dependencies]
qubit-text-io = "0.1"
```

## Quick Example

```rust
use qubit_text_io::{
    LineEnding,
    TextWrite,
    Utf8TextWriter,
};

let mut bytes = Vec::new();
let mut writer = Utf8TextWriter::new(&mut bytes).with_line_ending(LineEnding::CrLf);

writer.write_char('中')?;
writer.write_str("abc")?;
writer.write_line("done")?;
writer.flush()?;

assert_eq!("中abcdone\r\n".as_bytes(), bytes.as_slice());

# Ok::<(), std::io::Error>(())
```

## Main Capabilities

### Text Traits

| Trait | Purpose |
| --- | --- |
| `TextRead` | Reads Unicode scalar values and remaining text |
| `TextLineRead` | Reads one line at a time while preserving line terminators |
| `TextWrite` | Writes Unicode text to a sink without exposing byte encoding |

These traits let business logic depend on text semantics instead of byte
streams. For example, a formatter can write to a UTF-8 file, a GBK file, a
`String`, or a custom database sink through the same `TextWrite` boundary.

### Adapters

| Adapter | Purpose |
| --- | --- |
| `StrTextReader` | Reads text from a borrowed `&str` |
| `StringTextReader` | Reads text from an owned `String` |
| `Utf8TextReader` | Streams UTF-8 text from a byte reader |
| `EncodedTextReader` | Decodes a bounded byte reader with an explicit encoding |
| `StringTextWriter` | Writes text into a borrowed `String` with line-ending config |
| `Utf8TextWriter` | Writes text as UTF-8 bytes |
| `EncodedTextWriter` | Encodes text with an explicit encoding |

### Coding Error Policy

`CodingErrorPolicy` controls how encoded adapters handle malformed input bytes
or text that cannot be represented by the target encoding:

| Policy | Behavior |
| --- | --- |
| `Strict` | Return an error on malformed or unencodable data |
| `Replace` | Use the replacement behavior provided by `encoding_rs` |

### Line Endings

`LineEnding` configures `TextWrite::write_line`:

| Variant | Bytes / text |
| --- | --- |
| `LineEnding::Lf` | `\n` |
| `LineEnding::CrLf` | `\r\n` |
| `LineEnding::Cr` | `\r` |

## Prelude

`qubit_text_io::prelude` re-exports the core traits, adapters, and small value
types used by most callers.

```rust
use qubit_text_io::prelude::*;
```

## Crate Boundary

`qubit-text-io` is intentionally limited to text-oriented I/O traits and
adapters. It does not try to provide full Unicode text processing.

This crate does not implement:

- grapheme cluster segmentation;
- Unicode normalization;
- locale-aware collation or case folding;
- display-width calculation;
- filesystem path utilities;
- database-specific writers.

Use crates such as `unicode-segmentation`, `unicode-normalization`,
`unicode-width`, or ICU4X when those semantics are needed.

## Runtime Dependencies

This crate depends on:

- Rust standard library;
- `encoding_rs` for explicit legacy and web-compatible encodings.

## Testing & Code Coverage

This project maintains test coverage for the public traits, adapters, line
ending behavior, UTF-8 handling, and encoded text behavior.

### Running Tests

```bash
# Run all tests
cargo test

# Run with coverage report
./coverage.sh

# Generate text format report
./coverage.sh text

# Run CI checks (format, clippy, test, coverage, audit)
./ci-check.sh
```

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

Contributions are welcome. Please feel free to submit a Pull Request.

### Development Guidelines

- Follow the Rust API guidelines.
- Keep text I/O concerns in `qubit-text-io`.
- Use `qubit-io` for byte-stream utilities.
- Keep encoding policy explicit at adapter boundaries.
- Maintain comprehensive test coverage.
- Document public APIs with examples when they clarify behavior.
- Ensure `./ci-check.sh` passes before submitting a PR.

## Author

**Haixing Hu**

## Related Projects

- [qubit-io](https://github.com/qubit-ltd/rs-io): byte-stream I/O traits,
  helpers, codecs, and wrappers for Rust.
- More Rust libraries from Qubit are published under the
  [qubit-ltd](https://github.com/qubit-ltd) organization on GitHub.

---

Repository: [https://github.com/qubit-ltd/rs-text-io](https://github.com/qubit-ltd/rs-text-io)
