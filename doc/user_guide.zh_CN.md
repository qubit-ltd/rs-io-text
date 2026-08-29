# Qubit IO Text 用户指南

[English](user_guide.md) · [README](../README.zh_CN.md) ·
[API 文档](https://docs.rs/qubit-io-text)

本指南面向需要处理 Unicode 文本和文本行、而不是已编码字节的 Rust 应用，适用于
`qubit-io-text` 0.4。Charset 算法来自 `qubit-codec-text`，字节流和字符流抽象来自
`qubit-io`。

## 概念模型

同步 `TextRead`、`TextLineRead` 与 `TextWrite` 直接处理 Rust 的 `char` 和 `str`；
异步 `AsyncTextRead`、`AsyncTextLineRead` 与 `AsyncTextWrite` 提供对应的运行时无关
操作。
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
qubit-io-text = "0.4"
qubit-codec-text = "0.4"
qubit-io = "0.15"
```

本指南使用由 `qubit-codec-text` 和 `qubit-io` 提供的 `Utf8Codec` 与
`AsyncOutput`；Rust 消费方必须将这两个 crate 声明为直接依赖。

## 核心流程

```rust
use std::io::Cursor;

use qubit_codec_text::{
    CharsetDecodePolicy,
    CharsetEncodePolicy,
    Utf8Codec,
};
use qubit_io_text::{
    CharsetTextReader,
    CharsetTextWriter,
    LineEnding,
    TextRead,
    TextWrite,
};

let mut bytes = Vec::new();
let mut writer = CharsetTextWriter::new(
    &mut bytes,
    Utf8Codec,
    CharsetEncodePolicy::report(),
)
.with_line_ending(LineEnding::CrLf);
writer.write_line("subject: status")?;
writer.write_str("中文")?;
writer.finish()?;

let mut reader = CharsetTextReader::new(
    Cursor::new(bytes),
    Utf8Codec,
    CharsetDecodePolicy::report(),
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

畸形输入使用 `CharsetDecodePolicy`，无法编码的输出使用
`CharsetEncodePolicy`。两者均可独立报告、忽略或使用自定义替换字符，并在 adapter
构造时选定。

`LineEnding::Lf`、`LineEnding::CrLf` 和 `LineEnding::Cr` 决定 `write_line` 追加的
内容。内置文本读取器默认识别 LF、CRLF 和 CR，并在返回字符串中保留完整终止符。
需要自定义策略时，可在读取器上调用 `with_line_endings` 配置 `LineEndingSet`；写入器
则使用 `LineEnding` 配置输出终止符。流确定为 UTF-8 时，可使用 `Utf8TextReader` 和
`Utf8TextWriter`，它们在相同的字节流边界提供严格的便利包装。

## 异步流程

异步 charset adapter 实现了异步文本 trait。构造过程不执行 I/O。

```rust
use qubit_io::AsyncOutput;
use qubit_codec_text::{CharsetEncodePolicy, Utf8Codec};
use qubit_io_text::{
    AsyncCharsetTextWriter,
    LineEnding,
};

async fn write_document<O>(output: O) -> std::io::Result<O>
where
    O: AsyncOutput<Item = u8> + Unpin,
{
    let mut writer = AsyncCharsetTextWriter::new(
        output,
        Utf8Codec,
        CharsetEncodePolicy::report(),
    )
    .with_line_ending(LineEnding::CrLf);
    writer.write_line_fully_async("subject: status").await?;
    writer.finish_async().await?;
    let (output, pending) = writer.into_parts();
    debug_assert!(pending.is_empty());
    Ok(output)
}
```

异步 reader 会在挂起或取消时保留不完整编码字符。异步 writer 的
`write_chars_async` 与 `write_str_async` 每次提交一个可报告的前缀，并返回其长度；使用
剩余后缀继续写入。`*_fully_async` 便利循环取消后无法可靠恢复源位置；取消敏感场景只
使用单步 API，不要整体重试字符串。

## 错误与诊断

- `Strict` 会报告畸形编码输入、不完整的 EOF 编码尾部和当前 codec 无法编码的 Unicode
  文本。
- `finish` 或 `finish_async` 失败后，writer 仍可用于重试。
- 异步 reader 的 `input()` 可能已越过其缓冲文本。
- `into_parts()` 不执行 I/O；在成功 finish 前调用它会明确放弃尚未发出的 codec
  lifecycle 输出。
- 当需要限制解码结果时，使用 `read_to_string_limited`、
  `read_to_string_limited_async` 或
  `read_to_string_with_charset_limited`。上限按该调用追加的 UTF-8 字节计算；
  超限会返回 `InvalidData` 并将目标字符串恢复到调用前长度。reader 仍可能消费或预读
  底层输入，且该上限不限制原始输入字节。

  对 `read_line_limited` 和 `read_line_limited_async`，超长行会先消费到配置的行结束符，
  然后才返回错误，因此下一次读取从下一条逻辑记录开始。

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
