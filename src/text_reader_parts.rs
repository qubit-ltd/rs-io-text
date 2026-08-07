// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
//    You may obtain a copy of the License at
//
//        http://www.apache.org/licenses/LICENSE-2.0
//
//    Unless required by applicable law or agreed to in writing, software
//    distributed under the License is distributed on an "AS IS" BASIS,
//    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//    See the License for the specific language governing permissions and
//    limitations under the License.
// =============================================================================
//! Named components returned when a buffered text reader is consumed.

use qubit_io::Buffer;

/// The components of a buffered text reader that may contain unread state.
///
/// The input can be physically ahead of `unread_bytes`; consume the byte
/// buffer before reading from `input`. Process `pending_chars` before reading
/// more decoded text. The decoder is returned without being finalized.
#[must_use = "all returned reader state must be handled"]
#[derive(Debug)]
pub struct TextReaderParts<I, D> {
    /// The wrapped byte input.
    pub input: I,
    /// Encoded bytes already read from `input` but not consumed by the decoder.
    pub unread_bytes: Buffer<u8>,
    /// The decoder retaining codec state across input boundaries.
    pub decoder: D,
    /// Decoded characters already available to the caller.
    pub pending_chars: Vec<char>,
}
