// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structured recognition of decoded-text append-limit errors for fuzz targets.

use std::io;

use qubit_io_text::TextAppendLimitError;

/// Returns whether `error` is this crate's append-limit error for
/// `expected_limit`.
pub fn is_append_limit_error(error: &io::Error, expected_limit: usize) -> bool {
    error.kind() == io::ErrorKind::InvalidData
        && error
            .get_ref()
            .and_then(|source| source.downcast_ref::<TextAppendLimitError>())
            .is_some_and(|limit| limit.max_append_len() == expected_limit)
}
