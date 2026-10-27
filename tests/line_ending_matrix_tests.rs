// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::fmt::Debug;
use std::future::Future;
use std::io;
use std::io::Cursor;
use std::io::Read;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use qubit_codec_text::CharsetDecodePolicy;
use qubit_codec_text::Utf8Codec;
use qubit_io::AsyncInput;
use qubit_io_text::AsyncCharsetTextReader;
use qubit_io_text::AsyncTextLineReader;
use qubit_io_text::BufferedTextReader;
use qubit_io_text::CharsetTextReader;
use qubit_io_text::InputTextReader;
use qubit_io_text::LineEnding;
use qubit_io_text::LineEndingSet;
use qubit_io_text::StrTextReader;
use qubit_io_text::StringCharInput;
use qubit_io_text::StringTextReader;
use qubit_io_text::TextLineReader;
use qubit_io_text::TextReader;
use qubit_io_text::Utf8TextReader;

const SAMPLE: &str = "a\r\nb\rc\n尾\r";

fn endings() -> [LineEndingSet; 8] {
    [
        LineEndingSet::ALL,
        LineEndingSet::LF,
        LineEndingSet::CRLF,
        LineEndingSet::CR,
        LineEndingSet::ALL
            .without(LineEnding::Lf)
            .without(LineEnding::CrLf)
            .without(LineEnding::Cr),
        LineEndingSet::LF.with(LineEnding::CrLf),
        LineEndingSet::LF.with(LineEnding::Cr),
        LineEndingSet::CRLF.with(LineEnding::Cr),
    ]
}

const RECORDS: [&[&str]; 8] = [
    &["a\r\n", "b\r", "c\n", "尾\r"],
    &["a\r\n", "b\rc\n", "尾\r"],
    &["a\r\n", "b\rc\n尾\r"],
    &["a\r", "\nb\r", "c\n尾\r"],
    &["a\r\nb\rc\n尾\r"],
    &["a\r\n", "b\rc\n", "尾\r"],
    &["a\r", "\n", "b\r", "c\n", "尾\r"],
    &["a\r\n", "b\r", "c\n尾\r"],
];

fn assert_lines<R>(mut reader: R, expected: &[&str])
where
    R: TextLineReader,
    R::Error: Debug,
{
    for expected_line in expected {
        let mut output = String::from("seed:");
        assert!(reader.read_line(&mut output).expect("read expected line"));
        assert_eq!(format!("seed:{expected_line}"), output);
    }
    let mut output = String::from("seed:");
    assert!(!reader.read_line(&mut output).expect("read EOF"));
    assert_eq!("seed:", output);
}

fn collect_lines<R>(mut reader: R) -> Vec<String>
where
    R: TextLineReader,
    R::Error: Debug,
{
    let mut lines = Vec::new();
    loop {
        let mut line = String::new();
        if !reader.read_line(&mut line).expect("read line") {
            return lines;
        }
        lines.push(line);
    }
}

struct ShortRead {
    inner: Cursor<Vec<u8>>,
    max_read: usize,
}

impl Read for ShortRead {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = output.len().min(self.max_read);
        self.inner.read(&mut output[..count])
    }
}

#[test]
fn test_all_line_ending_sets_are_distinct() {
    let sets = endings();
    let members: Vec<_> = sets
        .iter()
        .map(|set| {
            (
                set.contains(LineEnding::Lf),
                set.contains(LineEnding::CrLf),
                set.contains(LineEnding::Cr),
            )
        })
        .collect();
    let unique: std::collections::HashSet<_> = members.iter().collect();
    assert_eq!(8, unique.len());
    assert_eq!(8, members.len());
}

#[test]
fn test_line_ending_matrix_sync_readers() -> io::Result<()> {
    for (index, set) in endings().into_iter().enumerate() {
        assert_lines(StrTextReader::new(SAMPLE).with_line_endings(set), RECORDS[index]);
        assert_lines(
            StringTextReader::new(SAMPLE.to_owned()).with_line_endings(set),
            RECORDS[index],
        );
        assert_lines(
            InputTextReader::new(StringCharInput::new(SAMPLE.to_owned())).with_line_endings(set),
            RECORDS[index],
        );

        let decoder = qubit_codec_text::CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
        assert_lines(
            BufferedTextReader::with_capacity(Cursor::new(SAMPLE.as_bytes()), decoder, 4).with_line_endings(set),
            RECORDS[index],
        );
        assert_lines(
            CharsetTextReader::new(
                Cursor::new(SAMPLE.as_bytes().to_vec()),
                Utf8Codec,
                CharsetDecodePolicy::report(),
            )
            .with_line_endings(set),
            RECORDS[index],
        );
        assert_lines(
            Utf8TextReader::with_capacity(Cursor::new(SAMPLE.as_bytes().to_vec()), 4).with_line_endings(set),
            RECORDS[index],
        );
        for (max_read, capacity) in [(1, 4), (2, 8), (8, 4)] {
            let input = ShortRead {
                inner: Cursor::new(SAMPLE.as_bytes().to_vec()),
                max_read,
            };
            let decoder = qubit_codec_text::CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
            assert_lines(
                BufferedTextReader::with_capacity(input, decoder, capacity).with_line_endings(set),
                RECORDS[index],
            );
        }
    }
    Ok(())
}

#[test]
fn test_line_ending_matrix_empty_and_consecutive_terminators() -> io::Result<()> {
    const CONSECUTIVE: &str = "\r\r\n\n";
    const REPEATED_CR: &str = "\r\r";
    const TAIL_CR: &str = "tail\r";
    const CONSECUTIVE_RECORDS: [&[&str]; 8] = [
        &["\r", "\r\n", "\n"],
        &["\r\r\n", "\n"],
        &["\r\r\n", "\n"],
        &["\r", "\r", "\n\n"],
        &["\r\r\n\n"],
        &["\r\r\n", "\n"],
        &["\r", "\r", "\n", "\n"],
        &["\r", "\r\n", "\n"],
    ];

    for (index, set) in endings().into_iter().enumerate() {
        assert!(collect_lines(StrTextReader::new("").with_line_endings(set)).is_empty());
        assert_eq!(
            CONSECUTIVE_RECORDS[index],
            collect_lines(StrTextReader::new(CONSECUTIVE).with_line_endings(set))
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        );
        let repeated_cr_expected = if set.contains(LineEnding::Cr) {
            vec!["\r", "\r"]
        } else {
            vec![REPEATED_CR]
        };
        assert_eq!(
            repeated_cr_expected,
            collect_lines(StringTextReader::new(REPEATED_CR.to_owned()).with_line_endings(set))
        );
        assert_eq!(
            vec![TAIL_CR],
            collect_lines(
                BufferedTextReader::with_capacity(
                    Cursor::new(TAIL_CR.as_bytes()),
                    qubit_codec_text::CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report(),),
                    4,
                )
                .with_line_endings(set)
            )
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
        );
    }

    Ok(())
}

#[derive(Debug)]
struct ShortAsyncInput {
    bytes: Vec<u8>,
    position: usize,
    max_read: usize,
}

impl AsyncInput for ShortAsyncInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        if count == 0 {
            return Poll::Ready(Ok(0));
        }
        let read = count
            .min(self.max_read)
            .min(self.bytes.len().saturating_sub(self.position));
        output[index..index + read].copy_from_slice(&self.bytes[self.position..self.position + read]);
        self.position += read;
        Poll::Ready(Ok(read))
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = std::task::Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    for _ in 0..65_536 {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
    }
    panic!("async line read exceeded the poll budget");
}

#[test]
fn test_line_ending_matrix_async_readers() -> io::Result<()> {
    for (index, set) in endings().into_iter().enumerate() {
        let input = ShortAsyncInput {
            bytes: SAMPLE.as_bytes().to_vec(),
            position: 0,
            max_read: 1 + index % 3,
        };
        let mut reader =
            AsyncCharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report()).with_line_endings(set);
        for expected in RECORDS[index] {
            let mut output = String::from("seed:");
            assert!(block_on(reader.read_line_async(&mut output)).expect("read async line"));
            assert_eq!(format!("seed:{expected}"), output);
        }
        let mut output = String::from("seed:");
        assert!(!block_on(reader.read_line_async(&mut output)).expect("read async EOF"));
        assert_eq!("seed:", output);

        let input = ShortAsyncInput {
            bytes: SAMPLE.as_bytes().to_vec(),
            position: 0,
            max_read: 2,
        };
        let mut reader = qubit_io_text::AsyncUtf8TextReader::new(input).with_line_endings(set);
        let mut actual = Vec::new();
        loop {
            let mut output = String::new();
            if !block_on(reader.read_line_async(&mut output))? {
                break;
            }
            actual.push(output);
        }
        assert_eq!(RECORDS[index], actual.iter().map(String::as_str).collect::<Vec<_>>());
    }
    Ok(())
}

#[test]
fn test_line_ending_matrix_limited_readers() -> io::Result<()> {
    for (index, set) in endings().into_iter().enumerate() {
        for record_index in 0..RECORDS[index].len() {
            let record = RECORDS[index][record_index];
            for limit in [0, 1, 2, 3, 4, SAMPLE.len()] {
                let sync_input = ShortRead {
                    inner: Cursor::new(SAMPLE.as_bytes().to_vec()),
                    max_read: 1 + limit % 8,
                };
                let decoder = qubit_codec_text::CharsetDecoder::with_policy(Utf8Codec, CharsetDecodePolicy::report());
                let mut sync_reader = BufferedTextReader::with_capacity(sync_input, decoder, 4).with_line_endings(set);
                for expected in &RECORDS[index][..record_index] {
                    let mut previous = String::new();
                    assert!(sync_reader.read_line_limited(&mut previous, SAMPLE.len())?);
                    assert_eq!(*expected, previous);
                }

                let mut output = String::from("seed:");
                if limit >= record.len() {
                    assert!(sync_reader.read_line_limited(&mut output, limit)?);
                    assert_eq!(format!("seed:{record}"), output);
                } else {
                    let error = sync_reader
                        .read_line_limited(&mut output, limit)
                        .expect_err("a record longer than the limit must fail");
                    assert_eq!(io::ErrorKind::InvalidData, error.kind());
                    assert_eq!("seed:", output);
                }
                if let Some(next) = RECORDS[index].get(record_index + 1) {
                    output.clear();
                    assert!(sync_reader.read_line_limited(&mut output, SAMPLE.len())?);
                    assert_eq!(*next, output);
                } else {
                    assert!(!sync_reader.read_line_limited(&mut output, SAMPLE.len())?);
                    assert_eq!(
                        if limit >= record.len() {
                            format!("seed:{record}")
                        } else {
                            "seed:".to_owned()
                        },
                        output
                    );
                }

                let async_input = ShortAsyncInput {
                    bytes: SAMPLE.as_bytes().to_vec(),
                    position: 0,
                    max_read: 1 + limit % 8,
                };
                let mut async_reader = AsyncCharsetTextReader::new_with_buffer_capacity(
                    async_input,
                    Utf8Codec,
                    CharsetDecodePolicy::report(),
                    4,
                )
                .with_line_endings(set);
                for expected in &RECORDS[index][..record_index] {
                    let mut previous = String::new();
                    assert!(block_on(
                        async_reader.read_line_limited_async(&mut previous, SAMPLE.len())
                    )?);
                    assert_eq!(*expected, previous);
                }

                let mut output = String::from("seed:");
                if limit >= record.len() {
                    assert!(block_on(async_reader.read_line_limited_async(&mut output, limit))?);
                    assert_eq!(format!("seed:{record}"), output);
                } else {
                    let error = block_on(async_reader.read_line_limited_async(&mut output, limit))
                        .expect_err("a record longer than the limit must fail");
                    assert_eq!(io::ErrorKind::InvalidData, error.kind());
                    assert_eq!("seed:", output);
                }
                if let Some(next) = RECORDS[index].get(record_index + 1) {
                    output.clear();
                    assert!(block_on(
                        async_reader.read_line_limited_async(&mut output, SAMPLE.len())
                    )?);
                    assert_eq!(*next, output);
                } else {
                    assert!(!block_on(
                        async_reader.read_line_limited_async(&mut output, SAMPLE.len())
                    )?);
                    assert_eq!(
                        if limit >= record.len() {
                            format!("seed:{record}")
                        } else {
                            "seed:".to_owned()
                        },
                        output
                    );
                }
            }
        }
    }

    for index in [5, 6] {
        let set = endings()[index];
        let first_record = RECORDS[index][0];
        for limit in [first_record.len().saturating_sub(1), first_record.len()] {
            let mut sync_reader =
                Utf8TextReader::with_capacity(Cursor::new(SAMPLE.as_bytes().to_vec()), 4).with_line_endings(set);
            let mut output = String::from("seed:");
            if limit < first_record.len() {
                let error = sync_reader
                    .read_line_limited(&mut output, limit)
                    .expect_err("UTF-8 wrapper must forward the append limit");
                assert_eq!(io::ErrorKind::InvalidData, error.kind());
                assert_eq!("seed:", output);
            } else {
                assert!(sync_reader.read_line_limited(&mut output, limit)?);
                assert_eq!(format!("seed:{first_record}"), output);
            }

            let async_input = ShortAsyncInput {
                bytes: SAMPLE.as_bytes().to_vec(),
                position: 0,
                max_read: 1,
            };
            let mut async_reader = qubit_io_text::AsyncUtf8TextReader::new(async_input).with_line_endings(set);
            let mut output = String::from("seed:");
            if limit < first_record.len() {
                let error = block_on(async_reader.read_line_limited_async(&mut output, limit))
                    .expect_err("async UTF-8 wrapper must forward the append limit");
                assert_eq!(io::ErrorKind::InvalidData, error.kind());
                assert_eq!("seed:", output);
            } else {
                assert!(block_on(async_reader.read_line_limited_async(&mut output, limit))?);
                assert_eq!(format!("seed:{first_record}"), output);
            }
        }
    }
    Ok(())
}

#[test]
fn test_line_ending_matrix_mixed_reads() {
    let set = LineEndingSet::CR;
    let mut reader = StrTextReader::new("a\r中z").with_line_endings(set);
    let mut line = String::new();
    assert!(reader.read_line(&mut line).expect("read first line"));
    assert_eq!("a\r", line);
    assert_eq!(Some('中'), reader.read_char().expect("read next character"));
    let mut rest = String::new();
    assert_eq!(1, reader.read_to_string(&mut rest).expect("read remaining text"));
    assert_eq!("z", rest);

    let mut reader = StringTextReader::new("a\r中z".to_owned()).with_line_endings(set);
    let mut line = String::new();
    assert!(reader.read_line(&mut line).expect("read first line"));
    assert_eq!("a\r", line);
    let mut chars = Vec::new();
    assert_eq!(2, reader.read_chars(&mut chars, 2).expect("read remaining characters"));
    assert_eq!(['中', 'z'], chars.as_slice());
}
