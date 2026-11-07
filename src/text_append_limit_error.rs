// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::error::Error;
use std::fmt;

/// An error indicating that decoded text exceeded an append-size limit.
///
/// The limit counts UTF-8 bytes in the text appended by the operation. This
/// type is carried as the source of an [`std::io::Error`] with kind
/// [`std::io::ErrorKind::InvalidData`], so callers can identify this condition
/// by downcasting the error source instead of matching its display text.
///
/// A reported line-limit error means the rejected line has been discarded.
/// Other limited reads retain their existing input-consumption behavior; this
/// error does not grant permission to retry an operation or imply that the
/// underlying input was rolled back.
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
/// use qubit_io_text::TextAppendLimitError;
/// use qubit_io_text::TextReader;
/// use qubit_io_text::Utf8TextReader;
///
/// let mut reader = Utf8TextReader::new(Cursor::new(b"too long"));
/// let mut output = String::new();
/// let error = reader.read_to_string_limited(&mut output, 3).unwrap_err();
/// let limit = error
///     .get_ref()
///     .and_then(|source| source.downcast_ref::<TextAppendLimitError>())
///     .map(TextAppendLimitError::max_append_len);
/// assert_eq!(Some(3), limit);
/// ```
#[must_use = "an append-limit error should be handled or reported"]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextAppendLimitError {
    /// The configured maximum number of UTF-8 bytes to append.
    max_append_len: usize,
}

impl TextAppendLimitError {
    /// Creates an append-limit error for the configured UTF-8 byte limit.
    ///
    /// # Parameters
    ///
    /// * `max_append_len` - Maximum number of UTF-8 bytes the operation may
    ///   append before rejecting the text.
    ///
    /// # Returns
    ///
    /// An error value carrying the configured append limit.
    #[inline]
    pub(crate) const fn new(max_append_len: usize) -> Self {
        Self { max_append_len }
    }

    /// Returns the configured maximum number of UTF-8 bytes to append.
    ///
    /// # Returns
    ///
    /// The byte limit configured for the operation that produced this error.
    #[must_use]
    #[inline]
    pub const fn max_append_len(&self) -> usize {
        self.max_append_len
    }
}

impl fmt::Display for TextAppendLimitError {
    /// Formats this error with its configured UTF-8 byte limit.
    ///
    /// # Parameters
    ///
    /// * `formatter` - Destination formatter provided by the formatting
    ///   framework.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the error text is written successfully.
    ///
    /// # Errors
    ///
    /// Returns the formatter's error if it cannot accept the output.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "decoded text exceeds the {}-byte append limit",
            self.max_append_len
        )
    }
}

impl Error for TextAppendLimitError {}
