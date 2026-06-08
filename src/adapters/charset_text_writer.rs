// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{self, Write};

use qubit_codec_text::{CharsetEncodePolicy, CharsetEncodeProbe, CharsetEncoder};

use crate::{BufferedWriter, CodingErrorPolicy, LineEnding, TextWrite};

/// Text writer that encodes Unicode text with a charset codec.
///
/// This adapter is a charset-specific wrapper around [`BufferedWriter`]. It
/// constructs the appropriate [`CharsetEncoder`] from the supplied codec and
/// unmappable-character policy.
#[derive(Debug)]
pub struct CharsetTextWriter<W, C>
where
    W: Write,
    C: CharsetEncodeProbe<Unit = u8>,
{
    writer: BufferedWriter<W, CharsetEncoder<C>>,
}

impl<W, C> CharsetTextWriter<W, C>
where
    W: Write,
    C: CharsetEncodeProbe<Unit = u8>,
{
    /// Creates a charset text writer with the default buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Byte writer to receive encoded bytes.
    /// - `codec`: Byte-oriented charset codec used for outgoing text.
    /// - `policy`: Unencodable text handling policy.
    ///
    /// # Returns
    ///
    /// Returns a text writer using LF line endings.
    ///
    /// # Panics
    ///
    /// In replacement mode, panics if `codec` cannot encode either the default
    /// replacement character or the fallback `?` replacement. That indicates a
    /// broken codec invariant, not recoverable input data.
    #[must_use]
    pub fn new(inner: W, codec: C, policy: CodingErrorPolicy) -> Self {
        let encoder = create_encoder(codec, policy);
        Self {
            writer: BufferedWriter::new(inner, encoder),
        }
    }

    /// Creates a charset text writer with a requested byte buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Byte writer to receive encoded bytes.
    /// - `codec`: Byte-oriented charset codec used for outgoing text.
    /// - `policy`: Unencodable text handling policy.
    /// - `capacity`: Requested internal byte buffer capacity.
    ///
    /// # Returns
    ///
    /// Returns a text writer using LF line endings.
    ///
    /// # Panics
    ///
    /// In replacement mode, panics if no replacement character can be encoded
    /// by the codec.
    #[must_use]
    pub fn with_capacity(inner: W, codec: C, policy: CodingErrorPolicy, capacity: usize) -> Self {
        let encoder = create_encoder(codec, policy);
        Self {
            writer: BufferedWriter::with_capacity(inner, encoder, capacity),
        }
    }

    /// Sets the line ending for this writer.
    ///
    /// # Parameters
    ///
    /// - `line_ending`: Line ending to use for subsequent lines.
    ///
    /// # Returns
    ///
    /// Returns this writer with the configured line ending.
    #[must_use]
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.writer = self.writer.with_line_ending(line_ending);
        self
    }

    /// Returns a shared reference to the wrapped byte writer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped byte writer. Pending bytes may still be buffered.
    #[must_use]
    pub const fn get_ref(&self) -> &W {
        self.writer.inner()
    }

    /// Returns a mutable reference to the wrapped byte writer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped byte writer. Flush first if it must observe all
    /// prior text writes.
    pub fn get_mut(&mut self) -> &mut W {
        self.writer.inner_mut()
    }

    /// Returns a shared reference to the wrapped byte writer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped byte writer.
    #[must_use]
    pub const fn inner(&self) -> &W {
        self.writer.inner()
    }

    /// Returns a mutable reference to the wrapped byte writer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped byte writer.
    pub fn inner_mut(&mut self) -> &mut W {
        self.writer.inner_mut()
    }

    /// Finishes codec-owned output and flushes pending bytes.
    ///
    /// # Errors
    ///
    /// Returns encoding finalization errors or I/O errors from the wrapped
    /// writer. After a successful finish, later write calls return
    /// [`io::ErrorKind::InvalidInput`].
    pub fn finish(&mut self) -> io::Result<()> {
        self.writer.finish()
    }

    /// Returns the wrapped byte writer after finishing and flushing.
    ///
    /// # Returns
    ///
    /// Returns the underlying byte writer after pending bytes reach it.
    ///
    /// # Errors
    ///
    /// Returns encoding finalization or I/O errors.
    pub fn into_inner(self) -> io::Result<W> {
        self.writer.into_inner()
    }
}

impl<W, C> TextWrite for CharsetTextWriter<W, C>
where
    W: Write,
    C: CharsetEncodeProbe<Unit = u8>,
{
    type Error = io::Error;

    #[inline]
    fn line_ending(&self) -> LineEnding {
        self.writer.configured_line_ending()
    }

    #[inline]
    fn write_char(&mut self, ch: char) -> Result<(), Self::Error> {
        self.writer.write_char(ch)
    }

    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        self.writer.write_chars(chars)
    }

    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        self.writer.write_str(text)
    }

    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.writer.write_line(line)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.writer.flush()
    }
}

/// Creates a charset encoder from the public text I/O policy.
///
/// # Parameters
///
/// - `codec`: Charset codec used for outgoing text.
/// - `policy`: Text I/O unencodable-character policy.
///
/// # Returns
///
/// Returns a streaming charset encoder.
///
/// # Panics
///
/// Panics only when replacement mode cannot build a replacement encoder for
/// the supplied codec, matching [`CharsetEncoder::new`] semantics.
fn create_encoder<C>(codec: C, policy: CodingErrorPolicy) -> CharsetEncoder<C>
where
    C: CharsetEncodeProbe<Unit = u8>,
{
    match policy {
        CodingErrorPolicy::Strict => {
            CharsetEncoder::with_policy(codec, CharsetEncodePolicy::report())
                .expect("reporting encode policy does not require an encodable replacement")
        }
        CodingErrorPolicy::Replace => CharsetEncoder::new(codec),
    }
}
