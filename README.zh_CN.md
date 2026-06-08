# Qubit IO Text

[![Rust CI](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io-text/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io-text/coverage-badge.json)](https://qubit-ltd.github.io/rs-io-text/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io-text.svg?color=blue)](https://crates.io/crates/qubit-io-text)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

面向 Rust 的文本 I/O trait 与 adapter。

## 概述

`qubit-io-text` 提供：

- 面向 Unicode 文本 source / sink 的 `TextRead`、`TextLineRead` 和 `TextWrite`；
- `StrTextReader`、`StringTextReader`、`StringTextWriter` 等内存 adapter；
- UTF-8 byte stream adapter：`Utf8TextReader` 和 `Utf8TextWriter`；
- 基于 `qubit-codec-text` 的 charset adapter：`CharsetTextReader` 和
  `CharsetTextWriter`；
- charset read/write 扩展 trait：`CharsetReadExt` 和 `CharsetWriteExt`；
- 通过 `LineEnding` 配置换行；
- 通过 `CodingErrorPolicy` 配置 strict 或 replacement 错误策略。

详细用法请参见[中文用户指南](doc/user_guide.zh_CN.md)。API 参考文档可在
[docs.rs](https://docs.rs/qubit-io-text) 查看。

## 设计目标

- **文本 I/O 层**：提供面向 stream 的文本抽象，但不承载 charset 算法本身。
- **Unicode 导向 Trait**：暴露 `char` 与 `str` 操作，而不是裸 byte 操作。
- **Codec 集成**：把 `qubit-codec-text` codec 适配到 byte stream。
- **错误策略显式**：在 adapter 边界明确 strict 和 replacement 行为。
- **Adapter 可组合**：支持内存、UTF-8 以及通用 charset-backed 文本 source/sink。

## 特性

### Text Trait

- **`TextRead`**：从文本 source 读取 Unicode scalar value。
- **`TextLineRead`**：按行读取文本并保留行终止符语义。
- **`TextWrite`**：向文本 sink 写入文本、字符和行。

### 内存 Adapter

- **`StrTextReader`**：从借用的 `str` 读取。
- **`StringTextReader`**：从 owned `String` 读取。
- **`StringTextWriter`**：写入 owned 或 borrowed `String` 缓冲区。

### Byte Stream Adapter

- **`Utf8TextReader` / `Utf8TextWriter`**：直接处理 UTF-8 byte stream。
- **`CharsetTextReader` / `CharsetTextWriter`**：基于 `qubit-codec-text`
  charset codec 的 adapter。
- **`CharsetReadExt` / `CharsetWriteExt`**：把 `Read` / `Write` stream
  包装为 charset text reader / writer 的扩展 trait。

### 配置

- **`LineEnding`**：控制 text writer 写出的行结束符。
- **`CodingErrorPolicy`**：把 strict 或 replacement 行为映射到 codec policy。

## 文档

- [中文用户指南](doc/user_guide.zh_CN.md)
- [API 文档](https://docs.rs/qubit-io-text)
- [英文 README](README.md)

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
qubit-io-text = "0.1"
```

## 快速开始

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

## API 参考

### Text Trait

| Trait | 用途 |
|-------|------|
| `TextRead` | 读取 Unicode scalar value 和字符串 |
| `TextLineRead` | 读取文本行 |
| `TextWrite` | 写入字符串、字符和文本行 |

### Adapter 类型

| 类型 | 用途 |
|------|------|
| `StrTextReader` | 借用 `str` reader |
| `StringTextReader` | Owned `String` reader |
| `StringTextWriter` | 基于 `String` 的 writer |
| `Utf8TextReader` / `Utf8TextWriter` | UTF-8 byte stream adapter |
| `CharsetTextReader` / `CharsetTextWriter` | 基于 charset codec 的 byte stream adapter |
| `BufferedCharsetTextReader` / `BufferedCharsetTextWriter` | buffered charset stream alias |
| `CharsetReadExt` / `CharsetWriteExt` | 面向 `Read` / `Write` 的扩展 trait |

### 配置类型

| 类型 | 用途 |
|------|------|
| `LineEnding` | 选择 `\n`、`\r\n` 或自定义行结束符 |
| `CodingErrorPolicy` | 选择 strict 或 replacement 行为 |

## 与文本 Codec 的关系

`qubit-io-text` 依赖 `qubit-codec-text` 提供 charset 算法。只需要缓冲区级编码解码时，
直接使用 `qubit-codec-text`；需要将这些 codec 适配到文本 I/O trait 或 byte stream 时，
使用 `qubit-io-text`。

## 性能考虑

Text trait 默认方法基于 `read_char` 和 `write_str`，以保持行为可预测。
Byte stream adapter 把 charset 工作委托给 `qubit-codec-text`，内存 adapter
则完全避免 byte decode。

## 测试与代码覆盖率

本项目通过 `tests/` 下的集成测试覆盖 text I/O 行为。

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行覆盖率报告
./coverage.sh

# 生成文本格式报告
./coverage.sh text

# 对齐 CI 要求
./align-ci.sh

# 运行 CI 检查（格式化、clippy、测试、覆盖率、安全审计）
RS_CI_SKIP_TOOLCHAIN_UPDATE=1 ./ci-check.sh
```

## 依赖项

运行时依赖保持很少：

- `qubit-codec-text` 提供 charset 算法和 codec policy 类型。
- `qubit-io` 提供通用 stream helper。

## 许可证

Copyright (c) 2026. Haixing Hu.

根据 Apache 许可证 2.0 版（"许可证"）授权；
除非遵守许可证，否则您不得使用此文件。
您可以在以下位置获取许可证副本：

    http://www.apache.org/licenses/LICENSE-2.0

除非适用法律要求或书面同意，否则根据许可证分发的软件
按"原样"分发，不附带任何明示或暗示的担保或条件。
有关许可证下的特定语言管理权限和限制，请参阅许可证。

完整的许可证文本请参阅 [LICENSE](LICENSE)。

## 贡献

欢迎贡献！请随时提交 Pull Request。

### 开发指南

- 保持 text I/O adapter 与 charset codec 算法分离。
- 覆盖 strict、replacement、EOF 和 partial-read 行为。
- 保持 README 示例与用户指南对齐。
- 提交 PR 前确保所有检查通过。

## 作者

**胡海星**

## 相关项目

- [qubit-codec-text](https://github.com/qubit-ltd/rs-codec-text)：缓冲区级 charset codec。
- [qubit-io](https://github.com/qubit-ltd/rs-io)：通用 `std::io` helper。
- Qubit 旗下的更多 Rust 库发布在 GitHub 组织
  [qubit-ltd](https://github.com/qubit-ltd)。

---

仓库地址：[https://github.com/qubit-ltd/rs-io-text](https://github.com/qubit-ltd/rs-io-text)
