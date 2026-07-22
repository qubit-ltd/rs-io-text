// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io;

use qubit_io::Output;

/// Concrete forwarding wrapper for a boxed character output.
pub(super) struct BoxedCharOutput<'a> {
    output: Box<dyn Output<Item = char> + 'a>,
}

impl<'a> BoxedCharOutput<'a> {
    /// Creates a forwarding wrapper for `output`.
    ///
    /// # Parameters
    ///
    /// - `output`: Boxed character output to forward to.
    ///
    /// # Returns
    ///
    /// Returns a concrete output wrapper around `output`.
    pub(super) const fn new(output: Box<dyn Output<Item = char> + 'a>) -> Self {
        Self { output }
    }
}

impl Output for BoxedCharOutput<'_> {
    type Item = char;

    /// Forwards an unchecked character write to the boxed output.
    ///
    /// # Parameters
    ///
    /// - `input`: Source slice visible to the boxed output.
    /// - `index`: Start index of the readable source range.
    /// - `count`: Maximum number of characters to write.
    ///
    /// # Returns
    ///
    /// Returns the number of characters written.
    ///
    /// # Errors
    ///
    /// Returns any error produced by the boxed output.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `input`.
    #[inline]
    unsafe fn write_unchecked(
        &mut self,
        input: &[char],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        // SAFETY: Forwarded from the trait caller.
        unsafe { self.output.write_unchecked(input, index, count) }
    }

    /// Flushes the boxed output.
    ///
    /// # Errors
    ///
    /// Returns any error produced by the boxed output.
    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}
