// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::io::Cursor;
use std::io::Read;

use qubit_codec::ByteOrder;
use qubit_codec_text::CharsetCodec;
use qubit_codec_text::CharsetDecodePolicy;
use qubit_codec_text::Utf16ByteCodec;
use qubit_codec_text::Utf32ByteCodec;
use qubit_io_text::CharsetTextReader;
use qubit_io_text::TextReader;

/// An input that caps each underlying read to exercise arbitrary byte splits.
struct ShortReadInput {
    inner: Cursor<Vec<u8>>,
    limit: usize,
}

impl ShortReadInput {
    fn new(bytes: &[u8], limit: usize) -> Self {
        Self {
            inner: Cursor::new(bytes.to_vec()),
            limit,
        }
    }
}

impl Read for ShortReadInput {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = output.len().min(self.limit);
        self.inner.read(&mut output[..count])
    }
}

#[derive(Clone, Copy)]
enum Encoding {
    Utf16Le,
    Utf16Be,
    Utf32Le,
    Utf32Be,
}

impl Encoding {
    fn decode_reference(self, bytes: &[u8]) -> Option<String> {
        match self {
            Self::Utf16Le | Self::Utf16Be => {
                if bytes.len() % 2 != 0 {
                    return None;
                }
                let units = bytes.chunks_exact(2).map(|pair| match self {
                    Self::Utf16Le => u16::from_le_bytes([pair[0], pair[1]]),
                    Self::Utf16Be => u16::from_be_bytes([pair[0], pair[1]]),
                    _ => unreachable!(),
                });
                char::decode_utf16(units).collect::<Result<String, _>>().ok()
            }
            Self::Utf32Le | Self::Utf32Be => {
                if bytes.len() % 4 != 0 {
                    return None;
                }
                let mut text = String::new();
                for chunk in bytes.chunks_exact(4) {
                    let unit = match self {
                        Self::Utf32Le => u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]),
                        Self::Utf32Be => u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]),
                        _ => unreachable!(),
                    };
                    text.push(char::from_u32(unit)?);
                }
                Some(text)
            }
        }
    }

    fn check<C: CharsetCodec<Unit = u8> + Copy>(self, bytes: &[u8], codec: C) {
        let expected = self.decode_reference(bytes);
        for capacity in 1..=8 {
            for short_read in 1..=4 {
                let input = ShortReadInput::new(bytes, short_read);
                let mut reader =
                    CharsetTextReader::new_with_buffer_capacity(input, codec, CharsetDecodePolicy::report(), capacity);
                let mut actual = String::new();
                match &expected {
                    Some(expected) => {
                        let count = reader
                            .read_to_string(&mut actual)
                            .expect("reference-valid fixed-width input must decode");
                        assert_eq!(expected, &actual);
                        assert_eq!(expected.chars().count(), count);
                    }
                    None => {
                        let error = reader
                            .read_to_string(&mut actual)
                            .expect_err("invalid or truncated fixed-width input must fail");
                        assert_eq!(io::ErrorKind::InvalidData, error.kind());
                    }
                }
            }
        }
    }
}

/// Checks arbitrary bytes against independent standard-library UTF-16/32
/// references.
pub fn fuzz_multibyte_reference(bytes: &[u8]) {
    Encoding::Utf16Le.check(bytes, Utf16ByteCodec::new(ByteOrder::LittleEndian));
    Encoding::Utf16Be.check(bytes, Utf16ByteCodec::new(ByteOrder::BigEndian));
    Encoding::Utf32Le.check(bytes, Utf32ByteCodec::new(ByteOrder::LittleEndian));
    Encoding::Utf32Be.check(bytes, Utf32ByteCodec::new(ByteOrder::BigEndian));
}

#[cfg(test)]
mod tests {
    use super::fuzz_multibyte_reference;

    #[test]
    fn test_fixed_width_reference_seed_matrix() {
        let seeds: &[&[u8]] = &[
            &[],
            &[0x41, 0x00, 0x2d, 0x4e], // UTF-16LE: A中
            &[0x00, 0x41, 0x4e, 0x2d], // UTF-16BE: A中
            &[0x3d, 0xd8, 0x42, 0xde], // UTF-16LE: 🙂
            &[0xd8, 0x3d, 0xde, 0x42], // UTF-16BE: 🙂
            &[0xff, 0xdb, 0xff, 0xdf], // UTF-16LE: U+10FFFF
            &[0xdb, 0xff, 0xdf, 0xff], // UTF-16BE: U+10FFFF
            &[0xff, 0xfe],             // BOM is U+FEFF, not a signature to strip
            &[0xfe, 0xff],
            &[0xff, 0xfe, 0x00, 0x00],
            &[0x00, 0x00, 0xfe, 0xff],
            &[0x00],                         // odd UTF-16 tail
            &[0x00, 0xd8],                   // isolated high surrogate
            &[0x00, 0xdc],                   // isolated low surrogate
            &[0x00, 0xdc, 0x00, 0xd8],       // reversed low/high surrogate pair
            &[0x41, 0x00, 0x00],             // UTF-16 valid prefix plus odd tail
            &[0x00, 0xd8, 0x00, 0x00],       // UTF-32LE surrogate
            &[0x00, 0x00, 0xd8, 0x00],       // UTF-32BE surrogate
            &[0x00, 0x00, 0x11, 0x00],       // UTF-32LE above U+10FFFF
            &[0x00, 0x11, 0x00, 0x00],       // UTF-32BE above U+10FFFF
            &[0x00, 0x00, 0x00, 0x41],       // UTF-32BE A
            &[0x41, 0x00, 0x00, 0x00],       // UTF-32LE A
            &[0xff, 0xff, 0x10, 0x00],       // UTF-32LE: U+10FFFF
            &[0x00, 0x10, 0xff, 0xff],       // UTF-32BE: U+10FFFF
            &[0x00, 0x00, 0x00, 0x00, 0x41], // truncated UTF-32 tail
        ];
        for seed in seeds {
            fuzz_multibyte_reference(seed);
        }
    }
}
