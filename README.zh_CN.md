# Qubit IO Text

[![Rust CI](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io-text/coverage-badge.json)](https://qubit-ltd.github.io/rs-io-text/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io-text.svg?color=blue)](https://crates.io/crates/qubit-io-text)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

`qubit-io-text` 让 Rust 应用处理 Unicode 文本和文本行，并在流边界明确指定字节编码。
它提供：

- 面向同步 Unicode 文本的 `TextRead`、`TextLineRead` 和 `TextWrite`；
- 面向运行时无关异步文本操作的 `AsyncTextRead`、`AsyncTextLineRead` 和
  `AsyncTextWrite`；
- `StrTextReader`、`StringCharInput`、`StringCharOutput`、
  `InputTextReader`、`OutputTextWriter` 等字符串与字符流 adapter；
- 基于 Qubit 字节输入与输出的严格 UTF-8 便利 adapter；
- 基于 `qubit_io::Input<Item = u8>` / `Output<Item = u8>` 的同步 charset
  adapter；
- 基于 `AsyncInput<Item = u8>` / `AsyncOutput<Item = u8>`、运行时无关的
  `AsyncCharsetTextReader` 与 `AsyncCharsetTextWriter`；
- 来自 `qubit-codec-text` 的 charset policy、写入器 `LineEnding` 配置，以及读取器
  `LineEndingSet` 配置（默认识别 LF、CRLF 和 CR）。

Charset 算法保留在 `qubit-codec-text`；本 crate 只负责在流上驱动算法，不选择
异步运行时。

## 安装

```toml
[dependencies]
qubit-io-text = "0.5"
```

## 快速开始：编码一条 UTF-8 文本消息

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

## Charset 与异步依赖

Charset 和异步示例使用这些 crate 所拥有的类型，因此消费方应显式声明直接依赖：

```toml
[dependencies]
qubit-io-text = "0.5"
qubit-codec-text = "0.4"
qubit-io = "0.15"
```

## 运行时无关的异步 API

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

在依赖 codec trailer 或底层 flush 结果前，必须调用 `finish()` 或
`finish_async()`。finish 失败时 writer 仍由调用方持有，可检查或重试。finish 成功
后，使用 `into_parts()` 可在不执行额外 I/O 的情况下取回 owned output。

## 核心能力

| 领域 | API |
| --- | --- |
| Unicode 文本 trait | `TextRead`、`TextLineRead`、`TextWrite` |
| 内存文本 | `StrTextReader`、`StringTextReader`、`StringTextWriter` |
| 字符流 | `StringCharInput`、`StringCharOutput`、`InputTextReader`、`OutputTextWriter` |
| UTF-8 字节流 | 基于 `Input`/`Output` 的 `Utf8TextReader`、`Utf8TextWriter` |
| 同步 charset | `CharsetTextReader`、`CharsetTextWriter`、`CharsetReadExt`、`CharsetWriteExt` |
| 异步文本 | `AsyncTextRead`、`AsyncTextLineRead`、`AsyncTextWrite` |
| 异步 charset | `AsyncCharsetTextReader`、`AsyncCharsetTextWriter` |
| 策略 | `CharsetDecodePolicy`、`CharsetEncodePolicy`、`LineEnding`、`LineEndingSet` |

异步 charset reader 会在挂起或取消时保留未完成字符的编码字节。异步 writer 的
`write_chars_async` 和 `write_str_async` 每次只提交一个可报告的前缀；按返回数量推进
输入后再继续。`*_fully_async` 是便利循环，取消后无法可靠恢复其源位置；取消敏感场景
只使用单步 API，不要重试完整文本。

本 crate 不拥有 charset 算法，也不选择异步运行时。需要实战场景教程时，请参阅
[中文用户指南](doc/user_guide.zh_CN.md)或 [English user guide](doc/user_guide.md)；
全部公开项目请参阅 [API 文档](https://docs.rs/qubit-io-text)。

## 测试

```bash
# 使用默认 feature 集运行测试
cargo test

# 使用项目声明的全部 feature 运行测试
cargo test --all-features

# 运行项目 CI 检查
./ci-check.sh

# 检查代码覆盖率
./coverage.sh
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

本项目基于 Apache License 2.0 授权。完整许可证文本请参阅
[LICENSE](LICENSE)。

## 贡献

欢迎贡献。请遵循 Rust API 指南，及时更新公共 API 文档与测试，并在提交
Pull Request 前运行 `./align-ci.sh`格式化代码，运行`./ci-check.sh`对齐CI要求。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-io-text](https://github.com/qubit-ltd/rs-io-text)
