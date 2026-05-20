/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Text I/O traits.

mod text_line_read;
mod text_read;
mod text_write;

pub use text_line_read::TextLineRead;
pub use text_read::TextRead;
pub use text_write::TextWrite;
