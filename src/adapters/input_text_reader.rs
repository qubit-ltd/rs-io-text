// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io;

use qubit_io::Input;

use crate::{
    TextLineRead,
    TextRead,
};

/// Text reader over a `qubit_io::Input<Item = char>`.
#[derive(Debug)]
pub struct InputTextReader<I>
where
    I: Input<Item = char>,
{
    input: I,
}

impl<I> InputTextReader<I>
where
    I: Input<Item = char>,
{
    /// Creates a text reader over a character input.
    ///
    /// # Parameters
    /// - `input`: Character input to read.
    ///
    /// # Returns
    /// A text reader wrapping `input`.
    #[must_use]
    pub const fn new(input: I) -> Self {
        Self { input }
    }

    /// Returns a shared reference to the wrapped input.
    ///
    /// # Returns
    /// The wrapped input.
    #[must_use]
    pub const fn get_ref(&self) -> &I {
        &self.input
    }

    /// Returns a mutable reference to the wrapped input.
    ///
    /// # Returns
    /// The wrapped input.
    pub fn get_mut(&mut self) -> &mut I {
        &mut self.input
    }

    /// Returns the wrapped input.
    ///
    /// # Returns
    /// The underlying character input.
    #[must_use]
    pub fn into_inner(self) -> I {
        self.input
    }
}

impl<I> TextRead for InputTextReader<I>
where
    I: Input<Item = char>,
{
    type Error = io::Error;

    #[inline]
    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        let mut ch = ['\0'];
        if self.input.read(&mut ch)? == 0 {
            Ok(None)
        } else {
            Ok(Some(ch[0]))
        }
    }

    fn read_chars(
        &mut self,
        output: &mut Vec<char>,
        max: usize,
    ) -> Result<usize, Self::Error> {
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

    fn read_to_string(
        &mut self,
        output: &mut String,
    ) -> Result<usize, Self::Error> {
        let mut count = 0;
        while let Some(ch) = self.read_char()? {
            output.push(ch);
            count += 1;
        }
        Ok(count)
    }
}

impl<I> TextLineRead for InputTextReader<I>
where
    I: Input<Item = char>,
{
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        let mut read = false;
        while let Some(ch) = self.read_char()? {
            output.push(ch);
            read = true;
            if ch == '\n' {
                break;
            }
        }
        Ok(read)
    }
}
