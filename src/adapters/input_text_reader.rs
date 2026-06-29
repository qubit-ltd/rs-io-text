// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::collections::VecDeque;
use std::fmt;
use std::io;

use qubit_io::{BufferedInput, Input};

use crate::{TextLineRead, TextRead};

/// Default character chunk capacity for text reads.
const DEFAULT_CHAR_CHUNK_CAPACITY: usize = 256;

/// Text reader over a boxed `qubit_io::Input<Item = char>`.
pub struct InputTextReader<'a> {
    input: Box<dyn Input<Item = char> + 'a>,
    pending: VecDeque<char>,
}

impl<'a> InputTextReader<'a> {
    /// Creates a text reader over a character input.
    ///
    /// Unbuffered inputs are wrapped in [`BufferedInput`] automatically.
    ///
    /// # Parameters
    /// - `input`: Character input to read.
    ///
    /// # Returns
    /// A text reader wrapping `input`.
    #[must_use]
    pub fn new<I>(input: I) -> Self
    where
        I: Input<Item = char> + 'a,
    {
        Self {
            input: box_input(input),
            pending: VecDeque::new(),
        }
    }

    /// Creates a text reader from an already boxed character input.
    ///
    /// Unbuffered boxed inputs are wrapped in [`BufferedInput`] automatically.
    ///
    /// # Parameters
    /// - `input`: Boxed character input to read.
    ///
    /// # Returns
    /// A text reader wrapping `input`.
    #[must_use]
    pub fn from_boxed(input: Box<dyn Input<Item = char> + 'a>) -> Self {
        let input = if input.is_buffered() {
            input
        } else {
            Box::new(BufferedInput::new(BoxedCharInput { input }))
        };
        Self {
            input,
            pending: VecDeque::new(),
        }
    }

    /// Returns a shared reference to the wrapped input.
    ///
    /// # Returns
    /// The wrapped input trait object.
    #[must_use]
    pub fn get_ref(&self) -> &(dyn Input<Item = char> + 'a) {
        self.input.as_ref()
    }

    /// Returns a mutable reference to the wrapped input.
    ///
    /// # Returns
    /// The wrapped input trait object.
    pub fn get_mut(&mut self) -> &mut (dyn Input<Item = char> + 'a) {
        self.input.as_mut()
    }

    /// Returns the wrapped input.
    ///
    /// Pending characters already read past a line boundary are discarded.
    ///
    /// # Returns
    /// The underlying boxed character input.
    #[must_use]
    pub fn into_inner(self) -> Box<dyn Input<Item = char> + 'a> {
        self.input
    }

    fn drain_pending_chars(&mut self, output: &mut Vec<char>, max: usize) -> usize {
        let mut count = 0;
        while count < max {
            let Some(ch) = self.pending.pop_front() else {
                break;
            };
            output.push(ch);
            count += 1;
        }
        count
    }

    fn drain_pending_string(&mut self, output: &mut String) -> usize {
        let mut count = 0;
        while let Some(ch) = self.pending.pop_front() {
            output.push(ch);
            count += 1;
        }
        count
    }

    fn drain_pending_line(&mut self, output: &mut String) -> bool {
        while let Some(ch) = self.pending.pop_front() {
            output.push(ch);
            if ch == '\n' {
                return true;
            }
        }
        false
    }
}

impl TextRead for InputTextReader<'_> {
    type Error = io::Error;

    #[inline]
    fn read_char(&mut self) -> Result<Option<char>, Self::Error> {
        if let Some(ch) = self.pending.pop_front() {
            return Ok(Some(ch));
        }
        let mut ch = ['\0'];
        if self.input.read_fully(&mut ch)? == 0 {
            Ok(None)
        } else {
            Ok(Some(ch[0]))
        }
    }

    fn read_chars(&mut self, output: &mut Vec<char>, max: usize) -> Result<usize, Self::Error> {
        let mut count = self.drain_pending_chars(output, max);
        let mut chars = ['\0'; DEFAULT_CHAR_CHUNK_CAPACITY];
        while count < max {
            let requested = (max - count).min(chars.len());
            let read = self.input.read_fully(&mut chars[..requested])?;
            if read == 0 {
                break;
            }
            output.extend_from_slice(&chars[..read]);
            count += read;
        }
        Ok(count)
    }

    fn read_to_string(&mut self, output: &mut String) -> Result<usize, Self::Error> {
        let mut count = self.drain_pending_string(output);
        let mut chars = ['\0'; DEFAULT_CHAR_CHUNK_CAPACITY];
        loop {
            let read = self.input.read_fully(&mut chars)?;
            if read == 0 {
                return Ok(count);
            }
            output.extend(chars[..read].iter().copied());
            count += read;
        }
    }
}

impl TextLineRead for InputTextReader<'_> {
    fn read_line(&mut self, output: &mut String) -> Result<bool, Self::Error> {
        let initial_len = output.len();
        if self.drain_pending_line(output) {
            return Ok(true);
        }
        let mut read_any = output.len() > initial_len;

        let mut chars = ['\0'; DEFAULT_CHAR_CHUNK_CAPACITY];
        loop {
            let read = self.input.read_fully(&mut chars)?;
            if read == 0 {
                return Ok(read_any);
            }
            read_any = true;
            if let Some(index) = chars[..read].iter().position(|ch| *ch == '\n') {
                output.extend(chars[..=index].iter().copied());
                self.pending.extend(chars[index + 1..read].iter().copied());
                return Ok(true);
            }
            output.extend(chars[..read].iter().copied());
        }
    }
}

impl fmt::Debug for InputTextReader<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InputTextReader")
            .field("is_buffered", &self.input.is_buffered())
            .field("pending_len", &self.pending.len())
            .finish()
    }
}

struct BoxedCharInput<'a> {
    input: Box<dyn Input<Item = char> + 'a>,
}

impl Input for BoxedCharInput<'_> {
    type Item = char;

    #[inline]
    unsafe fn read_unchecked(
        &mut self,
        output: &mut [char],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        // SAFETY: Forwarded from the caller.
        unsafe { self.input.read_unchecked(output, index, count) }
    }
}

fn box_input<'a, I>(input: I) -> Box<dyn Input<Item = char> + 'a>
where
    I: Input<Item = char> + 'a,
{
    if input.is_buffered() {
        Box::new(input)
    } else {
        Box::new(BufferedInput::new(input))
    }
}
