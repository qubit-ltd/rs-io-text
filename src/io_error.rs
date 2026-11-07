// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::error::Error as StdError;
use std::io;

use qubit_codec::CapacityError;

use crate::TextAppendLimitError;

/// Converts a decoder or decoder-contract error into a text I/O error.
///
/// # Type Parameters
///
/// - `E`: The concrete decoder error type, which must be thread-safe and
///   provide a `'static` error source.
///
/// # Parameters
///
/// - `error`: The decoder error to retain as the I/O error's source.
#[must_use]
pub(crate) fn decode_error_to_io<E>(error: E) -> io::Error
where
    E: StdError + Send + Sync + 'static,
{
    io::Error::new(io::ErrorKind::InvalidData, error)
}

/// Converts an encoder or encoder-contract error into a text I/O error.
///
/// # Type Parameters
///
/// - `E`: The concrete encoder error type, which must be thread-safe and
///   provide a `'static` error source.
///
/// # Parameters
///
/// - `error`: The encoder error to retain as the I/O error's source.
#[must_use]
pub(crate) fn encode_error_to_io<E>(error: E) -> io::Error
where
    E: StdError + Send + Sync + 'static,
{
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

/// Converts codec capacity planning errors into allocation-class I/O errors.
///
/// # Parameters
///
/// - `error`: The capacity planning error to retain as the I/O error's source.
#[must_use]
pub(crate) fn capacity_error_to_io(error: CapacityError) -> io::Error {
    io::Error::new(io::ErrorKind::OutOfMemory, error)
}

/// Creates an error for decoded text that exceeds an append-size limit.
///
/// # Parameters
///
/// - `max_append_len`: The maximum number of UTF-8 bytes the operation may
///   append before returning an error.
#[must_use]
pub(crate) fn text_append_limit_error(max_append_len: usize) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, TextAppendLimitError::new(max_append_len))
}
