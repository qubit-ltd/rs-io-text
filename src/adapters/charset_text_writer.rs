/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::io::{
    self,
    Write,
};

use qubit_codec_text::{
    CharsetEncodeError,
    CharsetEncodePolicy,
    CharsetEncodeProbe,
    CharsetEncoder,
    TranscodeProgress,
    TranscodeStatus,
    Transcoder,
};

use crate::{
    CodingErrorPolicy,
    LineEnding,
    TextWrite,
};

/// Text writer that encodes Unicode text with a charset codec.
#[derive(Debug)]
pub struct CharsetTextWriter<W, C>
where
    C: CharsetEncodeProbe<Unit = u8>,
{
    inner: W,
    encoder: CharsetEncoder<C>,
    line_ending: LineEnding,
}

impl<W, C> CharsetTextWriter<W, C>
where
    W: Write,
    C: CharsetEncodeProbe<Unit = u8>,
{
    /// Creates a charset text writer.
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
        let encoder = match policy {
            CodingErrorPolicy::Strict => CharsetEncoder::with_policy(codec, CharsetEncodePolicy::report())
                .expect("reporting encode policy does not require an encodable replacement"),
            CodingErrorPolicy::Replace => CharsetEncoder::new(codec),
        };
        Self {
            inner,
            encoder,
            line_ending: LineEnding::Lf,
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
    pub const fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Returns a shared reference to the wrapped byte writer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped byte writer.
    #[must_use]
    pub const fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped byte writer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped byte writer.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Returns the wrapped byte writer.
    ///
    /// # Returns
    ///
    /// Returns the underlying byte writer.
    #[must_use]
    pub fn into_inner(self) -> W {
        self.inner
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
        self.line_ending
    }

    #[inline]
    fn write_char(&mut self, ch: char) -> Result<(), Self::Error> {
        self.write_chars(&[ch])
    }

    fn write_chars(&mut self, chars: &[char]) -> Result<(), Self::Error> {
        let output_len = self.encoder.max_output_len(chars.len()).unwrap_or(chars.len()).max(1);
        let mut bytes = vec![0_u8; output_len];
        let progress = self
            .encoder
            .transcode(chars, 0, bytes.as_mut_slice(), 0)
            .map_err(encode_error_to_io)?;
        let written = encoded_written(progress)?;
        self.inner.write_all(&bytes[..written])?;
        Ok(())
    }

    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        let chars: Vec<char> = text.chars().collect();
        self.write_chars(chars.as_slice())
    }

    fn write_line(&mut self, line: &str) -> Result<(), Self::Error> {
        self.write_str(line)?;
        self.write_str(self.line_ending.as_str())
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush()
    }
}

/// Converts a charset encoding error into an I/O error.
///
/// # Parameters
///
/// - `error`: Charset encoding error.
///
/// # Returns
///
/// Returns an [`io::ErrorKind::InvalidInput`] error carrying the codec context.
fn encode_error_to_io(error: CharsetEncodeError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

/// Returns the written unit count for a completed encoder progress.
fn encoded_written(progress: TranscodeProgress) -> io::Result<usize> {
    match progress.status() {
        TranscodeStatus::Complete => Ok(progress.written()),
        TranscodeStatus::NeedOutput { .. } => Err(io::Error::other(
            "charset encoder requested more output than its maximum-output contract",
        )),
        TranscodeStatus::NeedInput { .. } => unreachable!("encoder requested more character input"),
    }
}
