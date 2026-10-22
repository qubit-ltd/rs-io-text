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

/// Character input over an owned [`String`].
///
/// `StringCharInput` exposes the owned text as a `qubit_io::Input<Item =
/// char>`. The current position is stored as a UTF-8 byte offset and is always
/// advanced on character boundaries.
///
/// # Examples
///
/// ```
/// use qubit_io::Input;
/// use qubit_io_text::StringCharInput;
///
/// let mut input = StringCharInput::new(String::from("hello"));
/// let mut output = [' '; 5];
/// assert_eq!(Input::read(&mut input, &mut output)?, 5);
/// assert_eq!(output, ['h', 'e', 'l', 'l', 'o']);
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct StringCharInput {
    /// The owned UTF-8 text being read.
    text: String,
    /// The next unread character's byte offset in `text`.
    position: usize,
}

impl StringCharInput {
    /// Creates a character input over owned text.
    ///
    /// # Parameters
    /// - `text`: Text to own and read.
    ///
    /// # Returns
    /// A character input positioned at the start of the text.
    #[inline]
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self { text, position: 0 }
    }

    /// Returns the current byte position in the underlying string.
    ///
    /// # Returns
    /// The current byte position.
    #[must_use]
    #[inline]
    pub const fn position(&self) -> usize {
        self.position
    }

    /// Returns a shared reference to the wrapped string.
    ///
    /// # Returns
    /// The wrapped string.
    #[must_use]
    #[inline]
    pub const fn get_ref(&self) -> &String {
        &self.text
    }

    /// Returns the owned string.
    ///
    /// # Returns
    /// The original string owned by this input.
    #[must_use]
    #[inline]
    pub fn into_inner(self) -> String {
        self.text
    }
}

impl Input for StringCharInput {
    type Item = char;

    /// Reads characters into an indexed output range without checking the
    /// range.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage.
    /// * `index` - Start index inside `output`.
    /// * `count` - Maximum number of characters to read.
    ///
    /// # Returns
    ///
    /// The number of characters written into `output[index..index + count]`.
    /// The value is between zero and `count`.
    ///
    /// # Errors
    ///
    /// Returns the input error reported by this implementation.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `output` and that the addition does not overflow.
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
