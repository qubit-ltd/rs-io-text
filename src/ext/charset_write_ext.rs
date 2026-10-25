// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io;

use qubit_codec_text::CharsetCodec;
use qubit_codec_text::CharsetEncodeError;
use qubit_codec_text::CharsetEncodePolicy;
use qubit_io::ByteOutput;
use qubit_io::OutputRef;

use crate::CharsetTextWriter;
use crate::TextWriter;

/// Extension methods for writing charset-encoded text to byte streams.
///
/// # Examples
///
/// ```
/// use qubit_codec_text::{CharsetEncodePolicy, Utf8Codec};
/// use qubit_io_text::{CharsetWriteExt, TextWriter};
///
/// let mut bytes = Vec::new();
/// let mut writer = (&mut bytes).charset_text_writer(Utf8Codec, CharsetEncodePolicy::report());
/// writer.write_str("中文")?;
/// writer.finish()?;
/// drop(writer);
/// assert_eq!("中文".as_bytes(), bytes.as_slice());
/// # Ok::<(), std::io::Error>(())
/// ```
pub trait CharsetWriteExt: ByteOutput + Sized {
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
    ///
    /// # Panics
    ///
    /// Panics in replacement mode when the configured replacement character
    /// cannot be encoded by the selected codec. Use
    /// [`Self::try_charset_text_writer`] for policies obtained from
    /// configuration.
    #[must_use]
    #[inline]
    fn charset_text_writer<C>(self, codec: C, policy: CharsetEncodePolicy) -> CharsetTextWriter<Self, C>
    where
        C: CharsetCodec<Unit = u8>,
    {
        CharsetTextWriter::new(self, codec, policy)
    }

    /// Fallibly wraps this byte writer as a charset text writer.
    ///
    /// # Parameters
    ///
    /// - `codec`: Charset codec used for outgoing text.
    /// - `policy`: Unencodable text handling policy.
    ///
    /// # Returns
    ///
    /// Returns the configured writer when the replacement policy is valid.
    ///
    /// # Errors
    ///
    /// Returns [`CharsetEncodeError`] when replacement mode cannot encode the
    /// configured replacement character.
    #[inline]
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
    ///
    /// # Panics
    ///
    /// Panics in replacement mode when the configured replacement character
    /// cannot be encoded by the selected codec. Use
    /// [`Self::try_buffered_charset_text_writer`] for policies obtained from
    /// configuration.
    #[must_use]
    #[inline]
    fn buffered_charset_text_writer<C>(
        self,
        codec: C,
        policy: CharsetEncodePolicy,
        capacity: usize,
    ) -> CharsetTextWriter<Self, C>
    where
        C: CharsetCodec<Unit = u8>,
    {
        CharsetTextWriter::new_with_buffer_capacity(self, codec, policy, capacity)
    }

    /// Fallibly wraps this byte writer with a requested buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `codec`: Charset codec used for outgoing text.
    /// - `policy`: Unencodable text handling policy.
    /// - `capacity`: Requested byte buffer capacity.
    ///
    /// # Returns
    ///
    /// Returns the configured writer when the replacement policy is valid.
    ///
    /// # Errors
    ///
    /// Returns [`CharsetEncodeError`] when replacement mode cannot encode the
    /// configured replacement character.
    #[inline]
    fn try_buffered_charset_text_writer<C>(
        self,
        codec: C,
        policy: CharsetEncodePolicy,
        capacity: usize,
    ) -> Result<CharsetTextWriter<Self, C>, CharsetEncodeError>
    where
        C: CharsetCodec<Unit = u8>,
    {
        CharsetTextWriter::try_new_with_buffer_capacity(self, codec, policy, capacity)
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
    /// charset encoding. The writer may already have accepted or delivered a
    /// prefix, and the error does not report complete input progress; retrying
    /// the whole string may duplicate output. When the temporary writer is
    /// dropped, it may make a best-effort attempt to drain buffered output and
    /// cannot return any drain error. Therefore, this error does not guarantee
    /// that no more output was produced. The normal path reports finalization
    /// errors through `finish`. Keep a [`CharsetTextWriter`] and handle its
    /// `into_parts` result according to the specific error when state must be
    /// retained. `into_parts` is not a general rollback mechanism: it may not
    /// include a codec's un-emitted tail, and an indeterminate error does not
    /// make replay safe.
    fn write_str_with_charset<C>(&mut self, text: &str, codec: C, policy: CharsetEncodePolicy) -> io::Result<()>
    where
        C: CharsetCodec<Unit = u8>,
    {
        let mut writer = CharsetTextWriter::try_new(OutputRef::new(self), codec, policy)
            .map_err(crate::io_error::encode_error_to_io)?;
        writer.write_str(text)?;
        writer.finish()
    }
}

impl<W> CharsetWriteExt for W where W: ByteOutput + Sized {}
