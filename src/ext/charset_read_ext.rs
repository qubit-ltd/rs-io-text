// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
use std::io;

use qubit_codec_text::{CharsetCodec, CharsetDecodePolicy};
use qubit_io::{Input, InputRef};

use crate::{CharsetTextReader, TextRead};

/// Extension methods for reading charset-encoded text from byte streams.
pub trait CharsetReadExt: Input<Item = u8> + Sized {
    /// Wraps this byte reader as a charset text reader.
    ///
    /// # Parameters
    ///
    /// - `codec`: Charset codec used by the byte input.
    /// - `policy`: Malformed input handling policy.
    ///
    /// # Returns
    ///
    /// Returns a streaming text reader with the default buffer capacity.
    fn charset_text_reader<C>(
        self,
        codec: C,
        policy: CharsetDecodePolicy,
    ) -> CharsetTextReader<Self, C>
    where
        C: CharsetCodec<Unit = u8>,
    {
        CharsetTextReader::new(self, codec, policy)
    }

    /// Wraps this byte reader as a charset text reader with a buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `codec`: Charset codec used by the byte input.
    /// - `policy`: Malformed input handling policy.
    /// - `capacity`: Requested byte buffer capacity.
    ///
    /// # Returns
    ///
    /// Returns a streaming text reader using at least `capacity` bytes.
    fn buffered_charset_text_reader<C>(
        self,
        codec: C,
        policy: CharsetDecodePolicy,
        capacity: usize,
    ) -> CharsetTextReader<Self, C>
    where
        C: CharsetCodec<Unit = u8>,
    {
        CharsetTextReader::new_with_buffer_capacity(self, codec, policy, capacity)
    }

    /// Reads all remaining bytes as charset-encoded text.
    ///
    /// # Parameters
    ///
    /// - `codec`: Charset codec used by the byte input.
    /// - `policy`: Malformed input handling policy.
    ///
    /// # Returns
    ///
    /// Returns the decoded text.
    ///
    /// # Errors
    ///
    /// Returns I/O errors from the wrapped reader or invalid-data errors from
    /// charset decoding.
    fn read_to_string_with_charset<C>(
        &mut self,
        codec: C,
        policy: CharsetDecodePolicy,
    ) -> io::Result<String>
    where
        C: CharsetCodec<Unit = u8>,
    {
        let mut reader = CharsetTextReader::new(InputRef::new(self), codec, policy);
        let mut output = String::new();
        reader.read_to_string(&mut output)?;
        Ok(output)
    }

    /// Reads remaining bytes as charset-encoded text with an append-size limit.
    ///
    /// # Parameters
    ///
    /// - `codec`: Charset codec used by the byte input.
    /// - `policy`: Malformed input handling policy.
    /// - `max_append_len`: Maximum UTF-8 byte length of the decoded result.
    ///
    /// # Returns
    ///
    /// Returns the decoded text when it fits within `max_append_len` bytes.
    ///
    /// # Errors
    ///
    /// Returns I/O or charset-decoding errors. Returns
    /// [`io::ErrorKind::InvalidData`] when the decoded text exceeds
    /// `max_append_len`. The wrapped input can still be consumed or read ahead
    /// before that error is returned.
    fn read_to_string_with_charset_limited<C>(
        &mut self,
        codec: C,
        policy: CharsetDecodePolicy,
        max_append_len: usize,
    ) -> io::Result<String>
    where
        C: CharsetCodec<Unit = u8>,
    {
        let mut reader = CharsetTextReader::new(InputRef::new(self), codec, policy);
        let mut output = String::new();
        reader.read_to_string_limited(&mut output, max_append_len)?;
        Ok(output)
    }
}

impl<R> CharsetReadExt for R where R: Input<Item = u8> + Sized {}
