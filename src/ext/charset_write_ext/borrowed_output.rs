// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io;

use qubit_io::Output;

/// Borrowed output adapter used by one-shot extension methods.
pub(super) struct BorrowedOutput<'a, O>
where
    O: Output<Item = u8> + ?Sized,
{
    inner: &'a mut O,
}

impl<'a, O> BorrowedOutput<'a, O>
where
    O: Output<Item = u8> + ?Sized,
{
    /// Creates an adapter that forwards writes to a borrowed output.
    ///
    /// # Parameters
    ///
    /// - `inner`: Borrowed byte output to forward to.
    ///
    /// # Returns
    ///
    /// Returns a forwarding adapter borrowing `inner`.
    pub(super) const fn new(inner: &'a mut O) -> Self {
        Self { inner }
    }
}

impl<O> Output for BorrowedOutput<'_, O>
where
    O: Output<Item = u8> + ?Sized,
{
    type Item = u8;

    /// Forwards an unchecked byte write to the borrowed output.
    ///
    /// # Parameters
    ///
    /// - `input`: Source slice visible to the borrowed output.
    /// - `index`: Start index of the readable source range.
    /// - `count`: Maximum number of bytes to write.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written.
    ///
    /// # Errors
    ///
    /// Returns any error produced by the borrowed output.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `input`.
    #[inline]
    unsafe fn write_unchecked(
        &mut self,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        // SAFETY: The caller guarantees the source range for this adapter;
        // the same guarantee is forwarded to the wrapped output.
        unsafe { self.inner.write_unchecked(input, index, count) }
    }

    /// Flushes the borrowed output.
    ///
    /// # Errors
    ///
    /// Returns any error produced by the borrowed output.
    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
