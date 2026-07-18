// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair

use std::error::Error as StdError;
use std::io;

use qubit_codec::CapacityError;

/// Converts a decoder or decoder-contract error into a text I/O error.
pub(crate) fn decode_error_to_io<E>(error: E) -> io::Error
where
    E: StdError + Send + Sync + 'static,
{
    io::Error::new(io::ErrorKind::InvalidData, error)
}

/// Converts an encoder or encoder-contract error into a text I/O error.
pub(crate) fn encode_error_to_io<E>(error: E) -> io::Error
where
    E: StdError + Send + Sync + 'static,
{
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

/// Converts codec capacity planning errors into allocation-class I/O errors.
pub(crate) fn capacity_error_to_io(error: CapacityError) -> io::Error {
    io::Error::new(io::ErrorKind::OutOfMemory, error)
}
