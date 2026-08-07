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

pub use crate::AsyncCharsetTextReader;
pub use crate::AsyncCharsetTextWriter;
pub use crate::AsyncTextLineRead;
pub use crate::AsyncTextRead;
pub use crate::AsyncTextWrite;
pub use crate::AsyncUtf8TextReader;
pub use crate::AsyncUtf8TextWriter;
pub use crate::BufferedReader;
pub use crate::BufferedWriter;
pub use crate::CharsetReadExt;
pub use crate::CharsetStringDecoder;
pub use crate::CharsetStringEncoder;
pub use crate::CharsetTextReader;
pub use crate::CharsetTextWriter;
pub use crate::CharsetWriteExt;
pub use crate::InputTextReader;
pub use crate::LineEnding;
pub use crate::LineEndingSet;
pub use crate::OutputTextWriter;
pub use crate::StrCharInput;
pub use crate::StrTextReader;
pub use crate::StringCharInput;
pub use crate::StringCharOutput;
pub use crate::StringTextReader;
pub use crate::StringTextWriter;
pub use crate::TextLineRead;
pub use crate::TextRead;
pub use crate::TextReaderParts;
pub use crate::TextWrite;
pub use crate::Utf8TextReader;
pub use crate::Utf8TextWriter;
