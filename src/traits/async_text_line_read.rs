// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
use super::AsyncTextRead;

/// Asynchronously reads text by line while preserving line terminators.
#[allow(async_fn_in_trait)]
pub trait AsyncTextLineRead: AsyncTextRead {
    /// Reads one line into `output`.
    async fn read_line_async(
        &mut self,
        output: &mut String,
    ) -> Result<bool, Self::Error>;
}
