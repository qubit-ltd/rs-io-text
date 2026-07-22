// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use core::fmt;
use std::io;

/// Error returned when a consuming writer conversion cannot finish.
///
/// The value retains both the I/O error and the original writer so callers can
/// inspect pending state, repair a transient output failure, and retry.
///
/// # Type Parameters
///
/// - `T`: Writer type retained after the failed conversion.
#[derive(Debug)]
pub struct IntoInnerError<T> {
    error: io::Error,
    writer: T,
}

impl<T> IntoInnerError<T> {
    /// Creates a recoverable consuming-conversion error.
    ///
    /// # Parameters
    ///
    /// - `error`: I/O error that prevented the conversion.
    /// - `writer`: Writer retained after the failed conversion.
    ///
    /// # Returns
    ///
    /// Returns an error containing both supplied values.
    pub(crate) const fn new(error: io::Error, writer: T) -> Self {
        Self { error, writer }
    }

    /// Returns the I/O error that prevented conversion.
    ///
    /// # Returns
    ///
    /// Returns a shared reference to the retained I/O error.
    #[must_use]
    #[inline(always)]
    pub const fn error(&self) -> &io::Error {
        &self.error
    }

    /// Returns the retained writer.
    ///
    /// # Returns
    ///
    /// Returns a shared reference to the retained writer.
    #[must_use]
    #[inline(always)]
    pub const fn writer(&self) -> &T {
        &self.writer
    }

    /// Returns a mutable reference to the retained writer.
    ///
    /// # Returns
    ///
    /// Returns a mutable reference to the retained writer.
    #[inline(always)]
    pub const fn writer_mut(&mut self) -> &mut T {
        &mut self.writer
    }

    /// Consumes this value and returns the retained writer.
    ///
    /// # Returns
    ///
    /// Returns the retained writer and discards the I/O error.
    #[must_use]
    #[inline(always)]
    pub fn into_writer(self) -> T {
        self.writer
    }

    /// Consumes this value and returns the underlying I/O error.
    ///
    /// # Returns
    ///
    /// Returns the retained I/O error and discards the writer.
    #[must_use]
    #[inline(always)]
    pub fn into_error(self) -> io::Error {
        self.error
    }

    /// Consumes this value and returns both the error and retained writer.
    ///
    /// # Returns
    ///
    /// Returns `(error, writer)`.
    #[must_use]
    #[inline(always)]
    pub fn into_parts(self) -> (io::Error, T) {
        (self.error, self.writer)
    }
}

impl<T> fmt::Display for IntoInnerError<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<T> std::error::Error for IntoInnerError<T>
where
    T: fmt::Debug,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
