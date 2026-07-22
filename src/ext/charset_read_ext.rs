// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
use std::io;

use qubit_codec_text::CharsetCodec;
use qubit_io::Input;

use crate::{
    CharsetTextReader,
    CodingErrorPolicy,
    TextRead,
};

mod borrowed_input;

use borrowed_input::BorrowedInput;

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
        policy: CodingErrorPolicy,
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
        policy: CodingErrorPolicy,
        capacity: usize,
    ) -> CharsetTextReader<Self, C>
    where
        C: CharsetCodec<Unit = u8>,
    {
        CharsetTextReader::new_with_buffer_capacity(
            self, codec, policy, capacity,
        )
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
        policy: CodingErrorPolicy,
    ) -> io::Result<String>
    where
        C: CharsetCodec<Unit = u8>,
    {
        let mut reader =
            CharsetTextReader::new(BorrowedInput::new(self), codec, policy);
        let mut output = String::new();
        reader.read_to_string(&mut output)?;
        Ok(output)
    }
}

impl<R> CharsetReadExt for R where R: Input<Item = u8> + Sized {}
