// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_codec_text::CharsetDecodePolicy;
use qubit_codec_text::MalformedAction;

/// Fixed-width encoding interpreted by this independent reference decoder.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ReferenceEncoding {
    Utf16Le,
    Utf16Be,
    Utf32Le,
    Utf32Be,
}

/// Decoded text and the byte offset of the first reported malformed segment.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ReferenceText {
    pub(crate) text: String,
    pub(crate) error_offset: Option<usize>,
}

/// Decodes closed fixed-width input using standard-library scalar conversion.
pub(crate) fn decode_reference(
    input: &[u8],
    encoding: ReferenceEncoding,
    policy: CharsetDecodePolicy,
) -> ReferenceText {
    let mut result = ReferenceText {
        text: String::new(),
        error_offset: None,
    };
    let mut offset = 0;
    let is_utf16 = matches!(encoding, ReferenceEncoding::Utf16Le | ReferenceEncoding::Utf16Be);

    while offset < input.len() {
        let remaining = input.len() - offset;
        let unit_size = if is_utf16 { 2 } else { 4 };
        if remaining < unit_size {
            handle_error(&mut result, policy, offset);
            break;
        }

        if is_utf16 {
            let first = read_u16(&input[offset..offset + 2], encoding);
            if (0xd800..=0xdbff).contains(&first) {
                if remaining < 4 {
                    handle_error(&mut result, policy, offset);
                    break;
                }
                let second = read_u16(&input[offset + 2..offset + 4], encoding);
                if (0xdc00..=0xdfff).contains(&second) {
                    let scalar = 0x1_0000 + (((first as u32 - 0xd800) << 10) | (second as u32 - 0xdc00));
                    result
                        .text
                        .push(char::from_u32(scalar).expect("valid UTF-16 surrogate pair"));
                    offset += 4;
                } else {
                    handle_error(&mut result, policy, offset + 2);
                    if result.error_offset.is_some() {
                        break;
                    }
                    offset += 2;
                }
            } else if (0xdc00..=0xdfff).contains(&first) {
                handle_error(&mut result, policy, offset);
                if result.error_offset.is_some() {
                    break;
                }
                offset += 2;
            } else {
                result
                    .text
                    .push(char::from_u32(first as u32).expect("non-surrogate UTF-16 unit"));
                offset += 2;
            }
        } else {
            let scalar = read_u32(&input[offset..offset + 4], encoding);
            if let Some(character) = char::from_u32(scalar) {
                result.text.push(character);
                offset += 4;
            } else {
                handle_error(&mut result, policy, offset);
                if result.error_offset.is_some() {
                    break;
                }
                offset += 4;
            }
        }
    }
    result
}

fn handle_error(result: &mut ReferenceText, policy: CharsetDecodePolicy, offset: usize) {
    match policy.malformed_action() {
        MalformedAction::Report => result.error_offset = Some(offset),
        MalformedAction::Replace => result.text.push(policy.replacement()),
        MalformedAction::Ignore => {}
    }
}

fn read_u16(bytes: &[u8], encoding: ReferenceEncoding) -> u16 {
    let pair = [bytes[0], bytes[1]];
    match encoding {
        ReferenceEncoding::Utf16Le => u16::from_le_bytes(pair),
        ReferenceEncoding::Utf16Be => u16::from_be_bytes(pair),
        _ => unreachable!("UTF-16 encoding expected"),
    }
}

fn read_u32(bytes: &[u8], encoding: ReferenceEncoding) -> u32 {
    let quad = [bytes[0], bytes[1], bytes[2], bytes[3]];
    match encoding {
        ReferenceEncoding::Utf32Le => u32::from_le_bytes(quad),
        ReferenceEncoding::Utf32Be => u32::from_be_bytes(quad),
        _ => unreachable!("UTF-32 encoding expected"),
    }
}

#[cfg(test)]
mod tests {
    use qubit_codec_text::CharsetDecodePolicy;

    use super::ReferenceEncoding;
    use super::ReferenceText;
    use super::decode_reference;

    fn check(
        input: &[u8],
        encoding: ReferenceEncoding,
        policy: CharsetDecodePolicy,
        text: &str,
        error_offset: Option<usize>,
    ) {
        assert_eq!(
            ReferenceText {
                text: text.to_owned(),
                error_offset
            },
            decode_reference(input, encoding, policy)
        );
    }

    #[test]
    fn test_utf16_le_policy_literal_matrix() {
        use ReferenceEncoding::Utf16Le;
        check(
            &[0x41, 0, 0x2d, 0x4e],
            Utf16Le,
            CharsetDecodePolicy::report(),
            "A中",
            None,
        );
        check(&[0, 0xd8, 0x41, 0], Utf16Le, CharsetDecodePolicy::report(), "", Some(2));
        check(&[0, 0xdc, 0x41, 0], Utf16Le, CharsetDecodePolicy::report(), "", Some(0));
        check(
            &[0x41, 0, 0, 0xd8],
            Utf16Le,
            CharsetDecodePolicy::report(),
            "A",
            Some(2),
        );
        check(&[0, 0xd8, 0x41], Utf16Le, CharsetDecodePolicy::report(), "", Some(0));
        check(&[0x41, 0, 0x7f], Utf16Le, CharsetDecodePolicy::report(), "A", Some(2));
        check(
            &[0, 0xd8, 0x41, 0],
            Utf16Le,
            CharsetDecodePolicy::replace('!'),
            "!A",
            None,
        );
        check(
            &[0, 0xdc, 0x41, 0],
            Utf16Le,
            CharsetDecodePolicy::replace('!'),
            "!A",
            None,
        );
        check(
            &[0x41, 0, 0, 0xd8],
            Utf16Le,
            CharsetDecodePolicy::replace('!'),
            "A!",
            None,
        );
        check(&[0, 0xd8, 0x41], Utf16Le, CharsetDecodePolicy::replace('!'), "!", None);
        check(&[0x41, 0, 0x7f], Utf16Le, CharsetDecodePolicy::replace('!'), "A!", None);
        check(&[0, 0xd8, 0x41, 0], Utf16Le, CharsetDecodePolicy::ignore(), "A", None);
        check(&[0, 0xdc, 0x41, 0], Utf16Le, CharsetDecodePolicy::ignore(), "A", None);
        check(&[0x41, 0, 0, 0xd8], Utf16Le, CharsetDecodePolicy::ignore(), "A", None);
        check(&[0, 0xd8, 0x41], Utf16Le, CharsetDecodePolicy::ignore(), "", None);
        check(&[0x41, 0, 0x7f], Utf16Le, CharsetDecodePolicy::ignore(), "A", None);
    }

    #[test]
    fn test_utf16_endianness_and_special_literals() {
        check(
            &[0, 0x41, 0x4e, 0x2d],
            ReferenceEncoding::Utf16Be,
            CharsetDecodePolicy::report(),
            "A中",
            None,
        );
        check(
            &[0x3d, 0xd8, 0x42, 0xde],
            ReferenceEncoding::Utf16Le,
            CharsetDecodePolicy::report(),
            "🙂",
            None,
        );
        check(
            &[0xff, 0xfe, 0xfd, 0xff],
            ReferenceEncoding::Utf16Le,
            CharsetDecodePolicy::report(),
            "\u{feff}\u{fffd}",
            None,
        );
        check(
            &[0, 0xd8, 0, 0xd8, 0x41, 0],
            ReferenceEncoding::Utf16Le,
            CharsetDecodePolicy::replace('!'),
            "!!A",
            None,
        );
        check(
            &[0, 0xd8, 0x41, 0],
            ReferenceEncoding::Utf16Le,
            CharsetDecodePolicy::replace('\n'),
            "\nA",
            None,
        );
        check(
            &[0, 0x41, 0xd8, 0],
            ReferenceEncoding::Utf16Be,
            CharsetDecodePolicy::report(),
            "A",
            Some(2),
        );
    }

    #[test]
    fn test_utf32_policy_and_tail_literals() {
        use ReferenceEncoding::Utf32Be;
        use ReferenceEncoding::Utf32Le;
        check(&[0x41, 0, 0, 0], Utf32Le, CharsetDecodePolicy::report(), "A", None);
        check(&[0, 0, 0, 0x41], Utf32Be, CharsetDecodePolicy::report(), "A", None);
        check(&[0, 0xd8, 0, 0], Utf32Le, CharsetDecodePolicy::report(), "", Some(0));
        check(&[0, 0, 0x11, 0], Utf32Le, CharsetDecodePolicy::report(), "", Some(0));
        check(&[0xfd, 0xff, 0, 0], Utf32Le, CharsetDecodePolicy::report(), "�", None);
        check(
            &[0x41, 0, 0, 0, 0],
            Utf32Le,
            CharsetDecodePolicy::report(),
            "A",
            Some(4),
        );
        check(
            &[0x41, 0, 0, 0, 0, 0],
            Utf32Le,
            CharsetDecodePolicy::report(),
            "A",
            Some(4),
        );
        check(
            &[0x41, 0, 0, 0, 0, 0, 0],
            Utf32Le,
            CharsetDecodePolicy::report(),
            "A",
            Some(4),
        );
        check(
            &[0, 0, 0, 0x41, 0, 0, 0xd8, 0],
            Utf32Be,
            CharsetDecodePolicy::replace('!'),
            "A!",
            None,
        );
        check(
            &[0, 0, 0, 0x41, 0, 0, 0xd8, 0, 0, 0, 0, 0x42],
            Utf32Be,
            CharsetDecodePolicy::ignore(),
            "AB",
            None,
        );
    }
}
