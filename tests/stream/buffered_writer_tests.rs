// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;

use qubit_codec_text::{
    CharsetEncodePolicy,
    CharsetEncoder,
};
use qubit_io_text::{
    BufferedWriter,
    LineEnding,
    TextWrite,
    Utf8Codec,
};

#[test]
fn test_buffered_writer_encodes_utf8_into_shared_output_buffer()
-> std::io::Result<()> {
    let encoder =
        CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
            .expect("strict UTF-8 encoder should be constructible");
    let mut writer =
        BufferedWriter::with_capacity(Cursor::new(Vec::new()), encoder, 1);

    writer.write_str("Aé🙂")?;
    let cursor = writer.into_inner()?;

    assert_eq!("Aé🙂".as_bytes(), cursor.into_inner().as_slice());
    Ok(())
}

#[test]
fn test_buffered_writer_applies_configured_line_ending() -> std::io::Result<()>
{
    let encoder =
        CharsetEncoder::with_policy(Utf8Codec, CharsetEncodePolicy::report())
            .expect("strict UTF-8 encoder should be constructible");
    let mut writer = BufferedWriter::new(Cursor::new(Vec::new()), encoder)
        .with_line_ending(LineEnding::CrLf);

    writer.write_line("line")?;
    let cursor = writer.into_inner()?;

    assert_eq!(b"line\r\n", cursor.into_inner().as_slice());
    Ok(())
}
