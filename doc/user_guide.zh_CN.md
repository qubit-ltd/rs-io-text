# Qubit IO Text 用户指南

[English](user_guide.md) · [README](../README.zh_CN.md) ·
[API 文档](https://docs.rs/qubit-io-text)

本指南面向需要处理 Unicode 文本和文本行、而不是已编码字节的 Rust 应用，适用于
`qubit-io-text` 0.8。Charset 算法来自 `qubit-codec-text`，字节流和字符流抽象来自
`qubit-io`。

## 概念模型

同步 `TextReader`、`TextLineReader` 与 `TextWriter` 直接处理 Rust 的 `char` 和 `str`；
异步 `AsyncTextReader`、`AsyncTextLineReader` 与 `AsyncTextWriter` 提供对应的运行时无关
操作。
Adapter 再将这些 trait 连接到字符串、字符流、UTF-8 字节流或可配置 charset。

| 需求 | 主要 API | 输入或输出 |
| --- | --- | --- |
| 内存中的 Unicode 文本 | `StrTextReader`、`StringTextReader`、`StringTextWriter` | 字符串 |
| 字符流 | `InputTextReader`、`OutputTextWriter` | `CharInput`、`CharOutput` |
| 严格 UTF-8 字节 | `Utf8TextReader`、`Utf8TextWriter` | `ByteInput`、`ByteOutput` |
| 同步 charset 转换 | `CharsetTextReader`、`CharsetTextWriter` | `ByteInput`、`ByteOutput` |
| 运行时无关的异步文本 | `AsyncTextReader`、`AsyncTextLineReader`、`AsyncTextWriter` | Unicode 文本 |
| 运行时无关的异步转换 | `AsyncCharsetTextReader`、`AsyncCharsetTextWriter`、`AsyncUtf8TextReader`、`AsyncUtf8TextWriter` | `AsyncByteInput`、`AsyncByteOutput` |

`TextReader::read_to_string` 将内容追加到目标并返回追加的 Unicode scalar value 数量，
而不是 UTF-8 字节数；`TextWriter::write_line` 会追加配置好的 `LineEnding`。

## 实战场景：在字节流上保持 Unicode 文本

假设应用收到一条 UTF-8 消息，添加一个以 CRLF 结束的 header 后，要将有效字节交给
另一个组件。流程是在构造时确定 codec 和错误策略，写入文本，完成 codec 生命周期，
再将同一批字节读回文本。

## 安装

```toml
[dependencies]
qubit-io-text = "0.8"
qubit-codec-text = "0.9"
qubit-io = "0.19"
```

本指南使用由 `qubit-codec-text` 和 `qubit-io` 提供的 `Utf8Codec` 与
`AsyncByteOutput`；Rust 消费方必须将这两个 crate 声明为直接依赖。

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
    TextReader,
    TextWriter,
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
drop(writer);

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

若策略来自运行时配置，应使用可失败构造器，例如
`CharsetTextWriter::try_new`。`AsciiCodec` 无法编码
`CharsetEncodePolicy::replace('中')`，因此该策略会在构造时被拒绝：

```rust
use qubit_codec_text::{AsciiCodec, CharsetEncodePolicy};
use qubit_io_text::CharsetTextWriter;

let result = CharsetTextWriter::try_new(
    Vec::<u8>::new(),
    AsciiCodec,
    CharsetEncodePolicy::replace('中'),
);
assert!(result.is_err());
```

`LineEnding::Lf`、`LineEnding::CrLf` 和 `LineEnding::Cr` 决定 `write_line` 追加的
内容。内置文本读取器默认识别 LF、CRLF 和 CR，并在返回字符串中保留完整终止符。
需要自定义策略时，可在读取器上调用 `with_line_endings` 配置 `LineEndingSet`；写入器
则使用 `LineEnding` 配置输出终止符。流确定为 UTF-8 时，可使用 `Utf8TextReader` 和
`Utf8TextWriter`，它们在相同的字节流边界提供严格的便利包装。

## 异步流程

运行时无关的 `AsyncTextReader`、`AsyncTextLineReader` 与 `AsyncTextWriter` 定义异步文本操作。
Charset 与 UTF-8 adapter 实现相应 trait；构造 adapter 时不会执行 I/O。

```rust
use qubit_io::AsyncByteOutput;
use qubit_codec_text::{CharsetEncodePolicy, Utf8Codec};
use qubit_io_text::{
    AsyncCharsetTextWriter,
    LineEnding,
};

async fn write_document<O>(output: O) -> std::io::Result<O>
where
    O: AsyncByteOutput + Unpin,
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

- `CharsetDecodePolicy::report()` 报告畸形编码输入和 EOF 处不完整的编码尾部；
  `CharsetEncodePolicy::report()` 报告当前 codec 无法编码的 Unicode 字符。只有在明确允许
  忽略或替换时，才选择对应的 `ignore()` 或 `replace(...)` 策略。
- `finish` 或 `finish_async` 失败后，writer 仍可用于重试。
- 同步文本操作不保证事务性。读取失败时，目标可能保留已经追加的前缀，输入也可能
  已被消费；写入失败时，前缀可能已被接受或交付，重试整个字符串或整行可能造成重复。
  这些错误不报告已消费前缀。具体 API 可提供更强保证，尤其是下文的追加限额回滚规则
  仍然适用。需要确认缓冲交付时，应显式处理 flush 错误。
- 异步 reader 的 `input()` 可能已越过其缓冲文本。
- 同步 buffered writer 的 `into_parts()` 不执行 I/O，并返回 `BufferedOutputRecovery`。`Pending` 表示尚未被报告写出的字节；`Indeterminate` 表示底层写入 panic，可能产生前缀副作用，不应自动重放。恢复不会调用 finish，编码器尚未发出的生命周期尾部也可能不在返回的 buffer 中。异步 writer 的 `into_parts()` 仍返回原始待写字节向量。
- 当需要限制解码结果时，使用 `read_to_string_limited`、
  `read_to_string_limited_async` 或
  `read_to_string_with_charset_limited`。上限按该调用追加的 UTF-8 字节计算；
  超限会返回 `InvalidData` 并将目标字符串恢复到调用前长度。reader 仍可能消费或预读
  底层输入，且该上限不限制原始输入字节。

  对 `read_line_limited` 和 `read_line_limited_async`，超长行会先消费到配置的行结束符，
  然后才返回错误，因此下一次读取从下一条逻辑记录开始。

- 异步行读取若在追加上限超出前取消，目标字符串可能已有成功解码的前缀。应在同一
  reader 上继续读取，不要重复追加这个前缀；上限按每次调用追加的字节计算，不是整条
  逻辑行的累计上限。行一旦超限，目标字符串会回滚。如果取消或临时输入错误中断了
  被拒绝记录的丢弃，应继续调用同一 reader 的行读取方法：它会按原限额和换行配置完成
  丢弃，再报告一次原始 `InvalidData`。恢复期间若先遇到输入错误，会先返回该输入错误。
  例如输入 `abcdef\nok\n`、限额为 `4` 时，`abcdef\n` 会被丢弃并报告
  `InvalidData`，下一次读取才返回 `ok\n`。字符读取和 `into_parts` 不会自动恢复这项
  行恢复。

## 排障与最佳实践

| 症状 | 首先检查 |
| --- | --- |
| 换行符不符合预期 | 调用 `write_line` 前在 writer 上配置 `LineEnding`。 |
| 解码拒绝数据 | 确认 codec，并有意选择 `CharsetDecodePolicy::report()` 或 `CharsetDecodePolicy::replace(...)` 及其替换字符。 |
| 输出似乎不完整 | finish charset writer；仅 flush 不会结束 codec。 |
| 失败后重试导致文本重复 | 同步错误不报告已接受的前缀；异步单步方法会返回成功进度。异步写入从返回的剩余后缀继续；必要时使用 framing 或幂等机制。 |

需要组合导入文本 trait 和 adapter 时，可使用 `qubit_io_text::prelude::*`。Codec 应从
`qubit-codec-text` 导入，因为本 crate 不拥有 charset 算法。

## 延伸阅读

- [README](../README.zh_CN.md) 与 [English README](../README.md)
- [English user guide](user_guide.md)
- [API 文档](https://docs.rs/qubit-io-text)
