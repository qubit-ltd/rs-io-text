// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{
    self,
    Write,
};

use qubit_codec_text::CharsetCodec;

use crate::{
    CharsetTextWriter,
    CodingErrorPolicy,
    TextWrite,
};

/// Extension methods for writing charset-encoded text to byte streams.
pub trait CharsetWriteExt: Write + Sized {
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
        policy: CodingErrorPolicy,
    ) -> CharsetTextWriter<Self, C>
    where
        C: CharsetCodec<Unit = u8>,
    {
        CharsetTextWriter::new(self, codec, policy)
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
        policy: CodingErrorPolicy,
        capacity: usize,
    ) -> CharsetTextWriter<Self, C>
    where
        C: CharsetCodec<Unit = u8>,
    {
        CharsetTextWriter::with_capacity(self, codec, policy, capacity)
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
        policy: CodingErrorPolicy,
    ) -> io::Result<()>
    where
        C: CharsetCodec<Unit = u8>,
    {
        let mut writer = CharsetTextWriter::new(self, codec, policy);
        writer.write_str(text)?;
        writer.finish()
    }
}

impl<W> CharsetWriteExt for W where W: Write + Sized {}
