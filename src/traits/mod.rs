// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Text I/O traits.

mod async_text_line_read;
mod async_text_read;
mod async_text_write;
mod text_line_read;
mod text_read;
mod text_write;

pub use async_text_line_read::AsyncTextLineRead;
pub use async_text_read::AsyncTextRead;
pub use async_text_write::AsyncTextWrite;
pub use text_line_read::TextLineRead;
pub use text_read::TextRead;
pub use text_write::TextWrite;
