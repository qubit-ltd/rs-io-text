# Qubit IO Text 用户指南

[English](user_guide.md) · [README](../README.zh_CN.md) ·
[API 文档](https://docs.rs/qubit-io-text)

本指南面向需要处理 Unicode 文本和文本行、而不是已编码字节的 Rust 应用，适用于
`qubit-io-text` 0.9。Charset 算法来自 `qubit-codec-text`，字节流和字符流抽象来自
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
qubit-io-text = "0.9"
qubit-codec-text = "^0.9"
qubit-io = "0.19"
```

本指南使用由 `qubit-codec-text` 和 `qubit-io` 提供的 `Utf8Codec` 与
`AsyncByteOutput`；Rust 消费方必须将这两个 crate 声明为直接依赖。

## 核心流程

```rust
use std::io::Cursor;

use qubit_codec_text::CharsetDecodePolicy;
use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::Utf8Codec;
use qubit_io_text::CharsetTextReader;
use qubit_io_text::CharsetTextWriter;
use qubit_io_text::LineEnding;
use qubit_io_text::TextReader;
use qubit_io_text::TextWriter;

fn main() -> std::io::Result<()> {
    let mut bytes = Vec::new();
    let mut writer = CharsetTextWriter::new(&mut bytes, Utf8Codec, CharsetEncodePolicy::report())
        .with_line_ending(LineEnding::CrLf);
    writer.write_line("subject: status")?;
    writer.write_str("中文")?;
    writer.finish()?;
    drop(writer);
    let mut reader = CharsetTextReader::new(Cursor::new(bytes), Utf8Codec, CharsetDecodePolicy::report());
    let mut text = String::new();
    let chars = reader.read_to_string(&mut text)?;
    assert_eq!(19, chars);
    assert_eq!("subject: status\r\n中文", text);
    Ok(())
}
```

编码器结束成功后，即使交付或 flush 失败，writer 也会拒绝新的文本。保留同一个 writer 并
重试 `finish()` 以完成交付；不要重放原始文本。编码器结束报错不保证通常可以安全重试。
仅需一次转换时，可使用 `CharsetReadExt` 和
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

成功返回时，`write_str_async` 的进度按 UTF-8 字节计数，`write_chars_async` 则按 `char` 元素计数；继续写入时，要据此截取各自的输入后缀。[AsyncTextWriter 示例](https://docs.rs/qubit-io-text/latest/qubit_io_text/trait.AsyncTextWriter.html#examples) 展示两种推进方式和显式结束编码器的步骤；[AsyncTextLineReader 示例](https://docs.rs/qubit-io-text/latest/qubit_io_text/trait.AsyncTextLineReader.html#examples) 展示保留终止符的逐行读取。

## 错误与诊断

- `CharsetDecodePolicy::report()` 报告畸形编码输入和 EOF 处不完整的编码尾部；
  `CharsetEncodePolicy::report()` 报告当前 codec 无法编码的 Unicode 字符。只有在明确允许
  忽略或替换时，才选择对应的 `ignore()` 或 `replace(...)` 策略。
- 编码器结束成功后，writer 会拒绝新的文本，即使输出交付或 flush 返回错误。保留该
  writer 并重试 `finish`，以继续交付而不重复结束编码器；不要重放原始文本。编码器结束
  本身报错，不代表事务回滚或通常可以安全重试。
  此重试建议针对正常返回的 I/O 错误。捕获底层 I/O panic 后，异步恢复没有不确定交付标记；
  重试或重放前请参阅[恢复待写字节](#恢复待写字节)。
- `CharsetStringDecoder::decode_to_string_into` 成功时追加文本；返回 `Err` 时目标字符串
  保留原内容。这不回滚 decoder 状态，也不覆盖 panic；流式 reader 有各自的前缀保留约定。
- 同步文本操作不保证事务性。读取失败时，目标可能保留已经追加的前缀，输入也可能
  已被消费；写入失败时，前缀可能已被接受或交付，重试整个字符串或整行可能造成重复。
  这些错误不报告已消费前缀。具体 API 可提供更强保证，尤其是下文的追加限额回滚规则
  仍然适用。需要确认缓冲交付时，应显式处理 flush 错误。
- buffered reader 可能会预先解码超出当前请求字符或行的数据。使用报告错误策略时，
  同一解码窗口后部的非法字节可能导致读取先报错，前面的合法字符或完整行尚未返回。
  例如输入为 `ok\n` 后接 `0xff` 时，读取可能先返回 `ok\n`，也可能先返回
  `InvalidData`，具体取决于底层输入如何分块。已追加到目标字符串的文本与尚未登记为
  可读的解码字符属于不同状态。错误不保证交付所有合法前缀、回滚输入或可安全重试；
  调小缓冲区也不是恢复协议。需要记录隔离时，应按所用编码和协议 framing 规则先隔离
  编码记录，再分别解码。不要按 LF 字节切分任意编码输入。
- 一次性方法 `read_to_string_with_charset` 和
  `read_to_string_with_charset_limited` 失败时不会返回已解码前缀，也不会交还临时
  decoder 或未交付的缓冲状态；底层输入却可能已经前移或被预读。
  `write_str_with_charset` 失败时可能已有前缀交付，整段重试会造成重复；临时 writer
  析构时还可能尽力排空待写数据，而析构错误无法返回。需要保留恢复状态时，应显式持有
  `CharsetTextReader` 或 `CharsetTextWriter`，并按[读端](#恢复未读文本)或
  [写端](#恢复待写字节)的 `into_parts` 边界处理。
  一次性调用和解码结果限额均不提供输入回滚。
- 同步 `BufferedTextWriter::write_str` 和 `CharsetTextWriter::write_str` 的主编码阶段
  输入位置从零开始，按本次调用的 Rust `char` 计数，不是 UTF-8 字节偏移或整个流的
  累计位置。charset 适配器还会同步修正 charset source 的输入字符位置；泛型编码器的
  不透明领域 source 保留其自身的位置语义。`write_line` 将正文和行结束符作为两次
  独立调用编码，各自从零计数。
- `AsyncCharsetTextWriter` 的单步编码错误按该次传入的字符切片或字符串后缀从零计数，
  完整写入错误则按该次完整调用的原始输入从零计数，不受内部窗口影响。完整行写入会
  分别编码正文和追加的行结束符，两者各自从零计数。这些索引用于诊断，不是可安全重试
  的游标；完整写入在返回错误或被取消前可能已经提交前缀。
- 本库的 `AsyncCharsetTextWriter` 和 `AsyncUtf8TextWriter` 在编码器结束成功后，
  通过固有方法和 `AsyncTextWriter` 方法写入空字符串或空字符切片，也会返回
  `InvalidInput`。开放状态的空写仍成功且不执行 I/O。调用方应省略结束后的空写，或在
  调用前判断输入是否为空；尚有待交付数据时重试 `finish_async`，不要重放文本。
- 异步 reader 的 `input()` 可能已越过其缓冲文本。
- 同步 buffered writer 的 `into_parts()` 不执行 I/O，并返回 `BufferedOutputRecovery`。`Pending` 表示尚未被报告写出的字节；`Indeterminate` 表示底层写入 panic，可能产生前缀副作用，不应自动重放。恢复不会调用 finish，编码器尚未发出的生命周期尾部也可能不在返回的 buffer 中。异步 writer 的 `into_parts()` 仍返回原始待写字节向量；`into_buffered_parts()` 直接返回已有缓冲区而不复制，待写窗口是 `pending.readable()`。异步恢复以正常的底层 I/O 返回合同为前提，panic 后没有不确定交付标记；重试或重放前请参阅[恢复待写字节](#恢复待写字节)。
- 当需要限制解码结果时，使用 `read_to_string_limited`、
  `read_to_string_limited_async` 或
  `read_to_string_with_charset_limited`。上限按该调用追加的 UTF-8 字节计算；
  超限会返回 `InvalidData` 并将目标字符串恢复到调用前长度。reader 仍可能消费或预读
  底层输入，且该上限不限制原始输入字节。

  对 `read_line_limited` 和 `read_line_limited_async`，被拒绝的行会先丢弃到配置的行结束符
  或 EOF，再返回原始的 `InvalidData` 限额错误。如果丢弃过程中遇到输入错误，会先返回该
  错误，reader 会保留恢复状态。应在同一 reader 上继续调用行读取方法；它会沿用原限额和
  换行配置继续丢弃，完成后只报告一次原始 `InvalidData`。之后读取才会进入下一条记录。

- 异步行读取若在追加上限超出前取消，目标字符串可能已有成功解码的前缀。应在同一
  reader 上继续读取，不要重复追加这个前缀；上限按每次调用追加的字节计算，不是整条
  逻辑行的累计上限。行一旦超限，目标字符串会回滚。如果取消发生在丢弃被拒绝行的过程
  中，下一次行读取也会继续恢复。
  例如输入 `abcdef\nok\n`、限额为 `4` 时，`abcdef\n` 会被丢弃并报告
  `InvalidData`，下一次读取才返回 `ok\n`。字符读取和 `into_parts` 不会自动恢复这项
  行恢复。读端 `into_parts` 的独立交接方式见[恢复未读文本](#恢复未读文本)。

### 恢复待写字节

该函数消费调用方已经持有、需要停止使用的 writer，不执行 finish 或 I/O。正常完成应先
调用 finish；发生错误后，仅在外围输出协议允许时处理 `pending.readable()`，不能重放已交付
前缀。对于 `AsyncUtf8TextWriter`，可调用 `writer.into_inner().into_buffered_parts()`。
只需要 `Vec<u8>` 的调用方可以继续使用 `into_parts()`。

恢复语义以底层 I/O 遵守正常返回合同为前提。异步缓冲器不追踪底层 I/O panic 后的污染或
不确定交付；捕获此类 panic 后，返回的待写字节可能包含已经交付的前缀。必须先核对外部
副作用，再决定是否重试或重放。`pending.readable()` 用于定位缓冲区内的逻辑窗口，但窗口
本身不能证明 panic 前这些字节均未交付。此 panic 限制不改变正常返回的 I/O 错误下的重试
语义，也不表示取消会丢失已报告的进度。

```rust
use qubit_codec_text::CharsetCodec;
use qubit_io::AsyncByteOutput;
use qubit_io::Buffer;
use qubit_io_text::AsyncCharsetTextWriter;

fn recover_pending<O, C>(writer: AsyncCharsetTextWriter<O, C>) -> (O, Buffer<u8>)
where
    O: AsyncByteOutput,
    C: CharsetCodec<Unit = u8>,
{
    writer.into_buffered_parts()
}
```

### 恢复未读文本

当应用需要接管 reader 已经预读的文本时，可使用 reader 的 `into_parts()`。下面已返回
给调用方的字符 `a` 由调用方持有，不属于剩余状态。先处理 `pending_chars`，再处理
`unread_bytes.readable()` 和底层输入中尚未读取的字节；多字节字符可能跨越这两个字节
片段，因此应将它们连续拼接并作为同一字节流解码。

本示例仅适用于已知有效的 UTF-8 输入，且没有额外 codec 状态。一般 charset 应保留返回的
decoder，并仅在其生命周期允许时继续驱动；它可能已经启动，已经结束的 decoder 不能通过
拆解重新打开。不要把这些片段交给可能会延迟 reset codec 的新 buffered reader。拆解不会
回滚解码错误，也不会恢复被拒绝行的丢弃状态；遇到这些情况时应继续使用原 reader。

```rust
use std::io;
use std::io::Cursor;
use std::io::Read;

use qubit_io_text::TextReader;
use qubit_io_text::Utf8TextReader;

fn main() -> io::Result<()> {
    let mut reader = Utf8TextReader::with_capacity(
        Cursor::new("abc🙂tail".as_bytes().to_vec()), 4,
    );
    assert_eq!(Some('a'), reader.read_char()?);
    let mut parts = reader.into_parts();
    assert_eq!(['b', 'c'], parts.pending_chars.as_slice());
    assert_eq!([0xf0], parts.unread_bytes.readable());
    let mut remaining: String = parts.pending_chars.into_iter().collect();
    let mut bytes = parts.unread_bytes.readable().to_vec();
    parts.input.read_to_end(&mut bytes)?;
    let tail = std::str::from_utf8(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    remaining.push_str(tail);
    assert_eq!("bc🙂tail", remaining);
    Ok(())
}
```

## 排障与最佳实践

| 症状 | 首先检查 |
| --- | --- |
| 换行符不符合预期 | 行读取前在 reader 上配置 `LineEndingSet`；调用 `write_line` 前在 writer 上配置 `LineEnding`。 |
| 解码拒绝数据 | 确认 codec，并有意选择 `CharsetDecodePolicy::report()` 或 `CharsetDecodePolicy::replace(...)` 及其替换字符。 |
| 输出似乎不完整 | finish charset writer；仅 flush 不会结束 codec。 |
| 失败后重试导致文本重复 | 同步错误不报告已接受的前缀；异步单步方法会返回成功进度。异步写入从返回的剩余后缀继续；必要时使用 framing 或幂等机制。 |

需要组合导入文本 trait 和 adapter 时，可使用 `qubit_io_text::prelude::*`。Codec 应从
`qubit-codec-text` 导入，因为本 crate 不拥有 charset 算法。

## 延伸阅读

- [README](../README.zh_CN.md) 与 [English README](../README.md)
- [English user guide](user_guide.md)
- [API 文档](https://docs.rs/qubit-io-text)

## 恢复中断的行读取

仅当调用方已确认可以继续读取时，才使用这种模式。`line` 只能包含当前尚未交付记录的
前缀；不能包含任意预置文本或已经返回给调用方的记录。返回 false 描述的是本次调用的
进度，因此到达 EOF 时仍可能保留待交付的前缀。示例会原样传播输入和解码错误，不会自动
重试。返回 `Some` 后，目标内容已取出且目标为空；再次在 EOF 调用会返回 `None`。

```rust
use std::io;

use qubit_io::AsyncByteInput;
use qubit_io_text::AsyncUtf8TextReader;

pub async fn resume_line<I>(
    reader: &mut AsyncUtf8TextReader<I>,
    line: &mut String,
) -> io::Result<Option<String>>
where
    I: AsyncByteInput + Unpin,
{
    let appended = reader.read_line_async(line).await?;
    if appended || !line.is_empty() {
        Ok(Some(std::mem::take(line)))
    } else {
        Ok(None)
    }
}
```
