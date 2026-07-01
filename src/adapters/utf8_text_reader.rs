// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{self, BufRead, BufReader, Read};

use crate::{TextLineRead, TextRead};
use qubit_io::UncheckedSlice;

/// Streaming text reader for UTF-8 byte input.
#[derive(Debug)]
pub struct Utf8TextReader<R> {
    inner: R,
}

impl<R> Utf8TextReader<R>
where
    R: BufRead,
{
    /// Creates a UTF-8 text reader over a buffered byte reader.
    ///
    /// # Parameters
    /// - `inner`: Buffered byte reader that yields UTF-8 data.
    ///
    /// # Returns
    /// A text reader wrapping `inner`.
    #[must_use]
    pub const fn new(inner: R) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the wrapped reader.
    ///
    /// # Returns
    /// The wrapped reader.
    #[must_use]
    pub const fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped reader.
    ///
    /// # Returns
    /// The wrapped reader.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Returns the wrapped reader.
    ///
    /// # Returns
    /// The underlying buffered reader.
    #[must_use]
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R> Utf8TextReader<BufReader<R>>
where
    R: Read,
{
    /// Creates a UTF-8 text reader over an unbuffered byte reader.
    ///
    /// # Parameters
    /// - `reader`: Byte reader that yields UTF-8 data.
    ///
    /// # Returns
    /// A text reader wrapping `reader` in [`BufReader`].
    #[must_use]
    pub fn from_read(reader: R) -> Self {
        Self {
            inner: BufReader::new(reader),
        }
    }
}

impl<R> TextRead for Utf8TextReader<R>
where
    R: BufRead,
{
    type Error = io::Error;

    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        read_utf8_char(&mut self.inner)
    }

    fn read_chars(&mut self, output: &mut Vec<char>, max: usize) -> Result<usize, Self::Error> {
        let mut count = 0;
        while count < max {
            match self.read_char()? {
                Some(ch) => {
                    output.push(ch);
                    count += 1;
                }
                None => break,
            }
        }
        Ok(count)
    }

    fn read_to_string(&mut self, output: &mut String) -> Result<usize, Self::Error> {
        let start = output.len();
        self.inner.read_to_string(output)?;
        Ok(output[start..].chars().count())
    }
}

impl<R> TextLineRead for Utf8TextReader<R>
where
    R: BufRead,
{
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        Ok(self.inner.read_line(output)? != 0)
    }
}

/// Reads one UTF-8 character from a byte reader.
///
/// # Parameters
/// - `reader`: Reader to consume bytes from.
///
/// # Returns
/// The next character, or `None` at EOF.
///
/// # Errors
/// Returns an I/O error when the underlying reader fails, when EOF appears in
/// the middle of a character, or when the byte sequence is not valid UTF-8.
fn read_utf8_char<R>(reader: &mut R) -> io::Result<Option<char>>
where
    R: Read + ?Sized,
{
    let mut first = [0_u8; 1];
    let read = loop {
        match reader.read(&mut first) {
            Ok(read) => break read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                continue;
            }
            Err(error) => return Err(error),
        }
    };
    if read == 0 {
        return Ok(None);
    }
    let first = unsafe { UncheckedSlice::read(&first, 0) };
    let width = utf8_char_width(first)?;
    debug_assert!(UncheckedSlice::range_fits(4, 0, width));
    let mut buffer = [0_u8; 4];
    unsafe {
        UncheckedSlice::write(&mut buffer, 0, first);
    }
    if width > 1 {
        debug_assert!(UncheckedSlice::range_fits(buffer.len(), 1, width - 1));
        reader.read_exact(&mut buffer[1..width])?;
    }
    let text = std::str::from_utf8(&buffer[..width]).map_err(invalid_utf8_error)?;
    Ok(text.chars().next())
}

/// Returns the UTF-8 character width implied by the first byte.
///
/// # Parameters
/// - `byte`: First byte of a UTF-8 sequence.
///
/// # Returns
/// The expected character width in bytes.
///
/// # Errors
/// Returns [`io::ErrorKind::InvalidData`] when `byte` cannot start a UTF-8
/// sequence.
fn utf8_char_width(byte: u8) -> io::Result<usize> {
    match byte {
        0x00..=0x7F => Ok(1),
        0xC2..=0xDF => Ok(2),
        0xE0..=0xEF => Ok(3),
        0xF0..=0xF4 => Ok(4),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid UTF-8 leading byte: 0x{byte:02X}"),
        )),
    }
}

/// Converts a UTF-8 validation error to an I/O error.
///
/// # Parameters
/// - `error`: UTF-8 validation error.
///
/// # Returns
/// An [`io::Error`] with [`io::ErrorKind::InvalidData`].
fn invalid_utf8_error(error: std::str::Utf8Error) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid UTF-8 text: {error}"),
    )
}
