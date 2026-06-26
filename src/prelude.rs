// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Common text I/O traits and adapters for Qubit Text IO users.

pub use crate::{
    AsciiCodec,
    BufferedCharsetTextReader,
    BufferedCharsetTextWriter,
    BufferedReader,
    BufferedWriter,
    ByteOrder,
    CharsetCodec,
    CharsetReadExt,
    CharsetStringDecoder,
    CharsetStringEncoder,
    CharsetTextReader,
    CharsetTextWriter,
    CharsetWriteExt,
    CodingErrorPolicy,
    InputTextReader,
    Latin1Codec,
    LineEnding,
    OutputTextWriter,
    StrTextReader,
    StringInput,
    StringOutput,
    StringTextReader,
    StringTextWriter,
    TextLineRead,
    TextRead,
    TextWrite,
    Utf8Codec,
    Utf8TextReader,
    Utf8TextWriter,
    Utf16ByteCodec,
    Utf32ByteCodec,
};
