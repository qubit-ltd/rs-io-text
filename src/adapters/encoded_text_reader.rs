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

use encoding_rs::Encoding;

use crate::{
    CodingErrorPolicy,
    StringTextReader,
    TextLineRead,
    TextRead,
};

/// Text reader that decodes a byte reader with an explicit encoding.
///
/// This adapter currently decodes the complete byte input during construction
/// and then serves text from memory. It is useful for bounded resources such as
/// configuration files, database fields, and already-limited payloads.
#[derive(Debug)]
pub struct EncodedTextReader {
    inner: StringTextReader,
}

impl EncodedTextReader {
    /// Reads and decodes all bytes from `reader`.
    ///
    /// # Parameters
    /// - `reader`: Byte reader to decode.
    /// - `encoding`: Encoding used by the byte reader.
    /// - `policy`: Malformed input handling policy.
    ///
    /// # Returns
    /// A text reader over the decoded content.
    ///
    /// # Errors
    /// Returns an I/O error when reading fails or when strict decoding finds
    /// malformed input bytes.
    pub fn new<R>(
        mut reader: R,
        encoding: &'static Encoding,
        policy: CodingErrorPolicy,
    ) -> io::Result<Self>
    where
        R: Read,
    {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        let text = decode_bytes(bytes.as_slice(), encoding, policy)?;
        Ok(Self {
            inner: StringTextReader::new(text),
        })
    }

    /// Returns the inner decoded string reader.
    ///
    /// # Returns
    /// The decoded string reader.
    #[must_use]
    pub fn into_inner(self) -> StringTextReader {
        self.inner
    }
}

impl TextRead for EncodedTextReader {
    type Error = io::Error;

    #[inline]
    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        self.inner.read_char().map_err(|error| match error {})
    }

    #[inline]
    fn read_chars(&mut self, output: &mut Vec<char>, max: usize) -> Result<usize, Self::Error> {
        self.inner
            .read_chars(output, max)
            .map_err(|error| match error {})
    }

    #[inline]
    fn read_to_string(&mut self, output: &mut String) -> Result<usize, Self::Error> {
        self.inner
            .read_to_string(output)
            .map_err(|error| match error {})
    }
}

impl TextLineRead for EncodedTextReader {
    #[inline]
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        self.inner.read_line(output).map_err(|error| match error {})
    }
}

/// Decodes bytes into a string according to the requested policy.
///
/// # Parameters
/// - `bytes`: Encoded input bytes.
/// - `encoding`: Encoding used by `bytes`.
/// - `policy`: Malformed input handling policy.
///
/// # Returns
/// Decoded Unicode text.
///
/// # Errors
/// Returns [`io::ErrorKind::InvalidData`] when strict mode detects malformed
/// input bytes.
fn decode_bytes(
    bytes: &[u8],
    encoding: &'static Encoding,
    policy: CodingErrorPolicy,
) -> io::Result<String> {
    let (text, _, had_errors) = encoding.decode(bytes);
    if had_errors && policy == CodingErrorPolicy::Strict {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("input is not valid {}", encoding.name()),
        ));
    }
    Ok(text.into_owned())
}
