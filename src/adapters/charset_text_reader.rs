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
    Read,
};

use qubit_codec_text::{
    CharsetCodec,
    CharsetDecodeError,
    CharsetDecoder,
    Coder,
    CoderStatus,
};

use crate::{
    CodingErrorPolicy,
    StringTextReader,
    TextLineRead,
    TextRead,
};

/// Text reader that decodes a byte reader with a charset codec.
///
/// This adapter decodes the complete byte input during construction and then
/// serves Unicode text from memory. Use it for bounded inputs such as
/// configuration files, protocol fields, and already-limited payloads.
#[derive(Debug)]
pub struct CharsetTextReader {
    inner: StringTextReader,
}

impl CharsetTextReader {
    /// Reads and decodes all bytes from `reader`.
    ///
    /// # Parameters
    ///
    /// - `reader`: Byte reader to decode.
    /// - `codec`: Byte-oriented charset codec used by the input.
    /// - `policy`: Malformed input handling policy.
    ///
    /// # Returns
    ///
    /// Returns a text reader over the decoded content.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when reading fails or when strict decoding finds
    /// malformed or incomplete input bytes.
    pub fn new<R, C>(mut reader: R, codec: C, policy: CodingErrorPolicy) -> io::Result<Self>
    where
        R: Read,
        C: CharsetCodec<Unit = u8>,
    {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        let text = decode_bytes(bytes.as_slice(), codec, policy)?;
        Ok(Self {
            inner: StringTextReader::new(text),
        })
    }

    /// Returns the inner decoded string reader.
    ///
    /// # Returns
    ///
    /// Returns the decoded string reader.
    #[must_use]
    pub fn into_inner(self) -> StringTextReader {
        self.inner
    }
}

impl TextRead for CharsetTextReader {
    type Error = io::Error;

    #[inline]
    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        self.inner.read_char().map_err(|error| match error {})
    }

    #[inline]
    fn read_chars(&mut self, output: &mut Vec<char>, max: usize) -> Result<usize, Self::Error> {
        self.inner.read_chars(output, max).map_err(|error| match error {})
    }

    #[inline]
    fn read_to_string(&mut self, output: &mut String) -> Result<usize, Self::Error> {
        self.inner.read_to_string(output).map_err(|error| match error {})
    }
}

impl TextLineRead for CharsetTextReader {
    #[inline]
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        self.inner.read_line(output).map_err(|error| match error {})
    }
}

/// Decodes bytes into a string according to the requested policy.
///
/// # Parameters
///
/// - `bytes`: Encoded input bytes.
/// - `codec`: Charset codec used by `bytes`.
/// - `policy`: Malformed input handling policy.
///
/// # Returns
///
/// Returns decoded Unicode text.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidData`] when strict mode detects malformed or
/// incomplete input bytes.
fn decode_bytes<C>(bytes: &[u8], codec: C, policy: CodingErrorPolicy) -> io::Result<String>
where
    C: CharsetCodec<Unit = u8>,
{
    let mut decoder = CharsetDecoder::new(codec);
    decoder.set_malformed_action(policy.malformed_action());
    let output_len = decoder.max_output_len(bytes.len()).unwrap_or(bytes.len()).max(1);
    let mut chars = vec!['\0'; output_len];
    let progress = decoder
        .convert(bytes, 0, chars.as_mut_slice(), 0)
        .map_err(decode_error_to_io)?;
    match progress.status() {
        CoderStatus::Complete => {
            chars.truncate(progress.written());
            Ok(chars.into_iter().collect())
        }
        CoderStatus::NeedInput { .. } if policy == CodingErrorPolicy::Replace => {
            chars.truncate(progress.written());
            chars.push(CharsetDecoder::<C>::DEFAULT_REPLACEMENT);
            Ok(chars.into_iter().collect())
        }
        CoderStatus::NeedInput { .. } => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "incomplete {} input at unit {}",
                decoder.codec().charset(),
                progress.index().unwrap_or(bytes.len()),
            ),
        )),
        CoderStatus::NeedOutput { .. } => unreachable!("decoder maximum-output contract failed"),
    }
}

/// Converts a charset decoding error into an I/O error.
///
/// # Parameters
///
/// - `error`: Charset decoding error.
///
/// # Returns
///
/// Returns an [`io::ErrorKind::InvalidData`] error carrying the codec context.
fn decode_error_to_io(error: CharsetDecodeError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}
