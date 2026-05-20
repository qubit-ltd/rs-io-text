/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # Qubit Text IO
//!
//! Text-oriented I/O traits and adapters for Rust.
//!
//! This crate defines small traits for code that produces or consumes Unicode
//! text without choosing the final byte encoding or storage destination. It also
//! provides adapters for in-memory text, UTF-8 byte streams, and explicit
//! `encoding_rs` encodings.

mod adapters;
mod coding_error_policy;
mod line_ending;
pub mod prelude;
mod traits;

pub use adapters::{
    EncodedTextReader,
    EncodedTextWriter,
    StrTextReader,
    StringTextReader,
    StringTextWriter,
    Utf8TextReader,
    Utf8TextWriter,
};
pub use coding_error_policy::CodingErrorPolicy;
pub use line_ending::LineEnding;
pub use traits::{
    TextLineRead,
    TextRead,
    TextWrite,
};
