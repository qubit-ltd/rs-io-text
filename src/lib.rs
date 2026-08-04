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
//! runtime-neutral [`qubit_io::AsyncInput`] and [`qubit_io::AsyncOutput`].

mod adapters;
mod ext;
mod io_error;
mod line_ending;
mod line_ending_set;
pub mod prelude;
mod stream;
mod traits;

pub use adapters::{
    AsyncCharsetTextReader,
    AsyncCharsetTextWriter,
    AsyncUtf8TextReader,
    AsyncUtf8TextWriter,
    CharsetStringDecoder,
    CharsetStringEncoder,
    CharsetTextReader,
    CharsetTextWriter,
    InputTextReader,
    OutputTextWriter,
    StrCharInput,
    StrTextReader,
    StringCharInput,
    StringCharOutput,
    StringTextReader,
    StringTextWriter,
    Utf8TextReader,
    Utf8TextWriter,
};
pub use ext::{
    CharsetReadExt,
    CharsetWriteExt,
};
pub use line_ending::LineEnding;
pub use line_ending_set::LineEndingSet;
pub use stream::{
    BufferedReader,
    BufferedWriter,
};
pub use traits::{
    TextLineRead,
    TextRead,
    TextWrite,
};
