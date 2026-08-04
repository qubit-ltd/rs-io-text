// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
/// Asynchronously reads Unicode scalar values and strings from a text source.
#[allow(async_fn_in_trait)]
pub trait AsyncTextRead {
    /// Error returned by this text source.
    type Error;

    /// Reads the next Unicode scalar value.
    async fn read_char_async(&mut self) -> Result<Option<char>, Self::Error>;

    /// Reads up to `max` Unicode scalar values into `output`.
    async fn read_chars_async(
        &mut self,
        output: &mut Vec<char>,
        max: usize,
    ) -> Result<usize, Self::Error> {
        let mut count = 0;
        while count < max {
            let Some(ch) = self.read_char_async().await? else {
                break;
            };
            output.push(ch);
            count += 1;
        }
        Ok(count)
    }

    /// Reads all remaining text into `output`.
    async fn read_to_string_async(
        &mut self,
        output: &mut String,
    ) -> Result<usize, Self::Error> {
        let mut count = 0;
        while let Some(ch) = self.read_char_async().await? {
            output.push(ch);
            count += 1;
        }
        Ok(count)
    }
}
