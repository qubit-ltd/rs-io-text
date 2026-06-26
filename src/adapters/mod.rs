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
mod input_text_reader;
mod output_text_writer;
mod str_text_reader;
mod string_input;
mod string_output;
mod string_text_reader;
mod string_text_writer;
mod text_cursor;
mod utf8_text_reader;
mod utf8_text_writer;

pub use charset_text_reader::{BufferedCharsetTextReader, CharsetTextReader};
pub use charset_text_writer::{BufferedCharsetTextWriter, CharsetTextWriter};
pub use input_text_reader::InputTextReader;
pub use output_text_writer::OutputTextWriter;
pub use str_text_reader::StrTextReader;
pub use string_input::StringInput;
pub use string_output::StringOutput;
pub use string_text_reader::StringTextReader;
pub use string_text_writer::StringTextWriter;
pub use utf8_text_reader::Utf8TextReader;
pub use utf8_text_writer::Utf8TextWriter;
