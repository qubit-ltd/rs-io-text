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

use std::{
    hint::black_box,
    io::Cursor,
    time::Duration,
};

use criterion::{
    BenchmarkId,
    Criterion,
    Throughput,
    criterion_group,
    criterion_main,
};
use qubit_codec::ByteOrder;
use qubit_codec_text::{
    CharsetCodec,
    Utf8Codec,
    Utf16ByteCodec,
    Utf32ByteCodec,
};
use qubit_io_text::{
    CharsetStringDecoder,
    CharsetStringEncoder,
    CharsetTextReader,
    CharsetTextWriter,
    CodingErrorPolicy,
    TextRead,
    TextWrite,
};

use crate::tracking_allocator::measure_peak;

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

fn report_allocation<C>(
    encoding: &str,
    codec: C,
    fixture_name: &str,
    input: &str,
) where
    C: CharsetCodec + Clone,
    C::Unit: Default,
{
    let mut encoder = CharsetStringEncoder::new(codec.clone());
    let (encoded, encode_peak) = measure_peak(|| {
        encoder
            .encode_str(input)
            .expect("valid fixture should encode")
    });
    println!(
        "allocation encode/{encoding}/{fixture_name}: output_len={}, \
         output_capacity={}, peak_bytes={encode_peak}",
        encoded.len(),
        encoded.capacity(),
    );

    let mut decoder = CharsetStringDecoder::new(codec);
    let (decoded, decode_peak) = measure_peak(|| {
        decoder
            .decode_to_string(&encoded)
            .expect("valid fixture should decode")
    });
    println!(
        "allocation decode/{encoding}/{fixture_name}: output_len={}, \
         output_capacity={}, peak_bytes={decode_peak}",
        decoded.len(),
        decoded.capacity(),
    );
}

fn bench_owned_encode<C>(
    criterion: &mut Criterion,
    encoding: &str,
    codec: C,
    fixtures: &[(String, String)],
) where
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
        group.bench_with_input(
            BenchmarkId::new(encoding, fixture_name),
            input,
            |bencher, input| {
                bencher.iter(|| {
                    let output = encoder
                        .encode_str(black_box(input))
                        .expect("valid fixture should encode");
                    black_box((output.len(), output.capacity()));
                });
            },
        );
    }
    group.finish();
}

fn bench_owned_decode<C>(
    criterion: &mut Criterion,
    encoding: &str,
    codec: C,
    fixtures: &[(String, String)],
) where
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
        group.bench_with_input(
            BenchmarkId::new(encoding, fixture_name),
            encoded,
            |bencher, encoded| {
                bencher.iter(|| {
                    let output = decoder
                        .decode_to_string(black_box(encoded))
                        .expect("valid fixture should decode");
                    black_box((output.len(), output.capacity()));
                });
            },
        );
    }
    group.finish();
}

fn bench_streaming_charset(
    criterion: &mut Criterion,
    fixtures: &[(String, String)],
) {
    let mut group = criterion.benchmark_group("streaming_charset_utf8");
    group.sample_size(SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));

    for (fixture_name, input) in fixtures {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("read", fixture_name),
            input,
            |bencher, input| {
                bencher.iter_batched(
                    || Cursor::new(input.as_bytes().to_vec()),
                    |input| {
                        let mut reader = CharsetTextReader::new(
                            input,
                            Utf8Codec,
                            CodingErrorPolicy::Strict,
                        );
                        let mut output = String::new();
                        reader
                            .read_to_string(&mut output)
                            .expect("UTF-8 stream should decode");
                        black_box(output);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
        group.bench_with_input(
            BenchmarkId::new("write", fixture_name),
            input,
            |bencher, input| {
                bencher.iter_batched(
                    || Cursor::new(Vec::with_capacity(input.len())),
                    |output| {
                        let mut writer = CharsetTextWriter::new(
                            output,
                            Utf8Codec,
                            CodingErrorPolicy::Strict,
                        );
                        writer
                            .write_str(input)
                            .expect("UTF-8 stream should encode");
                        writer.finish().expect("UTF-8 stream should finish");
                        let (output, pending) = writer.into_parts();
                        assert!(pending.readable().is_empty());
                        black_box(output.into_inner());
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
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
}

criterion_group!(benches, bench_string_adapters);
criterion_main!(benches);
