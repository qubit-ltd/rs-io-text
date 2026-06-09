// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Text reader and writer adapters.

mod charset_text_reader;
mod charset_text_writer;
mod str_text_reader;
mod string_text_reader;
mod string_text_writer;
mod text_cursor;
mod utf8_text_reader;
mod utf8_text_writer;

pub use charset_text_reader::{BufferedCharsetTextReader, CharsetTextReader};
pub use charset_text_writer::{BufferedCharsetTextWriter, CharsetTextWriter};
pub use str_text_reader::StrTextReader;
pub use string_text_reader::StringTextReader;
pub use string_text_writer::StringTextWriter;
pub use utf8_text_reader::Utf8TextReader;
pub use utf8_text_writer::Utf8TextWriter;
