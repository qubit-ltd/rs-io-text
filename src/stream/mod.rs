// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Buffered text stream adapters.

mod buffered_reader;
mod buffered_writer;

pub use buffered_reader::BufferedReader;
pub use buffered_writer::BufferedWriter;
