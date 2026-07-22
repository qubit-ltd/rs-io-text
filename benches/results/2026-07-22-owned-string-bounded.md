# Owned string-adapter bounded-buffer result — 2026-07-22

Environment: Linux 6.17, Intel Core i5-9600K (6 CPUs), Rust 1.94.0.
Criterion used 20 samples, a 2-second warm-up, and a 5-second measurement
window. Timings include the owned output allocation.

## Median times

| Direction | Encoding | ASCII | Multilingual | Supplementary |
| --- | --- | ---: | ---: | ---: |
| encode | UTF-8 | 83.902 µs | 255.123 µs | 125.323 µs |
| encode | UTF-16LE | 188.716 µs | 267.405 µs | 225.699 µs |
| encode | UTF-32LE | 111.218 µs | 194.182 µs | 140.066 µs |
| decode | UTF-8 | 289.512 µs | 503.994 µs | 388.599 µs |
| decode | UTF-16LE | 292.356 µs | 426.095 µs | 362.327 µs |
| decode | UTF-32LE | 291.999 µs | 425.486 µs | 347.138 µs |

## Peak live allocation

The benchmark's single-threaded dry run measures allocations made during one
adapter call. Values are bytes.

| Direction | Encoding | ASCII | Multilingual | Supplementary |
| --- | --- | ---: | ---: | ---: |
| encode | UTF-8 | 65,536 | 262,144 | 131,072 |
| encode | UTF-16LE | 131,072 | 262,144 | 262,144 |
| encode | UTF-32LE | 262,144 | 524,288 | 262,144 |
| decode | UTF-8 | 66,560 | 246,784 | 91,136 |
| decode | UTF-16LE | 66,560 | 246,784 | 91,136 |
| decode | UTF-32LE | 66,560 | 246,784 | 91,136 |

Compared with the 2026-07-18 baseline, every measured peak is lower. Encoding
peak reductions range from 8.6% to 85.7%; decoding peak reductions range from
63.5% to 93.2%. The remaining encoding allocation is the owned encoded result.
The decode path retains the owned UTF-8 result plus a fixed 1,024-byte
character window.

The bounded conversion lowers every encoding median as well as peak memory.
Most decode medians also improve; the UTF-8 fixtures and UTF-16LE ASCII regress,
with the largest observed median regression in the UTF-8 ASCII decode case.
These numbers are intended as a reproducible trade-off record rather than a
performance threshold.

Command:

```shell
cargo bench --bench string_adapters -- --noplot
```
