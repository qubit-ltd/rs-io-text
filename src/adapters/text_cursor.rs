// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
/// Reads the next character from a string slice at the current byte position.
///
/// # Parameters
/// - `text`: Source text.
/// - `position`: Current byte position, updated when a character is read.
///
/// # Returns
/// The next character, or `None` when `position` is at EOF.
pub(crate) fn read_char_at(text: &str, position: &mut usize) -> Option<char> {
    let ch = text.get(*position..)?.chars().next()?;
    *position += ch.len_utf8();
    Some(ch)
}

/// Reads up to `max` characters from a string slice.
///
/// # Parameters
/// - `text`: Source text.
/// - `position`: Current byte position, updated after every character.
/// - `output`: Destination character vector.
/// - `max`: Maximum number of characters to append.
///
/// # Returns
/// The number of characters appended.
pub(crate) fn read_chars_at(
    text: &str,
    position: &mut usize,
    output: &mut Vec<char>,
    max: usize,
) -> usize {
    let mut count = 0;
    while count < max {
        match read_char_at(text, position) {
            Some(ch) => {
                output.push(ch);
                count += 1;
            }
            None => break,
        }
    }
    count
}

/// Reads all remaining text from a string slice.
///
/// # Parameters
/// - `text`: Source text.
/// - `position`: Current byte position, advanced to EOF.
/// - `output`: Destination string. Remaining text is appended.
///
/// # Returns
/// The number of characters appended.
pub(crate) fn read_to_string_at(
    text: &str,
    position: &mut usize,
    output: &mut String,
) -> usize {
    let remaining = &text[*position..];
    let count = remaining.chars().count();
    output.push_str(remaining);
    *position = text.len();
    count
}

/// Reads one line from a string slice.
///
/// # Parameters
/// - `text`: Source text.
/// - `position`: Current byte position, advanced past the returned line.
/// - `output`: Destination string. The line is appended.
///
/// # Returns
/// `true` when a line was appended, or `false` at EOF.
pub(crate) fn read_line_at(
    text: &str,
    position: &mut usize,
    output: &mut String,
) -> bool {
    if *position >= text.len() {
        return false;
    }
    let remaining = &text[*position..];
    let end = match remaining.find('\n') {
        Some(index) => *position + index + '\n'.len_utf8(),
        None => text.len(),
    };
    output.push_str(&text[*position..end]);
    *position = end;
    true
}
