// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Classifies strict Report reader errors for fuzz assertions.

use std::io;

use qubit_codec::TranscodeDecodeError;
use qubit_codec_text::Charset;
use qubit_codec_text::CharsetDecodeError;
use qubit_codec_text::CharsetDecodeErrorKind;

/// Returns whether a Report reader error represents malformed or incomplete
/// input.
///
/// The caller must independently establish that the reference input is invalid.
/// This does not validate the diagnostic index or a specific input-error
/// subtype.
///
/// # Parameters
///
/// - `error`: Error returned by a strict charset text reader.
/// - `charset`: Charset selected by the reader's codec.
///
/// # Returns
///
/// Returns `true` only for a supported input-error kind from the expected
/// transcode error wrapper and charset.
#[must_use]
pub fn is_report_input_error(error: &io::Error, charset: Charset) -> bool {
    if error.kind() != io::ErrorKind::InvalidData {
        return false;
    }
    let Some(source) = error
        .get_ref()
        .and_then(|source| source.downcast_ref::<TranscodeDecodeError<CharsetDecodeError>>())
    else {
        return false;
    };
    let decoded = CharsetDecodeError::from_transcode_error(charset, source.clone());
    decoded.charset() == charset
        && matches!(
            decoded.kind(),
            CharsetDecodeErrorKind::MalformedSequence { .. }
                | CharsetDecodeErrorKind::IncompleteSequence { .. }
                | CharsetDecodeErrorKind::InvalidCodePoint { .. }
        )
}
