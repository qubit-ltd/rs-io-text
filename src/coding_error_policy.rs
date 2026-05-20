/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
/// Controls how text adapters handle malformed or unencodable data.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum CodingErrorPolicy {
    /// Reject malformed input bytes or unencodable Unicode text.
    #[default]
    Strict,

    /// Replace malformed input bytes or unencodable Unicode text.
    Replace,
}
