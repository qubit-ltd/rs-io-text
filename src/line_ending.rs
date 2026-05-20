/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
/// Line ending used by text writers when writing a complete line.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum LineEnding {
    /// Unix line ending, `\n`.
    #[default]
    Lf,

    /// Windows line ending, `\r\n`.
    CrLf,

    /// Classic Mac OS line ending, `\r`.
    Cr,
}

impl LineEnding {
    /// Returns the string representation of this line ending.
    ///
    /// # Returns
    /// The line ending sequence as a string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
            Self::Cr => "\r",
        }
    }
}
