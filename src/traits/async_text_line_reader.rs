// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use super::AsyncTextReader;

/// Asynchronously reads text by line while preserving line terminators.
///
/// Implementations define the accepted line-ending policy. Built-in readers
/// preserve a recognized terminator in the returned text.
/// Cancellation behavior is implementation-specific; callers must consult the
/// concrete reader before retrying a cancelled operation.
///
/// # Examples
///
/// For input `A\r\n中\n🙂`, this function returns `A\r\n`, `中\n`, and `🙂`.
/// A successful read includes a final unterminated line; `false` means EOF
/// with no new text. Clear the destination after saving each line because
/// reads append to it, and recognized terminators remain in the returned text.
///
/// ```no_run
/// use qubit_io::AsyncByteInput;
/// use qubit_io_text::AsyncTextLineReader;
/// use qubit_io_text::AsyncUtf8TextReader;
///
/// async fn read_lines<I>(input: I) -> std::io::Result<Vec<String>>
/// where
///     I: AsyncByteInput + Unpin,
/// {
///     let mut reader = AsyncUtf8TextReader::new(input);
///     let mut lines = Vec::new();
///     let mut line = String::new();
///     while AsyncTextLineReader::read_line_async(&mut reader, &mut line).await? {
///         lines.push(line.clone());
///         line.clear();
///     }
///     Ok(lines)
/// }
/// ```
#[allow(async_fn_in_trait)]
pub trait AsyncTextLineReader: AsyncTextReader {
    /// Reads one line into `output`.
    ///
    /// The line is appended to `output`; its existing contents are preserved.
    ///
    /// # Parameters
    ///
    /// - `output` - Destination string receiving the next line.
    ///
    /// # Returns
    ///
    /// Returns `true` when a line or final unterminated text was appended, or
    /// `false` at EOF when no text was appended.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific source or decoding error. A line
    /// prefix can remain in `output` when a later read fails.
    async fn read_line_async(&mut self, output: &mut String) -> Result<bool, Self::Error>;
}
