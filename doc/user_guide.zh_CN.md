# Qubit Text IO 用户指南

Qubit Text IO 是 Qubit Rust 家族中的文本导向 I/O crate。它聚焦小型文本 trait
和 adapter，让业务逻辑处理 Unicode 文本，而把字节编码与具体目的地交给 adapter。

字节层 stream 工具请使用 [qubit-io](https://github.com/qubit-ltd/rs-io)。

## 何时使用本 crate

当你的代码处理的是 Unicode 文本，而不是原始字节时，可以使用 `qubit-text-io`。
典型场景包括 formatter、报告生成器、按行处理器、配置读取器，以及需要写入多个
文本目的地但不应知道其编码的组件。

适合场景：

- formatter 将字符串写入 UTF-8 文件、GBK 文件、`String` 或自定义日志 sink；
- parser 从外部编码可能不同的文本源按行读取；
- 组件需要显式换行符行为；
- 边界需要在 strict mode 下拒绝非法输入；
- 迁移工具需要 replacement mode 读取旧编码输入。

不适合场景：

- 二进制协议和字节级 framing；
- 文件系统路径管理；
- 递归目录操作；
- 完整 Unicode 分割、规范化、排序或显示宽度；
- 数据库专用持久化逻辑。

## 导入方式

直接导入使用到的 trait 和 adapter：

```rust
use qubit_text_io::{
    TextWrite,
    Utf8TextWriter,
};
```

当某个模块主要处理文本 I/O 时，可以使用 prelude：

```rust
use qubit_text_io::prelude::*;
```

## TextWrite

`TextWrite` 是文本生产者的主要抽象。生产者可以输出 Unicode 文本，而不提前选择
最终字节编码。

```rust
use qubit_text_io::TextWrite;

fn write_report<W>(writer: &mut W) -> Result<(), W::Error>
where
    W: TextWrite,
{
    writer.write_line("Report")?;
    writer.write_str("status: ok")?;
    writer.flush()
}
```

调用方决定 sink。它可以是 `String`、UTF-8 byte writer、encoded writer、logger
或数据库字段 adapter。

### 写入 String

`String` 直接实现了 `TextWrite`，默认使用 LF 换行：

```rust
use qubit_text_io::TextWrite;

let mut output = String::new();

output.write_line("hello")?;
output.write_char('中')?;

assert_eq!("hello\n中", output);

# Ok::<(), std::convert::Infallible>(())
```

如果需要显式配置换行符，使用 `StringTextWriter`：

```rust
use qubit_text_io::{
    LineEnding,
    StringTextWriter,
    TextWrite,
};

let mut output = String::new();
let mut writer = StringTextWriter::new(&mut output).with_line_ending(LineEnding::CrLf);

writer.write_line("hello")?;
drop(writer);

assert_eq!("hello\r\n", output);

# Ok::<(), std::convert::Infallible>(())
```

### 写入 UTF-8 字节

当目的地是应接收 UTF-8 字节的 `std::io::Write` sink 时，使用 `Utf8TextWriter`：

```rust
use qubit_text_io::{
    TextWrite,
    Utf8TextWriter,
};

let mut bytes = Vec::new();
let mut writer = Utf8TextWriter::new(&mut bytes);

writer.write_line("hello")?;
writer.flush()?;

assert_eq!(b"hello\n", bytes.as_slice());

# Ok::<(), std::io::Error>(())
```

### 写入显式编码

当目的地字节流需要特定编码时，使用 `EncodedTextWriter`：

```rust
use encoding_rs::GBK;
use qubit_text_io::{
    CodingErrorPolicy,
    EncodedTextWriter,
    TextWrite,
};

let mut bytes = Vec::new();
let mut writer = EncodedTextWriter::new(&mut bytes, GBK, CodingErrorPolicy::Strict);

writer.write_str("中文")?;
writer.flush()?;

let (decoded, _, had_errors) = GBK.decode(bytes.as_slice());
assert!(!had_errors);
assert_eq!("中文", decoded);

# Ok::<(), std::io::Error>(())
```

## TextRead 与 TextLineRead

`TextRead` 暴露 Unicode scalar value 和剩余文本；`TextLineRead` 增加按行读取能力，
并保留行终止符。

```rust
use qubit_text_io::{
    StrTextReader,
    TextRead,
};

let mut reader = StrTextReader::new("a中🙂");

assert_eq!(Some('a'), reader.read_char()?);
assert_eq!(Some('中'), reader.read_char()?);
assert_eq!(Some('🙂'), reader.read_char()?);
assert_eq!(None, reader.read_char()?);

# Ok::<(), std::convert::Infallible>(())
```

### 按行读取

`read_line` 会追加到目标字符串；只有 EOF 且没有追加任何文本时才返回 `false`。

```rust
use qubit_text_io::{
    StrTextReader,
    TextLineRead,
};

let mut reader = StrTextReader::new("first\r\nsecond");
let mut line = String::new();

assert!(reader.read_line(&mut line)?);
assert_eq!("first\r\n", line);

line.clear();
assert!(reader.read_line(&mut line)?);
assert_eq!("second", line);

line.clear();
assert!(!reader.read_line(&mut line)?);

# Ok::<(), std::convert::Infallible>(())
```

### 读取 UTF-8 字节

使用 `Utf8TextReader` 流式读取 UTF-8 byte input：

```rust
use std::io::Cursor;

use qubit_text_io::{
    TextLineRead,
    Utf8TextReader,
};

let input = Cursor::new("hello\n世界".as_bytes());
let mut reader = Utf8TextReader::from_read(input);
let mut line = String::new();

assert!(reader.read_line(&mut line)?);
assert_eq!("hello\n", line);

# Ok::<(), std::io::Error>(())
```

### 读取显式编码

`EncodedTextReader` 会在构造时解码全部输入，然后从内存中提供文本读取能力。它适用于
已经有大小边界的输入。

```rust
use std::io::Cursor;

use encoding_rs::GBK;
use qubit_text_io::{
    CodingErrorPolicy,
    EncodedTextReader,
    TextRead,
};

let (bytes, _, had_errors) = GBK.encode("中文");
assert!(!had_errors);

let mut reader = EncodedTextReader::new(
    Cursor::new(bytes.into_owned()),
    GBK,
    CodingErrorPolicy::Strict,
)?;
let mut output = String::new();

reader.read_to_string(&mut output)?;
assert_eq!("中文", output);

# Ok::<(), std::io::Error>(())
```

## 编码错误策略

`CodingErrorPolicy::Strict` 会拒绝非法输入字节和不可编码输出文本：

```rust
use std::io::Cursor;

use encoding_rs::UTF_8;
use qubit_text_io::{
    CodingErrorPolicy,
    EncodedTextReader,
};

let error = EncodedTextReader::new(
    Cursor::new(vec![0xFF]),
    UTF_8,
    CodingErrorPolicy::Strict,
)
.expect_err("strict mode rejects malformed UTF-8");

assert_eq!(std::io::ErrorKind::InvalidData, error.kind());
```

`CodingErrorPolicy::Replace` 会接受同样输入，并使用 `encoding_rs` 提供的替换行为。

## API 边界

本 crate 把 Rust `char` 视为 Unicode scalar value。它不等同于用户感知的“一个字符”。
一个 grapheme cluster 可能包含多个 Unicode scalar value。

当你需要以下能力时，请使用专门的 Unicode crate：

- grapheme cluster 边界；
- normalization；
- 区域相关大小写映射；
- 显示宽度；
- collation。

`qubit-text-io` 的职责是 I/O 边界：把外部编码和文本 source/sink 适配为 Rust
Unicode 文本。
