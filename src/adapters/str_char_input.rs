// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::Result;

use qubit_io::Input;
use qubit_utils::SliceRange;
use qubit_utils::UncheckedSlice;

/// Character input over a borrowed string slice.
///
/// `StrCharInput` exposes borrowed text as a `qubit_io::CharInput`.
/// The current position is stored as a UTF-8 byte offset and is always advanced
/// on character boundaries.
///
/// # Examples
///
/// ```
/// use qubit_io::Input;
/// use qubit_io_text::StrCharInput;
///
/// let mut input = StrCharInput::new("hello");
/// let mut output = [' '; 5];
/// assert_eq!(Input::read(&mut input, &mut output)?, 5);
/// assert_eq!(output, ['h', 'e', 'l', 'l', 'o']);
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct StrCharInput<'a> {
    /// Valid UTF-8 text borrowed from the caller for this adapter's lifetime.
    text: &'a str,
    /// Byte offset of the next unread character, always on a UTF-8 boundary
    /// and no greater than the borrowed text's byte length.
    position: usize,
}

impl<'a> StrCharInput<'a> {
    /// Creates a character input over borrowed text.
    ///
    /// # Parameters
    /// - `text`: Text to borrow and read.
    ///
    /// # Returns
    /// A character input positioned at the start of the text.
    #[inline]
    #[must_use]
    pub const fn new(text: &'a str) -> Self {
        Self { text, position: 0 }
    }

    /// Returns the current byte position in the underlying string slice.
    ///
    /// # Returns
    /// The current byte position.
    #[inline]
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

    /// Returns the borrowed string slice.
    ///
    /// # Returns
    /// The borrowed string slice.
    #[inline]
    #[must_use]
    pub const fn get_ref(&self) -> &'a str {
        self.text
    }
}

impl Input for StrCharInput<'_> {
    type Item = char;

    /// Reads characters into an indexed output range without checking the
    /// range.
    ///
    /// # Parameters
    ///
    /// - `output` - Destination character storage.
    /// - `index` - Starting element index in `output`.
    /// - `count` - Maximum number of Unicode scalar values to read.
    ///
    /// # Returns
    ///
    /// Returns the number of characters written, in `0..=count`. A value
    /// below `count` indicates that the borrowed text was exhausted.
    /// Destination indices count characters, not UTF-8 bytes.
    ///
    /// # Errors
    ///
    /// This in-memory implementation does not produce I/O errors.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `index + count` does not overflow and
    /// that `index..index + count` is a valid range within `output`.
    /// A zero-length request still requires a valid empty range.
    #[inline]
    unsafe fn read_unchecked(&mut self, output: &mut [char], index: usize, count: usize) -> Result<usize> {
        debug_assert!(
            SliceRange::range_fits(output.len(), index, count),
            "unchecked read range exceeds output buffer"
        );
        let mut read = 0;
        while read < count {
            let Some(ch) = self.text[self.position..].chars().next() else {
                break;
            };
            // SAFETY: The caller guarantees the full destination range is
            // valid. Since `read < count`, this index is inside that range.
            unsafe {
                UncheckedSlice::write(output, index + read, ch);
            }
            self.position += ch.len_utf8();
            read += 1;
        }
        Ok(read)
    }
}
