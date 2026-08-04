// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Configurable sets of text line endings.

use crate::LineEnding;

use std::io;

/// A set of line endings accepted by text readers.
///
/// The default set accepts LF, CRLF, and CR. When both CRLF and CR are
/// enabled, a CRLF sequence is consumed as one line ending.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LineEndingSet(u8);

impl LineEndingSet {
    /// A set accepting LF line endings.
    pub const LF: Self = Self(0b001);

    /// A set accepting CRLF line endings.
    pub const CRLF: Self = Self(0b010);

    /// A set accepting CR line endings.
    pub const CR: Self = Self(0b100);

    /// A set accepting all common line endings.
    pub const ALL: Self = Self(Self::LF.0 | Self::CRLF.0 | Self::CR.0);

    /// Creates a set accepting one line ending.
    #[must_use]
    pub const fn only(line_ending: LineEnding) -> Self {
        match line_ending {
            LineEnding::Lf => Self::LF,
            LineEnding::CrLf => Self::CRLF,
            LineEnding::Cr => Self::CR,
        }
    }

    /// Returns whether this set accepts `line_ending`.
    #[must_use]
    pub const fn contains(self, line_ending: LineEnding) -> bool {
        self.0 & Self::only(line_ending).0 != 0
    }

    /// Adds one line ending to this set.
    #[must_use]
    pub const fn with(self, line_ending: LineEnding) -> Self {
        Self(self.0 | Self::only(line_ending).0)
    }

    /// Removes one line ending from this set.
    #[must_use]
    pub const fn without(self, line_ending: LineEnding) -> Self {
        Self(self.0 & !Self::only(line_ending).0)
    }

    /// Returns whether this set accepts no line endings.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl Default for LineEndingSet {
    fn default() -> Self {
        Self::ALL
    }
}

/// Reads one line using configurable line-ending recognition.
pub(crate) fn read_line_with<E, ReadChar>(
    line_endings: LineEndingSet,
    output: &mut String,
    pending: &mut Option<char>,
    mut read_char: ReadChar,
) -> Result<bool, E>
where
    ReadChar: FnMut() -> Result<Option<char>, E>,
{
    let mut read = false;
    while let Some(ch) = if let Some(ch) = pending.take() {
        Some(ch)
    } else {
        read_char()?
    } {
        read = true;
        if ch == '\n' && line_endings.contains(LineEnding::Lf) {
            output.push(ch);
            return Ok(true);
        }
        if ch == '\r' {
            if line_endings.contains(LineEnding::CrLf) {
                // Retain CR while the lookahead can fail so callers can retry
                // without losing the character that started the boundary.
                *pending = Some('\r');
                match read_char()? {
                    Some('\n') => {
                        *pending = None;
                        output.push('\r');
                        output.push('\n');
                        return Ok(true);
                    }
                    Some(next) => {
                        *pending = None;
                        if line_endings.contains(LineEnding::Cr) {
                            output.push('\r');
                            *pending = Some(next);
                            return Ok(true);
                        }
                        output.push('\r');
                        *pending = Some(next);
                        continue;
                    }
                    None if line_endings.contains(LineEnding::Cr) => {
                        *pending = None;
                        output.push('\r');
                        return Ok(true);
                    }
                    None => {
                        *pending = None;
                        output.push('\r');
                        return Ok(true);
                    }
                }
            }
            if line_endings.contains(LineEnding::Cr) {
                output.push('\r');
                return Ok(true);
            }
        }
        output.push(ch);
    }
    Ok(read)
}

/// Consumes one line using configurable line-ending recognition.
///
/// The character already held in `pending` is consumed before the callback is
/// consulted. This is used after a bounded line read has failed so the next
/// read starts at the following logical line rather than at a suffix of the
/// oversized line.
pub(crate) fn discard_line_with<E, ReadChar>(
    line_endings: LineEndingSet,
    pending: &mut Option<char>,
    mut read_char: ReadChar,
) -> Result<(), E>
where
    ReadChar: FnMut() -> Result<Option<char>, E>,
{
    loop {
        let Some(ch) = (if let Some(ch) = pending.take() {
            Some(ch)
        } else {
            read_char()?
        }) else {
            return Ok(());
        };

        if ch == '\n' && line_endings.contains(LineEnding::Lf) {
            return Ok(());
        }
        if ch == '\r' {
            if line_endings.contains(LineEnding::CrLf) {
                *pending = Some('\r');
                match read_char()? {
                    Some('\n') => {
                        *pending = None;
                        return Ok(());
                    }
                    Some(next) => {
                        *pending = None;
                        if line_endings.contains(LineEnding::Cr) {
                            *pending = Some(next);
                            return Ok(());
                        }
                        *pending = Some(next);
                        continue;
                    }
                    None => {
                        *pending = None;
                        return Ok(());
                    }
                }
            }
            if line_endings.contains(LineEnding::Cr) {
                return Ok(());
            }
        }
    }
}

/// Appends one character while enforcing a UTF-8 byte limit.
pub(crate) fn append_limited_char(
    output: &mut String,
    initial_len: usize,
    max_append_len: usize,
    ch: char,
) -> io::Result<()> {
    let appended_len = output.len() - initial_len;
    if ch.len_utf8() > max_append_len.saturating_sub(appended_len) {
        output.truncate(initial_len);
        return Err(crate::io_error::text_append_limit_error(max_append_len));
    }
    output.push(ch);
    Ok(())
}
