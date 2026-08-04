// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Common text I/O traits and adapters for Qubit Text IO users.
//!
//! Charset codecs and byte-order types must be imported directly from their
//! owning `qubit-codec-text` or `qubit-codec` crate.

pub use crate::{
    AsyncCharsetTextReader,
    AsyncCharsetTextWriter,
    AsyncTextLineRead,
    AsyncTextRead,
    AsyncTextWrite,
    AsyncUtf8TextReader,
    AsyncUtf8TextWriter,
    BufferedReader,
    BufferedWriter,
    CharsetReadExt,
    CharsetStringDecoder,
    CharsetStringEncoder,
    CharsetTextReader,
    CharsetTextWriter,
    CharsetWriteExt,
    InputTextReader,
    LineEnding,
    LineEndingSet,
    OutputTextWriter,
    StrCharInput,
    StrTextReader,
    StringCharInput,
    StringCharOutput,
    StringTextReader,
    StringTextWriter,
    TextLineRead,
    TextRead,
    TextWrite,
    Utf8TextReader,
    Utf8TextWriter,
};
