# Qubit IO Text

[![Rust CI](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io-text/coverage-badge.json)](https://qubit-ltd.github.io/rs-io-text/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io-text.svg?color=blue)](https://crates.io/crates/qubit-io-text)
[![License](https://img.shields.io/crates/l/qubit-io-text.svg)](LICENSE)

面向 Rust 的文本 I/O trait 与 adapter。

`qubit-io-text` 提供：

- 面向 Unicode 文本 source / sink 的 `TextRead`、`TextLineRead` 和 `TextWrite`；
- `StrTextReader`、`StringTextReader`、`StringTextWriter` 等内存 adapter；
- UTF-8 byte stream adapter：`Utf8TextReader` 和 `Utf8TextWriter`；
- 基于 `qubit-codec-text` 的 charset adapter：`CharsetTextReader` 和
  `CharsetTextWriter`；
- 通过 `LineEnding` 配置换行；
- 通过 `CodingErrorPolicy` 配置 strict 或 replacement 错误策略。

详细用法请参见[中文用户指南](doc/user_guide.zh_CN.md)。API 参考文档可在
[docs.rs](https://docs.rs/qubit-io-text) 查看。

## 安装

```toml
[dependencies]
qubit-io-text = "0.1"
```

## 快速示例

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

## 与文本 Codec 的关系

`qubit-io-text` 依赖 `qubit-codec-text` 提供 charset 算法。只需要缓冲区级编码解码时，
直接使用 `qubit-codec-text`；需要将这些 codec 适配到文本 I/O trait 或 byte stream 时，
使用 `qubit-io-text`。

仓库地址：[https://github.com/qubit-ltd/rs-io-text](https://github.com/qubit-ltd/rs-io-text)
