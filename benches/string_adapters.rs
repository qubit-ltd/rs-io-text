// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Timing and allocation benchmarks for owned string adapters.

#[path = "support/tracking_allocator.rs"]
mod tracking_allocator;

use std::hint::black_box;
use std::io::Cursor;
use std::time::Duration;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_codec::ByteOrder;
use qubit_codec_text::CharsetCodec;
use qubit_codec_text::CharsetDecodePolicy;
use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::Utf8Codec;
use qubit_codec_text::Utf16ByteCodec;
use qubit_codec_text::Utf32ByteCodec;
use qubit_io_text::CharsetStringDecoder;
use qubit_io_text::CharsetStringEncoder;
use qubit_io_text::CharsetTextReader;
use qubit_io_text::CharsetTextWriter;
use qubit_io_text::LineEnding;
use qubit_io_text::TextReader;
use qubit_io_text::TextWriter;
use qubit_io_text::Utf8TextWriter;

use crate::tracking_allocator::measure_peak;
use crate::tracking_allocator::measure_peak_and_allocations;

const FIXTURE_REPEAT: usize = 2_048;
const SAMPLE_SIZE: usize = 20;

fn fixtures() -> [(String, String); 3] {
    [
        (
            String::from("ascii"),
            "Codec throughput 0123456789\n".repeat(FIXTURE_REPEAT),
        ),
        (
            String::from("multilingual"),
            "English 中文 Ελληνικά العربية हिन्दी\n".repeat(FIXTURE_REPEAT),
        ),
        (
            String::from("supplementary"),
            "Rust 🦀 rocket 🚀 music 𝄞 globe 🌍\n".repeat(FIXTURE_REPEAT),
        ),
    ]
}

fn encode_bytes<C>(codec: C, input: &str) -> Vec<C::Unit>
where
    C: CharsetCodec,
    C::Unit: Default,
{
    CharsetStringEncoder::new(codec)
        .encode_str(input)
        .expect("valid fixture should encode")
}

fn report_allocation<C>(encoding: &str, codec: C, fixture_name: &str, input: &str)
where
    C: CharsetCodec + Clone,
    C::Unit: Default,
{
    let mut encoder = CharsetStringEncoder::new(codec.clone());
    let (encoded, encode_peak) = measure_peak(|| encoder.encode_str(input).expect("valid fixture should encode"));
    println!(
        "allocation encode/{encoding}/{fixture_name}: output_len={}, \
         output_capacity={}, peak_bytes={encode_peak}",
        encoded.len(),
        encoded.capacity(),
    );

    let mut decoder = CharsetStringDecoder::new(codec);
    let (decoded, decode_peak) =
        measure_peak(|| decoder.decode_to_string(&encoded).expect("valid fixture should decode"));
    println!(
        "allocation decode/{encoding}/{fixture_name}: output_len={}, \
         output_capacity={}, peak_bytes={decode_peak}",
        decoded.len(),
        decoded.capacity(),
    );
}

fn bench_owned_encode<C>(criterion: &mut Criterion, encoding: &str, codec: C, fixtures: &[(String, String)])
where
    C: CharsetCodec + Clone,
    C::Unit: Default,
{
    let mut group = criterion.benchmark_group("owned_string_encode");
    group.sample_size(SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));
    for (fixture_name, input) in fixtures {
        let mut encoder = CharsetStringEncoder::new(codec.clone());
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new(encoding, fixture_name), input, |bencher, input| {
            bencher.iter(|| {
                let output = encoder
                    .encode_str(black_box(input))
                    .expect("valid fixture should encode");
                black_box((output.len(), output.capacity()));
            });
        });
    }
    group.finish();
}

fn bench_owned_decode<C>(criterion: &mut Criterion, encoding: &str, codec: C, fixtures: &[(String, String)])
where
    C: CharsetCodec + Clone,
    C::Unit: Default,
{
    let encoded: Vec<Vec<C::Unit>> = fixtures
        .iter()
        .map(|(_, input)| encode_bytes(codec.clone(), input))
        .collect();
    let mut group = criterion.benchmark_group("owned_string_decode");
    group.sample_size(SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));
    for ((fixture_name, input), encoded) in fixtures.iter().zip(&encoded) {
        let mut decoder = CharsetStringDecoder::new(codec.clone());
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new(encoding, fixture_name), encoded, |bencher, encoded| {
            bencher.iter(|| {
                let output = decoder
                    .decode_to_string(black_box(encoded))
                    .expect("valid fixture should decode");
                black_box((output.len(), output.capacity()));
            });
        });
    }
    group.finish();
}

fn bench_streaming_charset(criterion: &mut Criterion, fixtures: &[(String, String)]) {
    let mut group = criterion.benchmark_group("streaming_charset_utf8");
    group.sample_size(SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));

    for (fixture_name, input) in fixtures {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("read", fixture_name), input, |bencher, input| {
            bencher.iter_batched(
                || Cursor::new(input.as_bytes().to_vec()),
                |input| {
                    let mut reader = CharsetTextReader::new(input, Utf8Codec, CharsetDecodePolicy::report());
                    let mut output = String::new();
                    reader.read_to_string(&mut output).expect("UTF-8 stream should decode");
                    black_box(output);
                },
                criterion::BatchSize::SmallInput,
            );
        });
        group.bench_with_input(BenchmarkId::new("write", fixture_name), input, |bencher, input| {
            bencher.iter_batched(
                || Cursor::new(Vec::with_capacity(input.len())),
                |output| {
                    let mut writer = CharsetTextWriter::new(output, Utf8Codec, CharsetEncodePolicy::report());
                    writer.write_str(input).expect("UTF-8 stream should encode");
                    writer.finish().expect("UTF-8 stream should finish");
                    let (output, pending) = writer.into_parts();
                    let _ = black_box((output.into_inner(), pending));
                },
                criterion::BatchSize::SmallInput,
            );
        });
        group.bench_with_input(
            BenchmarkId::new("write_fast_utf8", fixture_name),
            input,
            |bencher, input| {
                bencher.iter_batched(
                    || Cursor::new(Vec::with_capacity(input.len())),
                    |output| {
                        let mut writer = Utf8TextWriter::new(output);
                        writer.write_str(input).expect("UTF-8 stream should encode");
                        writer.finish().expect("UTF-8 stream should finish");
                        let (output, pending) = writer.into_parts();
                        let _ = black_box((output.into_inner(), pending));
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

fn line_fixtures() -> [(String, String); 4] {
    [
        (String::from("short_ascii"), String::from("hello 123")),
        (String::from("long_ascii"), "ASCII payload 0123456789 ".repeat(256)),
        (
            String::from("multilingual"),
            "English 中文 Ελληνικά العربية हिन्दी".repeat(16),
        ),
        (
            String::from("supplementary"),
            "Rust 🦀 rocket 🚀 music 𝄞 globe 🌍".repeat(16),
        ),
    ]
}

const LINE_BUFFER_CAPACITY: usize = 8 * 1024;

fn line_writer(output_capacity: usize, ending: LineEnding) -> Utf8TextWriter<Cursor<Vec<u8>>> {
    Utf8TextWriter::with_capacity(Cursor::new(Vec::with_capacity(output_capacity)), LINE_BUFFER_CAPACITY)
        .with_line_ending(ending)
}

fn completed_line_output(writer: Utf8TextWriter<Cursor<Vec<u8>>>) -> Vec<u8> {
    let (output, _recovery) = writer.into_parts();
    output.into_inner()
}

fn bench_utf8_lines(criterion: &mut Criterion) {
    let fixtures = line_fixtures();
    let endings = [
        ("lf", LineEnding::Lf),
        ("crlf", LineEnding::CrLf),
        ("cr", LineEnding::Cr),
    ];
    let mut group = criterion.benchmark_group("utf8_line");
    group.sample_size(SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));

    for (fixture_name, line) in &fixtures {
        for (ending_name, ending) in endings {
            let expected = format!("{}{ending}", line, ending = ending.as_str()).into_bytes();
            let mut writer_line = line_writer(expected.len(), ending);
            writer_line.write_line(line).expect("valid UTF-8 line should write");
            writer_line.finish().expect("UTF-8 line writer should finish");
            let line_output = completed_line_output(writer_line);

            let mut split_writer = line_writer(expected.len(), ending);
            split_writer.write_str(line).expect("valid UTF-8 body should write");
            split_writer
                .write_str(ending.as_str())
                .expect("UTF-8 line ending should write");
            split_writer.finish().expect("UTF-8 split writer should finish");
            let split_output = completed_line_output(split_writer);
            assert_eq!(
                line_output, expected,
                "write_line output mismatch: {fixture_name}/{ending_name}"
            );
            assert_eq!(
                split_output, expected,
                "split-write output mismatch: {fixture_name}/{ending_name}"
            );
            assert_eq!(
                line_output, split_output,
                "write paths differ: {fixture_name}/{ending_name}"
            );

            group.throughput(Throughput::Bytes(expected.len() as u64));
            let id = format!("{fixture_name}_{ending_name}");
            group.bench_with_input(BenchmarkId::new("write_line", &id), line, |bencher, line| {
                bencher.iter_batched(
                    || line_writer(expected.len(), ending),
                    |mut writer| {
                        writer
                            .write_line(black_box(line))
                            .expect("valid UTF-8 line should write");
                        writer.finish().expect("UTF-8 line writer should finish");
                        black_box(completed_line_output(writer));
                    },
                    criterion::BatchSize::SmallInput,
                );
            });
            group.bench_with_input(BenchmarkId::new("two_write_str", &id), line, |bencher, line| {
                bencher.iter_batched(
                    || line_writer(expected.len(), ending),
                    |mut writer| {
                        writer
                            .write_str(black_box(line))
                            .expect("valid UTF-8 body should write");
                        writer
                            .write_str(ending.as_str())
                            .expect("UTF-8 line ending should write");
                        writer.finish().expect("UTF-8 split writer should finish");
                        black_box(completed_line_output(writer));
                    },
                    criterion::BatchSize::SmallInput,
                );
            });
        }
    }
    group.finish();
}

fn allocation_operations() -> [(&'static str, fn()); 7] {
    [
        ("construct_finish", || {
            let mut writer = Utf8TextWriter::with_capacity(std::io::sink(), 16);
            writer.finish().expect("empty UTF-8 writer should finish");
            black_box(writer);
        }),
        ("utf8_write_str", || {
            let mut writer = Utf8TextWriter::with_capacity(std::io::sink(), 16);
            writer.write_str("hello 中文").expect("UTF-8 string should write");
            writer.finish().expect("UTF-8 writer should finish");
            black_box(writer);
        }),
        ("utf8_write_line", || {
            let mut writer = Utf8TextWriter::with_capacity(std::io::sink(), 16);
            writer.write_line("hello 中文").expect("UTF-8 line should write");
            writer.finish().expect("UTF-8 writer should finish");
            black_box(writer);
        }),
        ("utf8_write_char", || {
            let mut writer = Utf8TextWriter::with_capacity(std::io::sink(), 16);
            writer.write_char('🦀').expect("UTF-8 character should write");
            writer.finish().expect("UTF-8 writer should finish");
            black_box(writer);
        }),
        ("charset_first_write_str", || {
            let mut writer = CharsetTextWriter::new_with_buffer_capacity(
                std::io::sink(),
                Utf16ByteCodec::new(ByteOrder::LittleEndian),
                CharsetEncodePolicy::report(),
                16,
            );
            writer.write_str("hello 中文").expect("charset string should write");
            writer.finish().expect("charset writer should finish");
            black_box(writer);
        }),
        ("charset_repeat_write_str", || {
            let mut writer = CharsetTextWriter::new_with_buffer_capacity(
                std::io::sink(),
                Utf16ByteCodec::new(ByteOrder::LittleEndian),
                CharsetEncodePolicy::report(),
                16,
            );
            writer
                .write_str("hello 中文")
                .expect("first charset string should write");
            writer
                .write_str("hello 中文")
                .expect("repeated charset string should write");
            writer.finish().expect("charset writer should finish");
            black_box(writer);
        }),
        ("charset_direct_char", || {
            let mut writer = CharsetTextWriter::new_with_buffer_capacity(
                std::io::sink(),
                Utf16ByteCodec::new(ByteOrder::LittleEndian),
                CharsetEncodePolicy::report(),
                16,
            );
            writer.write_char('🦀').expect("charset character should write");
            writer.finish().expect("charset writer should finish");
            black_box(writer);
        }),
    ]
}

fn bench_writer_allocation(criterion: &mut Criterion) {
    let empty_first = measure_peak_and_allocations(|| ());
    let empty_second = measure_peak_and_allocations(|| ());
    assert_eq!(
        empty_first,
        ((), 0, 0),
        "first empty allocation measurement must be clean"
    );
    assert_eq!(
        empty_second,
        ((), 0, 0),
        "allocation measurement must leave no residual counts"
    );

    let operations = allocation_operations();
    for (name, operation) in operations {
        let (_, peak_bytes, allocations) = measure_peak_and_allocations(operation);
        println!("allocation writer/{name}: successful_requests={allocations}, peak_bytes={peak_bytes}");
    }

    let mut group = criterion.benchmark_group("writer_allocation");
    group.sample_size(SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));
    for (name, operation) in allocation_operations() {
        group.bench_function(name, |bencher| {
            bencher.iter(&operation);
        });
    }
    group.finish();
}

fn bench_string_adapters(criterion: &mut Criterion) {
    let fixtures = fixtures();
    let utf16 = Utf16ByteCodec::new(ByteOrder::LittleEndian);
    let utf32 = Utf32ByteCodec::new(ByteOrder::LittleEndian);

    for (fixture_name, input) in &fixtures {
        report_allocation("utf8", Utf8Codec, fixture_name, input);
        report_allocation("utf16le", utf16, fixture_name, input);
        report_allocation("utf32le", utf32, fixture_name, input);
    }

    bench_owned_encode(criterion, "utf8", Utf8Codec, &fixtures);
    bench_owned_encode(criterion, "utf16le", utf16, &fixtures);
    bench_owned_encode(criterion, "utf32le", utf32, &fixtures);
    bench_owned_decode(criterion, "utf8", Utf8Codec, &fixtures);
    bench_owned_decode(criterion, "utf16le", utf16, &fixtures);
    bench_owned_decode(criterion, "utf32le", utf32, &fixtures);
    bench_streaming_charset(criterion, &fixtures);
    bench_utf8_lines(criterion);
    bench_writer_allocation(criterion);
}

criterion_group!(benches, bench_string_adapters);
criterion_main!(benches);
