# Qubit IO Text 用户指南

[English](user_guide.md) · [README](../README.zh_CN.md) ·
[API 文档](https://docs.rs/qubit-io-text)

本指南面向需要处理 Unicode 文本和文本行、而不是已编码字节的 Rust 应用，适用于
`qubit-io-text` 0.3。Charset 算法来自 `qubit-codec-text`，字节流和字符流抽象来自
`qubit-io`。

## 概念模型

同步 `TextRead`、`TextLineRead` 与 `TextWrite` 直接处理 Rust 的 `char` 和 `str`。
Adapter 再将这些 trait 连接到字符串、字符流、UTF-8 字节流或可配置 charset。

| 需求 | 主要 API | 输入或输出 |
| --- | --- | --- |
| 内存中的 Unicode 文本 | `StrTextReader`、`StringTextReader`、`StringTextWriter` | 字符串 |
| 字符流 | `InputTextReader`、`OutputTextWriter` | `Input<Item = char>`、`Output<Item = char>` |
| 严格 UTF-8 字节 | `Utf8TextReader`、`Utf8TextWriter` | `Input<Item = u8>`、`Output<Item = u8>` |
| 同步 charset 转换 | `CharsetTextReader`、`CharsetTextWriter` | `Input<Item = u8>`、`Output<Item = u8>` |
| 运行时无关的异步转换 | `AsyncCharsetTextReader`、`AsyncCharsetTextWriter` | `AsyncInput<Item = u8>`、`AsyncOutput<Item = u8>` |

`TextRead::read_to_string` 将内容追加到目标并返回追加的 Unicode scalar value 数量，
而不是 UTF-8 字节数；`TextWrite::write_line` 会追加配置好的 `LineEnding`。

## 贯穿场景：在字节流上保持 Unicode 文本

假设应用收到一条 UTF-8 消息，添加一个以 CRLF 结束的 header 后，要将有效字节交给
另一个组件。流程是在构造时确定 codec 和错误策略，写入文本，完成 codec 生命周期，
再将同一批字节读回文本。

## 安装

```toml
[dependencies]
qubit-io-text = "0.3"
```

## 核心流程

```rust
use std::io::Cursor;

use qubit_codec_text::Utf8Codec;
use qubit_io_text::{
    CharsetTextReader,
    CharsetTextWriter,
    CodingErrorPolicy,
    LineEnding,
    TextRead,
    TextWrite,
};

let mut bytes = Vec::new();
let mut writer = CharsetTextWriter::new(
    &mut bytes,
    Utf8Codec,
    CodingErrorPolicy::Strict,
)
.with_line_ending(LineEnding::CrLf);
writer.write_line("subject: status")?;
writer.write_str("中文")?;
writer.finish()?;

let mut reader = CharsetTextReader::new(
    Cursor::new(bytes),
    Utf8Codec,
    CodingErrorPolicy::Strict,
);
let mut text = String::new();
let chars = reader.read_to_string(&mut text)?;
assert_eq!(19, chars);
assert_eq!("subject: status\r\n中文", text);
# Ok::<(), std::io::Error>(())
```

依赖 codec 自己的尾部输出或底层 sink 已 flush 前，必须调用 `finish()`。finish 失败时
writer 会被保留，调用方可以检查或重试。仅需一次转换时，可使用 `CharsetReadExt` 和
`CharsetWriteExt` 的构造与 one-shot helper，无需长期保存 adapter。

## 策略、换行与 UTF-8

`CodingErrorPolicy::Strict` 会报告畸形输入和无法编码的输出；
`CodingErrorPolicy::Replace` 使用 codec 的替换值。策略在 adapter 构造时选定，因此
不能在有状态 charset 流的中途改变。

`LineEnding::Lf`、`LineEnding::CrLf` 和 `LineEnding::Cr` 决定 `write_line` 追加的
内容。流确定为 UTF-8 时，可使用 `Utf8TextReader` 和 `Utf8TextWriter`，它们在相同的
字节流边界提供严格的便利包装。

## 异步流程

异步 API 仅位于 charset adapter；并非每一个同步文本 trait 都有异步对应版本。构造过程
不执行 I/O。

```rust
use qubit_io::AsyncOutput;
use qubit_codec_text::Utf8Codec;
use qubit_io_text::{
    AsyncCharsetTextWriter,
    CodingErrorPolicy,
    LineEnding,
};

async fn write_document<O>(output: O) -> std::io::Result<O>
where
    O: AsyncOutput<Item = u8> + Unpin,
{
    let mut writer = AsyncCharsetTextWriter::new(
        output,
        Utf8Codec,
        CodingErrorPolicy::Strict,
    )
    .with_line_ending(LineEnding::CrLf);
    writer.write_line_async("subject: status").await?;
    writer.finish_async().await?;
    let (output, pending) = writer.into_parts();
    debug_assert!(pending.is_empty());
    Ok(output)
}
```

异步 reader 会在挂起或取消时保留不完整编码字符。异步 writer 会保留 pending 编码字节，
但取消高层写入仍可能使输出中留下文本前缀；除非外层协议允许重复前缀，否则不要整体
重试字符串。

## 错误与诊断

- `Strict` 会报告畸形编码输入、不完整的 EOF 编码尾部和当前 codec 无法编码的 Unicode
  文本。
- `finish` 或 `finish_async` 失败后，writer 仍可用于重试。
- 异步 reader 的 `input()` 可能已越过其缓冲文本，`into_input()` 会丢弃该缓冲状态。
- `into_parts()` 不执行 I/O；在成功 finish 前调用它会明确放弃尚未发出的 codec
  lifecycle 输出。

## 排障与最佳实践

| 症状 | 首先检查 |
| --- | --- |
| 换行符不符合预期 | 调用 `write_line` 前在 writer 上配置 `LineEnding`。 |
| 解码拒绝数据 | 确认 codec，并有意选择 `Strict` 或 `Replace`。 |
| 输出似乎不完整 | finish charset writer；仅 flush 不会结束 codec。 |
| 异步重试导致文本重复 | 将之前的写入视为部分完成，采用 framing 或幂等机制。 |

需要组合导入文本 trait 和 adapter 时，可使用 `qubit_io_text::prelude::*`。Codec 应从
`qubit-codec-text` 导入，因为本 crate 不拥有 charset 算法。

## 延伸阅读

- [README](../README.zh_CN.md) 与 [English README](../README.md)
- [English user guide](user_guide.md)
- [API 文档](https://docs.rs/qubit-io-text)
