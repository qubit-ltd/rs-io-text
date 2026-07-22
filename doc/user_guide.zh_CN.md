# Qubit IO Text 用户指南

当应用层应处理 Unicode scalar value 和文本行，而不是编码后的字节时，使用
`qubit-io-text`。Charset codec 由 `qubit-codec-text` 提供，字节流和字符流由
`qubit-io` 提供。

## 安装

```toml
[dependencies]
qubit-io-text = "0.3"
```

## API 分层

| 层级 | 主要 API | 底层 I/O |
| --- | --- | --- |
| 同步文本 trait | `TextRead`、`TextLineRead`、`TextWrite` | Unicode `char` 与 `str` |
| 内存 adapter | `StrTextReader`、`StringTextReader`、`StringTextWriter`、`StringCharInput`、`StringCharOutput` | 字符串 |
| 字符流桥接 | `InputTextReader`、`OutputTextWriter` | `Input<Item = char>`、`Output<Item = char>` |
| UTF-8 便利层 | `Utf8TextReader`、`Utf8TextWriter` | `Input<Item = u8>`、`Output<Item = u8>` |
| 同步 charset | `CharsetTextReader`、`CharsetTextWriter` | `Input<Item = u8>`、`Output<Item = u8>` |
| 异步 charset | `AsyncCharsetTextReader`、`AsyncCharsetTextWriter` | `AsyncInput<Item = u8>`、`AsyncOutput<Item = u8>` |

当前异步 API 位于 charset adapter 上，并没有为每个同步文本 trait 都定义一套异步
对应物。

## Unicode 文本 Trait

`TextRead::read_to_string` 把文本追加到目标，并返回追加的 Unicode scalar value
数量，而不是 UTF-8 字节数。

```rust
use qubit_io_text::{
    StrTextReader,
    TextRead,
};

let mut reader = StrTextReader::new("中🙂");
let mut text = String::new();
let count = reader.read_to_string(&mut text)?;

assert_eq!(2, count);
assert_eq!(7, text.len());
assert_eq!("中🙂", text);
# Ok::<(), core::convert::Infallible>(())
```

输入存在换行符时，`TextLineRead::read_line` 会把它附加到输出。
`TextWrite::write_line` 则添加 writer 配置的 `LineEnding`。

```rust
use qubit_io_text::{
    LineEnding,
    StringTextWriter,
    TextWrite,
};

let mut output = String::new();
let mut writer =
    StringTextWriter::new(&mut output).with_line_ending(LineEnding::CrLf);
writer.write_line("first")?;

assert_eq!("first\r\n", output);
# Ok::<(), std::io::Error>(())
```

## 同步 Charset 流

`CharsetTextReader<I, C>` 解码任意 `I: Input<Item = u8>`；
`CharsetTextWriter<O, C>` 编码到任意 `O: Output<Item = u8>`。

内置 codec 类型包括 `AsciiCodec`、`Latin1Codec`、`Utf8Codec`、
`Utf16ByteCodec` 与 `Utf32ByteCodec`。

```rust
use std::io::Cursor;

use qubit_codec_text::Utf8Codec;
use qubit_io_text::{
    CharsetTextReader,
    CharsetTextWriter,
    CodingErrorPolicy,
    TextRead,
    TextWrite,
};

let mut bytes = Vec::new();
let mut writer =
    CharsetTextWriter::new(&mut bytes, Utf8Codec, CodingErrorPolicy::Strict);
writer.write_str("hello 中")?;
writer.finish()?;

let mut reader = CharsetTextReader::new(
    Cursor::new(bytes),
    Utf8Codec,
    CodingErrorPolicy::Strict,
);
let mut text = String::new();
reader.read_to_string(&mut text)?;
assert_eq!("hello 中", text);
# Ok::<(), std::io::Error>(())
```

`CharsetReadExt` 与 `CharsetWriteExt` 在 `Input` / `Output` 上提供构造和
one-shot helper：

```rust
use std::io::Cursor;

use qubit_codec_text::Utf8Codec;
use qubit_io_text::{
    CharsetReadExt,
    CharsetWriteExt,
    CodingErrorPolicy,
};

let mut input = Cursor::new(b"hello".to_vec());
let text = input.read_to_string_with_charset(
    Utf8Codec,
    CodingErrorPolicy::Strict,
)?;

let mut output = Vec::new();
output.write_str_with_charset(
    &text,
    Utf8Codec,
    CodingErrorPolicy::Strict,
)?;
# Ok::<(), std::io::Error>(())
```

## 异步 Charset Reader

`AsyncCharsetTextReader` 拥有 `AsyncInput`、decoder、保留的字节缓冲和字符缓冲。
构造过程不会触发 I/O。

```rust
use qubit_io::AsyncInput;
use qubit_codec_text::Utf8Codec;
use qubit_io_text::{
    AsyncCharsetTextReader,
    CodingErrorPolicy,
};

async fn read_document<I>(input: I) -> std::io::Result<String>
where
    I: AsyncInput<Item = u8> + Unpin,
{
    let mut reader = AsyncCharsetTextReader::new(
        input,
        Utf8Codec,
        CodingErrorPolicy::Strict,
    );
    let mut text = String::new();
    reader.read_to_string_async(&mut text).await?;
    Ok(text)
}
```

它提供：

- `read_char_async`；
- `read_chars_async`；
- `read_to_string_async`；
- `read_line_async`。

Reader 会在下一个挂起点前把已经收到的字节提交到保留缓冲，因此取消读取不会丢失
半个编码字符。`input()` 指向的底层流可能已经越过缓冲中的字节；
`into_input()` 会丢弃所有文本缓冲状态。

## 异步 Charset Writer

`AsyncCharsetTextWriter` 拥有 `AsyncOutput`、有状态 encoder 和待写编码字节。

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
    writer.write_line_async("header").await?;
    writer.write_str_async("body").await?;
    writer.finish_async().await?;
    writer.into_output_async().await
}
```

Writer 还提供 `write_char_async`、`write_chars_async` 与 `flush_async`。Flush
只排空编码字节，不结束 encoder；finish 会生成 codec 自己的尾部输出、排空字节并
刷新底层。失败的 `finish_async()` 会保留 pending 状态，可再次调用重试；消费型
转换失败时，`try_into_output_async()` 也会通过 `IntoInnerError` 保留完整 writer。
同步的 `BufferedWriter`、`OutputTextWriter`、`CharsetTextWriter` 和
`Utf8TextWriter` 提供对应的可恢复转换方法。

挂起和取消不会丢失 pending 编码字节。但取消高层写操作时，文本前缀可能已经生效；
除非外层协议允许重复前缀，否则不要盲目重试整个字符串。

## 错误策略与 EOF

`CodingErrorPolicy::Strict` 会报告 malformed 输入、不完整的编码 EOF 尾部和无法
编码的输出；`CodingErrorPolicy::Replace` 使用 codec 的替换值。

策略在 adapter 构造时确定，因此不会在一个有状态 charset 流的中途改变。

## UTF-8 便利 Adapter

`Utf8TextReader` / `Utf8TextWriter` 是基于 `Input<Item = u8>` /
`Output<Item = u8>` 的严格 UTF-8 便利包装。它们委托给
`CharsetTextReader` / `CharsetTextWriter` 使用的同一套 buffered charset
状态机；标准库流通过 Qubit 的 blanket adapter 使用，不再形成第二套核心 I/O 边界。
