// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # Qubit Text IO
//!
//! Synchronous and asynchronous text I/O traits and adapters for Rust.
//!
//! This crate defines small traits for code that produces or consumes Unicode
//! text without choosing the final byte encoding or storage destination. It
//! also provides adapters for in-memory text, UTF-8 byte streams, and
//! byte-oriented [`qubit_codec_text`] charsets. [`AsyncCharsetTextReader`] and
//! [`AsyncCharsetTextWriter`] drive the same charset state machines through
//! runtime-neutral [`qubit_io::AsyncByteInput`] and
//! [`qubit_io::AsyncByteOutput`].
#![deny(missing_docs)]

mod adapters;
mod ext;
mod io_error;
mod line_ending;
mod line_ending_set;
pub mod prelude;
mod stream;
mod text_reader_parts;
mod traits;

pub use adapters::AsyncCharsetTextReader;
pub use adapters::AsyncCharsetTextWriter;
pub use adapters::AsyncUtf8TextReader;
pub use adapters::AsyncUtf8TextWriter;
pub use adapters::CharsetStringDecoder;
pub use adapters::CharsetStringEncoder;
pub use adapters::CharsetTextReader;
pub use adapters::CharsetTextWriter;
pub use adapters::InputTextReader;
pub use adapters::OutputTextWriter;
pub use adapters::StrCharInput;
pub use adapters::StrTextReader;
pub use adapters::StringCharInput;
pub use adapters::StringCharOutput;
pub use adapters::StringTextReader;
pub use adapters::StringTextWriter;
pub use adapters::Utf8TextReader;
pub use adapters::Utf8TextWriter;
pub use ext::CharsetReadExt;
pub use ext::CharsetWriteExt;
pub use line_ending::LineEnding;
pub use line_ending_set::LineEndingSet;
pub use stream::BufferedTextReader;
pub use stream::BufferedTextWriter;
pub use text_reader_parts::TextReaderParts;
pub use traits::AsyncTextLineReader;
pub use traits::AsyncTextReader;
pub use traits::AsyncTextWriter;
pub use traits::TextLineReader;
pub use traits::TextReader;
pub use traits::TextWriter;
