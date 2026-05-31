/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use qubit_codec_text::CharsetDecodePolicy;

/// Controls how text adapters handle malformed or unencodable data.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum CodingErrorPolicy {
    /// Reject malformed input bytes or unencodable Unicode text.
    #[default]
    Strict,

    /// Replace malformed input bytes or unencodable Unicode text.
    Replace,
}

impl CodingErrorPolicy {
    /// Converts this policy to a text-codec decode policy.
    ///
    /// # Returns
    ///
    /// Returns a reporting policy for strict mode and a default replacement
    /// policy for replacement mode.
    pub(crate) const fn decode_policy(self) -> CharsetDecodePolicy {
        match self {
            Self::Strict => CharsetDecodePolicy::report(),
            Self::Replace => CharsetDecodePolicy::replace(CharsetDecodePolicy::DEFAULT_REPLACEMENT),
        }
    }
}
