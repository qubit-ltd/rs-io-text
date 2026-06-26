// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::Result;

use qubit_io::{Output, UncheckedSlice, try_reserve_string};

/// Character output over a borrowed [`String`].
///
/// `StringOutput` exposes a mutable string as a
/// `qubit_io::Output<Item = char>`. Writes append Unicode scalar values to the
/// wrapped string, and flushing is a no-op.
#[derive(Debug)]
pub struct StringOutput<'a> {
    output: &'a mut String,
}

impl<'a> StringOutput<'a> {
    /// Creates a character output over `output`.
    ///
    /// # Parameters
    /// - `output`: Destination string.
    ///
    /// # Returns
    /// A character output that appends to `output`.
    #[must_use]
    pub const fn new(output: &'a mut String) -> Self {
        Self { output }
    }

    /// Returns a shared reference to the wrapped string.
    ///
    /// # Returns
    /// The wrapped string.
    #[must_use]
    pub fn get_ref(&self) -> &String {
        self.output
    }

    /// Returns the wrapped string mutably.
    ///
    /// # Returns
    /// The wrapped string.
    pub fn get_mut(&mut self) -> &mut String {
        self.output
    }
}

impl Output for StringOutput<'_> {
    type Item = char;

    /// Writes characters from an indexed input range.
    #[inline]
    unsafe fn write_unchecked(
        &mut self,
        input: &[char],
        index: usize,
        count: usize,
    ) -> Result<usize> {
        debug_assert!(
            UncheckedSlice::range_fits(input.len(), index, count),
            "unchecked write range exceeds input buffer"
        );
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
