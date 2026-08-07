// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io_text::LineEnding;
use qubit_io_text::LineEndingSet;
#[cfg(coverage)]
use qubit_io_text::StrTextReader;
#[cfg(coverage)]
use qubit_io_text::TextLineRead;

#[test]
fn test_line_ending_set_defaults_to_all_common_endings() {
    let endings = LineEndingSet::default();

    assert!(endings.contains(LineEnding::Lf));
    assert!(endings.contains(LineEnding::CrLf));
    assert!(endings.contains(LineEnding::Cr));
    assert_eq!(LineEndingSet::ALL, endings);
}

#[test]
fn test_line_ending_set_can_be_composed_and_reduced() {
    let endings = LineEndingSet::LF
        .with(LineEnding::CrLf)
        .without(LineEnding::Lf);

    assert!(endings.contains(LineEnding::CrLf));
    assert!(!endings.contains(LineEnding::Lf));
    assert!(!endings.contains(LineEnding::Cr));
    assert!(!endings.is_empty());
    assert!(
        LineEndingSet::ALL
            .without(LineEnding::Lf)
            .without(LineEnding::CrLf)
            .without(LineEnding::Cr)
            .is_empty()
    );
    assert_eq!(LineEndingSet::LF, LineEndingSet::only(LineEnding::Lf));
    assert_eq!(LineEndingSet::CRLF, LineEndingSet::only(LineEnding::CrLf));
    assert_eq!(LineEndingSet::CR, LineEndingSet::only(LineEnding::Cr));
}

#[cfg(coverage)]
#[test]
fn test_line_ending_reader_covers_cr_only_boundary() {
    let mut reader =
        StrTextReader::new("prefix\rtail").with_line_endings(LineEndingSet::CR);
    let mut line = String::new();

    assert!(reader.read_line(&mut line).is_ok_and(|read| read));
    assert_eq!("prefix\r", line);
}
