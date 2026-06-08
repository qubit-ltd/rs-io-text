// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Extension traits for charset text I/O.

mod charset_read_ext;
mod charset_write_ext;

pub use charset_read_ext::CharsetReadExt;
pub use charset_write_ext::CharsetWriteExt;
