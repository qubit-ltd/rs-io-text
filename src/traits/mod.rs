// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Text I/O traits.

mod async_text_line_reader;
mod async_text_reader;
mod async_text_writer;
mod text_line_reader;
mod text_reader;
mod text_writer;

pub use async_text_line_reader::AsyncTextLineReader;
pub use async_text_reader::AsyncTextReader;
pub use async_text_writer::AsyncTextWriter;
pub use text_line_reader::TextLineReader;
pub use text_reader::TextReader;
pub use text_writer::TextWriter;
