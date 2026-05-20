# Qubit Text IO

[![Rust CI](https://github.com/qubit-ltd/rs-text-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-text-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-text-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-text-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-text-io.svg?color=blue)](https://crates.io/crates/qubit-text-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

面向 Rust 的文本 I/O trait 与适配器。

## 概述

Qubit Text IO 提供一组小型 trait，让消费或生成 Unicode 文本的代码不必
提前决定最终字节编码或存储目的地。它位于字节层的
`std::io::Read` / `std::io::Write` 之上，也位于更高层的 Unicode 分词、
规范化、区域相关大小写转换和显示宽度计算之下。

当你需要以下能力时，可以使用本 crate：

- 用 `TextRead` 表达“读取 Unicode scalar value”的输入边界；
- 用 `TextLineRead` 表达按行读取文本的消费边界；
- 用 `TextWrite` 表达向可替换文本目的地写入文本的输出边界；
- 为文本 writer 配置换行符；
- 使用借用字符串、拥有字符串、UTF-8 字节流，以及基于 `encoding_rs` 的显式编码
  适配器，例如 GBK 或 UTF-8；
- 对非法输入字节或不可编码文本采用严格报错或替换策略。

如果需要字节流工具、二进制编码和 stream wrapper，请使用
[qubit-io](https://github.com/qubit-ltd/rs-io)。

更多用法、示例和 API 选择建议见[用户指南](doc/user_guide.zh_CN.md)。
API 参考文档可在 [docs.rs](https://docs.rs/qubit-text-io) 查看。

## 安装

```toml
[dependencies]
qubit-text-io = "0.1"
```

## 快速示例

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

## 主要能力

### 文本 Trait

| Trait | 用途 |
| --- | --- |
| `TextRead` | 读取 Unicode scalar value 和剩余文本 |
| `TextLineRead` | 按行读取文本，并保留行终止符 |
| `TextWrite` | 向文本目的地写入 Unicode 文本，不暴露字节编码 |

这些 trait 让业务逻辑依赖文本语义，而不是字节流。例如，一个 formatter 可以通过
同一个 `TextWrite` 边界写入 UTF-8 文件、GBK 文件、`String` 或自定义数据库字段。

### 适配器

| 适配器 | 用途 |
| --- | --- |
| `StrTextReader` | 从借用的 `&str` 读取文本 |
| `StringTextReader` | 从拥有的 `String` 读取文本 |
| `Utf8TextReader` | 从字节 reader 流式读取 UTF-8 文本 |
| `EncodedTextReader` | 使用显式编码解码有界字节 reader |
| `StringTextWriter` | 将文本写入借用的 `String`，并支持换行配置 |
| `Utf8TextWriter` | 将文本写为 UTF-8 字节 |
| `EncodedTextWriter` | 使用显式编码写出文本 |

### 编码错误策略

`CodingErrorPolicy` 控制 encoded adapter 如何处理非法输入字节或目标编码无法表达的
文本：

| 策略 | 行为 |
| --- | --- |
| `Strict` | 遇到非法或不可编码数据时报错 |
| `Replace` | 使用 `encoding_rs` 提供的替换行为 |

### 换行符

`LineEnding` 用于配置 `TextWrite::write_line`：

| 变体 | 文本 |
| --- | --- |
| `LineEnding::Lf` | `\n` |
| `LineEnding::CrLf` | `\r\n` |
| `LineEnding::Cr` | `\r` |

## Prelude

`qubit_text_io::prelude` 会重导出多数调用方常用的核心 trait、adapter 和小型值类型。

```rust
use qubit_text_io::prelude::*;
```

## Crate 边界

`qubit-text-io` 只关注文本导向的 I/O trait 与 adapter，不试图提供完整 Unicode
文本处理能力。

本 crate 不实现：

- grapheme cluster 分割；
- Unicode normalization；
- 区域相关的排序或大小写转换；
- 显示宽度计算；
- 文件系统路径工具；
- 数据库专用 writer。

如果需要这些语义，请使用 `unicode-segmentation`、`unicode-normalization`、
`unicode-width` 或 ICU4X 等专门 crate。

## 运行时依赖

本 crate 依赖：

- Rust 标准库；
- `encoding_rs`，用于显式 legacy 与 Web 兼容编码。

## 测试与覆盖率

本项目为公开 trait、adapter、换行行为、UTF-8 处理和 encoded text 行为维护测试覆盖。

### 运行测试

```bash
# 运行全部测试
cargo test

# 运行覆盖率报告
./coverage.sh

# 生成文本格式报告
./coverage.sh text

# 运行 CI 检查（格式、clippy、测试、覆盖率、audit）
./ci-check.sh
```

## 许可证

Copyright (c) 2026. Haixing Hu.

本项目基于 Apache License, Version 2.0 授权。你可以在遵守该许可证的前提下使用本项目。
许可证文本可从以下地址获取：

    http://www.apache.org/licenses/LICENSE-2.0

除非适用法律要求或书面同意，本项目按“原样”分发，不提供任何明示或暗示担保。
详见 [LICENSE](LICENSE)。

## 贡献

欢迎提交 Pull Request。

### 开发约定

- 遵循 Rust API guidelines。
- 将文本 I/O 相关能力保留在 `qubit-text-io`。
- 字节流工具使用 `qubit-io`。
- 在 adapter 边界显式表达编码策略。
- 维护完整测试覆盖。
- 公共 API 在示例有助于理解时补充示例。
- 提交 PR 前确保 `./ci-check.sh` 通过。

## 作者

**Haixing Hu**

## 相关项目

- [qubit-io](https://github.com/qubit-ltd/rs-io)：Rust 字节流 I/O trait、工具、
  codec 和 wrapper。
- Qubit 的更多 Rust 库发布在 [qubit-ltd](https://github.com/qubit-ltd) 组织下。

---

Repository: [https://github.com/qubit-ltd/rs-text-io](https://github.com/qubit-ltd/rs-text-io)
