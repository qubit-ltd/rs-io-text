// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io;

use qubit_io::Input;

/// Concrete forwarding wrapper for a boxed character input.
pub(super) struct BoxedCharInput<'a> {
    input: Box<dyn Input<Item = char> + 'a>,
}

impl<'a> BoxedCharInput<'a> {
    /// Creates a forwarding wrapper for `input`.
    ///
    /// # Parameters
    ///
    /// - `input`: Boxed character input to forward to.
    ///
    /// # Returns
    ///
    /// Returns a concrete input wrapper around `input`.
    pub(super) const fn new(input: Box<dyn Input<Item = char> + 'a>) -> Self {
        Self { input }
    }
}

impl Input for BoxedCharInput<'_> {
    type Item = char;

    /// Forwards an unchecked character read to the boxed input.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination slice visible to the boxed input.
    /// - `index`: Start index of the writable destination range.
    /// - `count`: Maximum number of characters to read.
    ///
    /// # Returns
    ///
    /// Returns the number of characters read.
    ///
    /// # Errors
    ///
    /// Returns any error produced by the boxed input.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `output`.
    #[inline]
    unsafe fn read_unchecked(
        &mut self,
        output: &mut [char],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        // SAFETY: Forwarded from the trait caller.
        unsafe { self.input.read_unchecked(output, index, count) }
    }
}
