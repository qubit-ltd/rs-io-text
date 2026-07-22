# Qubit IO Text

[![Rust CI](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io-text/coverage-badge.json)](https://qubit-ltd.github.io/rs-io-text/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io-text.svg?color=blue)](https://crates.io/crates/qubit-io-text)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

面向 Rust 的 Unicode 文本 trait，以及同步/异步 charset adapter。

`qubit-io-text` 提供：

- 面向同步 Unicode 文本的 `TextRead`、`TextLineRead` 和 `TextWrite`；
- `StrTextReader`、`StringCharInput`、`StringCharOutput`、
  `InputTextReader`、`OutputTextWriter` 等字符串与字符流 adapter；
- 基于 Qubit 字节输入与输出的严格 UTF-8 便利 adapter；
- 基于 `qubit_io::Input<Item = u8>` / `Output<Item = u8>` 的同步 charset
  adapter；
- 基于 `AsyncInput<Item = u8>` / `AsyncOutput<Item = u8>`、运行时无关的
  `AsyncCharsetTextReader` 与 `AsyncCharsetTextWriter`；
- 显式的 `CodingErrorPolicy` 与 `LineEnding` 配置。

Charset 算法保留在 `qubit-codec-text`；本 crate 只负责在流上驱动算法，不选择
异步运行时。

## 安装

```toml
[dependencies]
qubit-io-text = "0.3"
```

## 同步示例

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

## 异步示例

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

在依赖 codec trailer 或底层 flush 结果前，必须调用 `finish()` 或
`finish_async()`。消费型转换失败后需要取回 writer 并重试时，应使用
`try_into_output()` 或 `try_into_output_async()`。

## API 地图

| 领域 | API |
| --- | --- |
| Unicode 文本 trait | `TextRead`、`TextLineRead`、`TextWrite` |
| 内存文本 | `StrTextReader`、`StringTextReader`、`StringTextWriter` |
| 字符流 | `StringCharInput`、`StringCharOutput`、`InputTextReader`、`OutputTextWriter` |
| UTF-8 字节流 | 基于 `Input`/`Output` 的 `Utf8TextReader`、`Utf8TextWriter` |
| 同步 charset | `CharsetTextReader`、`CharsetTextWriter`、`CharsetReadExt`、`CharsetWriteExt` |
| 异步 charset | `AsyncCharsetTextReader`、`AsyncCharsetTextWriter` |
| 策略 | `CodingErrorPolicy`、`LineEnding` |

异步 charset reader 会在挂起或取消时保留未完成字符的编码字节；异步 writer 会
保留尚未被底层接收的编码字节。但取消高层写操作时，文本前缀可能已经生效，因此除非
外层协议允许，否则不要盲目重试完整文本。

详细说明见[中文用户指南](doc/user_guide.zh_CN.md)和
[API 文档](https://docs.rs/qubit-io-text)。

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
