// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io;

use qubit_io::Input;

/// Borrowed input adapter used by one-shot extension methods.
pub(super) struct BorrowedInput<'a, I>
where
    I: Input<Item = u8> + ?Sized,
{
    inner: &'a mut I,
}

impl<'a, I> BorrowedInput<'a, I>
where
    I: Input<Item = u8> + ?Sized,
{
    /// Creates an adapter that forwards reads to a borrowed input.
    ///
    /// # Parameters
    ///
    /// - `inner`: Borrowed byte input to forward to.
    ///
    /// # Returns
    ///
    /// Returns a forwarding adapter borrowing `inner`.
    pub(super) const fn new(inner: &'a mut I) -> Self {
        Self { inner }
    }
}

impl<I> Input for BorrowedInput<'_, I>
where
    I: Input<Item = u8> + ?Sized,
{
    type Item = u8;

    /// Forwards an unchecked byte read to the borrowed input.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination slice visible to the borrowed input.
    /// - `index`: Start index of the writable destination range.
    /// - `count`: Maximum number of bytes to read.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes read.
    ///
    /// # Errors
    ///
    /// Returns any error produced by the borrowed input.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `output`.
    #[inline]
    unsafe fn read_unchecked(
        &mut self,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        // SAFETY: The caller guarantees the destination range for this
        // adapter; the same guarantee is forwarded to the wrapped input.
        unsafe { self.inner.read_unchecked(output, index, count) }
    }
}
