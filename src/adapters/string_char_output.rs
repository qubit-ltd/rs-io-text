// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::Result;

use qubit_io::Output;
use qubit_utils::UncheckedSlice;
use qubit_utils::try_reserve_string;

/// Character output over a borrowed [`String`].
///
/// `StringCharOutput` exposes a mutable string as a
/// `qubit_io::CharOutput`. Writes append Unicode scalar values to the
/// wrapped string, and flushing is a no-op.
///
/// # Examples
///
/// ```
/// use qubit_io::Output;
/// use qubit_io_text::StringCharOutput;
///
/// let mut text = String::new();
/// let mut output = StringCharOutput::new(&mut text);
/// assert_eq!(output.write(&['h', 'i'])?, 2);
/// assert_eq!(output.get_ref(), "hi");
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct StringCharOutput<'a> {
    /// The destination string to which written characters are appended.
    output: &'a mut String,
}

impl<'a> StringCharOutput<'a> {
    /// Creates a character output over `output`.
    ///
    /// # Parameters
    /// - `output`: Destination string.
    ///
    /// # Returns
    /// A character output that appends to `output`.
    #[must_use]
    #[inline]
    pub const fn new(output: &'a mut String) -> Self {
        Self { output }
    }

    /// Returns a shared reference to the wrapped string.
    ///
    /// # Returns
    /// The wrapped string.
    #[must_use]
    #[inline]
    pub fn get_ref(&self) -> &String {
        self.output
    }

    /// Returns the wrapped string mutably.
    ///
    /// # Returns
    /// A mutable reference to the destination string, allowing callers to
    /// inspect or modify its contents while this adapter is borrowed.
    #[must_use]
    #[inline]
    pub fn get_mut(&mut self) -> &mut String {
        self.output
    }
}

impl Output for StringCharOutput<'_> {
    type Item = char;

    /// Appends characters from an indexed input range to the destination.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if reserving enough string capacity fails. No
    /// characters from this call are appended in that case.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `index..index + count` is a valid range in
    /// `input` and that computing the range end does not overflow.
    #[inline]
    unsafe fn write_unchecked(&mut self, input: &[char], index: usize, count: usize) -> Result<usize> {
        // SAFETY: The caller guarantees that the range is valid inside
        // `input`.
        let source = unsafe { UncheckedSlice::subslice(input, index, count) };
        let additional = source.iter().map(|ch| ch.len_utf8()).sum();
        try_reserve_string(self.output, additional)?;
        self.output.extend(source.iter().copied());
        Ok(count)
    }

    /// Flushes the string output.
    #[inline]
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
