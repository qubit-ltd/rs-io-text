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
use qubit_codec_text::CharsetEncodeError;
use qubit_codec_text::CharsetEncodePolicy;
use qubit_io::Output;
use qubit_io::OutputRef;

use crate::CharsetTextWriter;
use crate::TextWrite;

/// Extension methods for writing charset-encoded text to byte streams.
pub trait CharsetWriteExt: Output<Item = u8> + Sized {
    /// Wraps this byte writer as a charset text writer.
    ///
    /// # Parameters
    ///
    /// - `codec`: Charset codec used for outgoing text.
    /// - `policy`: Unencodable text handling policy.
    ///
    /// # Returns
    ///
    /// Returns a streaming text writer with the default buffer capacity.
    fn charset_text_writer<C>(
        self,
        codec: C,
        policy: CharsetEncodePolicy,
    ) -> CharsetTextWriter<Self, C>
    where
        C: CharsetCodec<Unit = u8>,
    {
        CharsetTextWriter::new(self, codec, policy)
    }

    /// Fallibly wraps this byte writer as a charset text writer.
    fn try_charset_text_writer<C>(
        self,
        codec: C,
        policy: CharsetEncodePolicy,
    ) -> Result<CharsetTextWriter<Self, C>, CharsetEncodeError>
    where
        C: CharsetCodec<Unit = u8>,
    {
        CharsetTextWriter::try_new(self, codec, policy)
    }

    /// Wraps this byte writer as a charset text writer with a buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `codec`: Charset codec used for outgoing text.
    /// - `policy`: Unencodable text handling policy.
    /// - `capacity`: Requested byte buffer capacity.
    ///
    /// # Returns
    ///
    /// Returns a streaming text writer using at least `capacity` bytes.
    fn buffered_charset_text_writer<C>(
        self,
        codec: C,
        policy: CharsetEncodePolicy,
        capacity: usize,
    ) -> CharsetTextWriter<Self, C>
    where
        C: CharsetCodec<Unit = u8>,
    {
        CharsetTextWriter::new_with_buffer_capacity(
            self, codec, policy, capacity,
        )
    }

    /// Fallibly wraps this byte writer with a requested buffer capacity.
    fn try_buffered_charset_text_writer<C>(
        self,
        codec: C,
        policy: CharsetEncodePolicy,
        capacity: usize,
    ) -> Result<CharsetTextWriter<Self, C>, CharsetEncodeError>
    where
        C: CharsetCodec<Unit = u8>,
    {
        CharsetTextWriter::try_new_with_buffer_capacity(
            self, codec, policy, capacity,
        )
    }

    /// Writes one string as charset-encoded text.
    ///
    /// # Parameters
    ///
    /// - `text`: Unicode text to encode and write.
    /// - `codec`: Charset codec used for outgoing text.
    /// - `policy`: Unencodable text handling policy.
    ///
    /// # Errors
    ///
    /// Returns I/O errors from the wrapped writer or invalid-input errors from
    /// charset encoding.
    fn write_str_with_charset<C>(
        &mut self,
        text: &str,
        codec: C,
        policy: CharsetEncodePolicy,
    ) -> io::Result<()>
    where
        C: CharsetCodec<Unit = u8>,
    {
        let mut writer =
            CharsetTextWriter::try_new(OutputRef::new(self), codec, policy)
                .map_err(crate::io_error::encode_error_to_io)?;
        writer.write_str(text)?;
        writer.finish()
    }
}

impl<W> CharsetWriteExt for W where W: Output<Item = u8> + Sized {}
