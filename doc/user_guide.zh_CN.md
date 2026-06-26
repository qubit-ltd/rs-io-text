# Qubit IO Text 用户指南

当应用代码希望处理 Unicode 文本，而不是直接处理原始字节时，使用 `qubit-io-text`。
本 crate 提供小型文本 trait，以及面向字符串、UTF-8 byte stream 和
`qubit-codec-text` byte-oriented charset 的 adapter。

## 能力地图

| 领域 | API | 用途 |
| --- | --- | --- |
| Trait | `TextRead`、`TextLineRead`、`TextWrite` | 抽象 Unicode 文本 source / sink |
| 内存 reader | `StrTextReader`、`StringTextReader` | 从借用或拥有字符串读取文本 |
| 内存 writer | `StringTextWriter` | 按换行策略写入借用 `String` |
| 字符 I/O 桥接 | `StringInput`、`StringOutput`、`InputTextReader`、`OutputTextWriter` | 组合 `qubit_io` 字符 input/output 与文本 trait |
| UTF-8 stream | `Utf8TextReader`、`Utf8TextWriter` | 将 `Read` / `Write` byte stream 适配为 UTF-8 文本 |
| Charset stream | `CharsetTextReader`、`CharsetTextWriter` | 适配 byte-oriented `qubit-codec-text` codec |
| 扩展 trait | `CharsetReadExt`、`CharsetWriteExt` | 从 `Read` 和 `Write` 创建 charset text stream |
| 策略值 | `LineEnding`、`CodingErrorPolicy` | 配置换行和 malformed / unmappable 处理 |

## 安装

```toml
[dependencies]
qubit-io-text = "0.1"
```

## 写入文本

`String`、`StringTextWriter`、`Utf8TextWriter`、`CharsetTextWriter`，以及
满足 `O: qubit_io::Output<Item = char>` 的 `OutputTextWriter<O>` 都实现了
`TextWrite`。

```rust
use qubit_io_text::TextWrite;

let mut output = String::new();
output.write_line("first")?;
output.write_str("second")?;

assert_eq!("first\nsecond", output);
# Ok::<(), std::convert::Infallible>(())
```

需要显式换行符时，使用 `StringTextWriter`：

```rust
use qubit_io_text::{
    LineEnding,
    StringTextWriter,
    TextWrite,
};

let mut output = String::new();
let mut writer = StringTextWriter::new(&mut output).with_line_ending(LineEnding::CrLf);

writer.write_line("first")?;

assert_eq!("first\r\n", output);
# Ok::<(), std::io::Error>(())
```

当 `I: qubit_io::Input<Item = char>` 时，`InputTextReader<I>` 实现
`TextRead`。`StringInput` 是 `StringTextReader` 使用的内存字符输入。

## UTF-8 Byte Stream

当 byte stream 固定是 UTF-8 时，使用 `Utf8TextWriter` 和 `Utf8TextReader`。

```rust
use std::io::Cursor;

use qubit_io_text::{
    TextRead,
    TextWrite,
    Utf8TextReader,
    Utf8TextWriter,
};

let mut bytes = Vec::new();
Utf8TextWriter::new(&mut bytes).write_str("a中")?;

let mut reader = Utf8TextReader::from_read(Cursor::new(bytes));
let mut text = String::new();
reader.read_to_string(&mut text)?;

assert_eq!("a中", text);
# Ok::<(), std::io::Error>(())
```

## Charset Adapter

`CharsetTextWriter` 和 `CharsetTextReader` 接收 `qubit-codec-text` 的 byte-oriented
codec，例如 `AsciiCodec`、`Latin1Codec`、`Utf8Codec`、`Utf16ByteCodec` 和
`Utf32ByteCodec`。

```rust
use qubit_io_text::{
    CharsetTextWriter,
    CodingErrorPolicy,
    TextWrite,
    Utf8Codec,
};

let mut bytes = Vec::new();
let mut writer = CharsetTextWriter::new(&mut bytes, Utf8Codec, CodingErrorPolicy::Strict);

writer.write_str("hello")?;
writer.flush()?;

assert_eq!(b"hello", bytes.as_slice());
# Ok::<(), std::io::Error>(())
```

Strict 模式会报告 malformed 输入或 unmappable 输出。Replace 模式使用
`qubit-codec-text` 提供的替换行为。

```rust
use std::io::Cursor;

use qubit_io_text::{
    CharsetTextReader,
    CodingErrorPolicy,
    TextRead,
    Utf8Codec,
};

let mut reader = CharsetTextReader::new(
    Cursor::new(vec![0xFF]),
    Utf8Codec,
    CodingErrorPolicy::Replace,
);
let mut output = String::new();

reader.read_to_string(&mut output)?;
assert_eq!("\u{FFFD}", output);
# Ok::<(), std::io::Error>(())
```

也可以通过扩展 trait 从标准 byte stream 创建同样的 charset stream：

```rust
use std::io::Cursor;

use qubit_io_text::{
    CharsetReadExt,
    CharsetWriteExt,
    CodingErrorPolicy,
    TextRead,
    Utf8Codec,
};

let mut reader = Cursor::new("hello".as_bytes().to_vec())
    .charset_text_reader(Utf8Codec, CodingErrorPolicy::Strict);
let mut text = String::new();
reader.read_to_string(&mut text)?;
assert_eq!("hello", text);

let mut bytes = Vec::new();
bytes.write_str_with_charset("hello", Utf8Codec, CodingErrorPolicy::Strict)?;
assert_eq!(b"hello", bytes.as_slice());
# Ok::<(), std::io::Error>(())
```

## 读取行

实现 `TextLineRead` 的 reader 在存在行终止符时会把它附加到输出中，与 Rust 标准库的
line-reading 行为一致。

```rust
use qubit_io_text::{
    StrTextReader,
    TextLineRead,
};

let mut reader = StrTextReader::new("first\nsecond");
let mut line = String::new();

assert!(reader.read_line(&mut line)?);
assert_eq!("first\n", line);
# Ok::<(), std::convert::Infallible>(())
```

## 选择合适层级

- 缓冲区级 charset 转换使用 `qubit-codec-text`。
- 文本需要通过 reader 或 writer trait 流动时，使用 `qubit-io-text`。
- 非文本专用的通用 byte stream helper 使用 `qubit-io`。
