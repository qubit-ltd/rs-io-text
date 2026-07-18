# String adapter benchmarks

Run the owned string-adapter benchmark with:

```shell
cargo bench --bench string_adapters -- --noplot
```

The timed measurements include all allocations performed by
`CharsetStringEncoder::encode_str` and
`CharsetStringDecoder::decode_to_string`. Before Criterion starts timing, the
benchmark also prints one single-threaded dry-run measurement for each fixture
and encoding. Each line reports the returned buffer's length and capacity plus
the peak number of live bytes allocated during that operation.

Throughput is normalized to the UTF-8 byte length of the logical source text so
that results for UTF-8, UTF-16LE, and UTF-32LE remain directly comparable.
