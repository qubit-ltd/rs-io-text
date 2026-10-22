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
use qubit_codec_text::CharsetEncoder;
use qubit_codec_text::Utf8Codec;
use qubit_io::BufferedOutputRecovery;
use qubit_io::Output;

use crate::BufferedWriter;
use crate::LineEnding;
use crate::TextWrite;

/// Text writer that encodes Unicode text with a charset codec.
///
/// This adapter is a charset-specific wrapper around [`BufferedWriter`]. It
/// constructs the appropriate [`CharsetEncoder`] from the supplied codec and
/// unmappable-character policy.
///
/// # Examples
///
/// ```
/// use qubit_codec_text::{CharsetEncodePolicy, Utf8Codec};
/// use qubit_io_text::{CharsetTextWriter, TextWrite};
///
/// let mut bytes = Vec::new();
/// let mut writer = CharsetTextWriter::new(
///     &mut bytes,
///     Utf8Codec,
///     CharsetEncodePolicy::report(),
/// );
/// writer.write_str("中文")?;
/// writer.finish()?;
/// let (output, recovery) = writer.into_parts();
/// let qubit_io::BufferedOutputRecovery::Pending(pending) = recovery else {
///     unreachable!("no inner write panicked");
/// };
/// assert!(pending.readable().is_empty());
/// assert_eq!("中文".as_bytes(), output.as_slice());
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct CharsetTextWriter<O, C>
where
    O: Output<Item = u8>,
    C: CharsetCodec<Unit = u8>,
{
    writer: BufferedWriter<O, CharsetEncoder<C>>,
}

impl<O, C> CharsetTextWriter<O, C>
where
    O: Output<Item = u8>,
    C: CharsetCodec<Unit = u8>,
{
    /// Creates a charset text writer with the default buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `output`: Byte writer to receive encoded bytes.
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
    #[inline]
    pub fn new(output: O, codec: C, policy: CharsetEncodePolicy) -> Self {
        Self::try_new(output, codec, policy).expect("charset encode policy replacement must be encodable")
    }

    /// Creates a charset text writer and reports an invalid replacement policy.
    ///
    /// # Parameters
    ///
    /// - `output`: Byte writer to receive encoded bytes.
    /// - `codec`: Byte-oriented charset codec used for outgoing text.
    /// - `policy`: Unencodable text handling policy.
    ///
    /// # Returns
    ///
    /// Returns a text writer using LF line endings.
    ///
    /// # Errors
    ///
    /// Returns [`CharsetEncodeError`] when replacement mode cannot encode the
    /// configured replacement character.
    pub fn try_new(output: O, codec: C, policy: CharsetEncodePolicy) -> Result<Self, CharsetEncodeError> {
        let encoder = CharsetEncoder::with_policy(codec, policy)?;
        Ok(Self {
            writer: BufferedWriter::new(output, encoder),
        })
    }

    /// Creates a charset text writer with a requested byte buffer capacity.
    ///
    /// # Parameters
    ///
    /// - `output`: Byte writer to receive encoded bytes.
    /// - `codec`: Byte-oriented charset codec used for outgoing text.
    /// - `policy`: Unencodable text handling policy.
    /// - `buffer_capacity`: Requested internal byte buffer capacity.
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
    #[inline]
    pub fn new_with_buffer_capacity(output: O, codec: C, policy: CharsetEncodePolicy, buffer_capacity: usize) -> Self {
        Self::try_new_with_buffer_capacity(output, codec, policy, buffer_capacity)
            .expect("charset encode policy replacement must be encodable")
    }

    /// Creates a charset text writer with a requested capacity and reports an
    /// invalid replacement policy.
    ///
    /// # Parameters
    ///
    /// - `output`: Byte writer to receive encoded bytes.
    /// - `codec`: Byte-oriented charset codec used for outgoing text.
    /// - `policy`: Unencodable text handling policy.
    /// - `buffer_capacity`: Requested internal byte buffer capacity.
    ///
    /// # Returns
    ///
    /// Returns a text writer using LF line endings.
    ///
    /// # Errors
    ///
    /// Returns [`CharsetEncodeError`] when replacement mode cannot encode the
    /// configured replacement character.
    pub fn try_new_with_buffer_capacity(
        output: O,
        codec: C,
        policy: CharsetEncodePolicy,
        buffer_capacity: usize,
    ) -> Result<Self, CharsetEncodeError> {
        let encoder = CharsetEncoder::with_policy(codec, policy)?;
        Ok(Self {
            writer: BufferedWriter::with_capacity(output, encoder, buffer_capacity),
        })
    }

    /// Returns a shared reference to the wrapped byte writer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped byte writer. Pending bytes may still be buffered.
    #[must_use = "the returned output and pending buffer must be handled"]
    #[inline(always)]
    pub const fn output(&self) -> &O {
        self.writer.inner()
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
    #[inline]
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.writer = self.writer.with_line_ending(line_ending);
        self
    }

    /// Finishes codec-owned output and flushes pending bytes.
    ///
    /// # Errors
    ///
    /// Returns encoding finalization errors or I/O errors from the wrapped
    /// writer. After a successful finish, later write calls return
    /// [`io::ErrorKind::InvalidInput`].
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after finishing codec output and flushing pending
    /// bytes.
    #[inline]
    pub fn finish(&mut self) -> io::Result<()> {
        self.writer.finish()
    }
    /// Recovers the wrapped output and the status of encoded bytes retained by this writer.
    ///
    /// This method performs no I/O and does not call [`Self::finish`] or flush
    /// the wrapped output. `Pending` bytes were not reported as written and may
    /// be retried. `Indeterminate` means an inner write panicked and may have
    /// produced an external prefix; do not replay it automatically. Call
    /// `finish` first for normal completion. Recovery does not replace finish:
    /// encoder lifecycle output not yet emitted may not be present in the
    /// returned byte buffer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output and pending or indeterminate encoded bytes in
    /// logical write order.
    #[must_use = "the wrapped output and recovery status must be handled"]
    #[inline(always)]
    pub fn into_parts(self) -> (O, BufferedOutputRecovery<u8>) {
        self.writer.into_parts()
    }
}

impl<O> CharsetTextWriter<O, Utf8Codec>
where
    O: Output<Item = u8>,
{
    /// Writes a valid UTF-8 string without re-encoding each scalar value.
    pub(crate) fn write_utf8_bytes(&mut self, text: &str) -> io::Result<()> {
        self.writer.write_encoded_bytes(text.as_bytes())
    }
}

impl<O, C> TextWrite for CharsetTextWriter<O, C>
where
    O: Output<Item = u8>,
    C: CharsetCodec<Unit = u8>,
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

    #[inline]
    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        self.writer.write_chars(chars)
    }

    #[inline]
    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        self.writer.write_str(text)
    }

    #[inline]
    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.writer.write_line(line)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.writer.flush()
    }
}
