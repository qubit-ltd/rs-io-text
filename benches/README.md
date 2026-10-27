# String adapter benchmarks

Run the owned string-adapter benchmark with:

```shell
cargo bench --bench string_adapters -- --noplot
```

Run the focused UTF-8 writer measurements with:

```shell
cargo bench --locked --bench string_adapters -- 'utf8_line|writer_allocation|streaming_charset_utf8' --noplot
```

Run the asynchronous string-window measurements before and after changing the
writer implementation with:

```shell
cargo bench --locked --bench string_adapters -- async_string_windows --noplot
```

The `async_string_windows` group measures complete string writes, repeated
single-step string writes, and complete character-slice writes for ASCII,
mixed Unicode, and emoji fixtures at capacities 4, 16, and 8192. Fixtures and
their character vectors are prepared outside timing. Each iteration includes
the same writer construction and `finish_async` boundary and uses an
immediately ready byte sink. Throughput is normalized to the source UTF-8 byte
length; each path is checked in a dry run for exact output length and empty
pending bytes. See the [2026-10-27 result record](results/2026-10-27-async-string-windows.md)
for paired measurements and environment details.

The `utf8_line` group compares `write_line` with two explicit `write_str`
calls for short and long ASCII, multilingual, and supplementary-plane text
under LF, CRLF, and CR endings. Input construction and writer construction
(including a preallocated output `Vec`) happen outside the timed routine. Both
paths use the same output capacity and 8 KiB internal byte buffer; both finish
the writer inside the timed routine. Throughput counts body bytes plus line
ending bytes. Before timing, each pair is checked against the expected output.

The `writer_allocation` group measures construction and finish, UTF-8 string,
line, and character writes, and generic UTF-16LE first and repeated string
writes, all using a sink and a fixed 16-byte internal buffer. Its single-thread
dry run reports successful allocation requests and peak live bytes. The helper
counts successful `alloc`, `alloc_zeroed`, and `realloc` requests only while
measurement is enabled; input and output `Vec` allocations are outside these
operations. Two empty dry runs check for residual counts. All new groups use
the existing 20 samples, 2-second warm-up, and 5-second measurement settings.
See [the 2026-10-26 baseline record](results/2026-10-26-utf8-writer-optimizations.md)
for measured results and environment details.

The timed measurements include all allocations performed by
`CharsetStringEncoder::encode_str` and
`CharsetStringDecoder::decode_to_string`. Before Criterion starts timing, the
benchmark also prints one single-threaded dry-run measurement for each fixture
and encoding. Each line reports the returned buffer's length and capacity plus
the peak number of live bytes allocated during that operation.

Throughput is normalized to the UTF-8 byte length of the logical source text so
that results for UTF-8, UTF-16LE, and UTF-32LE remain directly comparable.
