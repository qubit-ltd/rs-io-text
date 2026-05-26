/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use qubit_codec_text::{
    MalformedAction,
    UnmappableAction,
};

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
    /// Converts this policy to a text-codec malformed-input action.
    ///
    /// # Returns
    ///
    /// Returns [`MalformedAction::Report`] for strict mode and
    /// [`MalformedAction::Replace`] for replacement mode.
    pub(crate) const fn malformed_action(self) -> MalformedAction {
        match self {
            Self::Strict => MalformedAction::Report,
            Self::Replace => MalformedAction::Replace,
        }
    }

    /// Converts this policy to a text-codec unmappable-character action.
    ///
    /// # Returns
    ///
    /// Returns [`UnmappableAction::Report`] for strict mode and
    /// [`UnmappableAction::Replace`] for replacement mode.
    pub(crate) const fn unmappable_action(self) -> UnmappableAction {
        match self {
            Self::Strict => UnmappableAction::Report,
            Self::Replace => UnmappableAction::Replace,
        }
    }
}
