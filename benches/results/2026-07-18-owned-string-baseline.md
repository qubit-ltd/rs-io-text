# Owned string-adapter baseline — 2026-07-18

Environment: Linux 6.17, Intel Core i5-9600K (6 CPUs), Rust 1.94.0.
Criterion used 20 samples, a 2-second warm-up, and a 5-second
measurement window. Timings include the owned output allocation.

## Median times

| Direction | Encoding | ASCII | Multilingual | Supplementary |
| --- | --- | ---: | ---: | ---: |
| encode | UTF-8 | 118.456 µs | 264.641 µs | 150.417 µs |
| encode | UTF-16LE | 250.840 µs | 353.449 µs | 275.715 µs |
| encode | UTF-32LE | 138.134 µs | 212.657 µs | 157.074 µs |
| decode | UTF-8 | 257.813 µs | 488.600 µs | 364.174 µs |
| decode | UTF-16LE | 291.009 µs | 426.886 µs | 369.415 µs |
| decode | UTF-32LE | 303.988 µs | 439.280 µs | 364.328 µs |

## Peak live allocation

The benchmark's single-threaded dry run measures allocations made during one
adapter call. Values are bytes.

| Direction | Encoding | ASCII | Multilingual | Supplementary |
| --- | --- | ---: | ---: | ---: |
| encode | UTF-8 | 458,752 | 573,440 | 524,288 |
| encode | UTF-16LE | 458,752 | 573,440 | 524,288 |
| encode | UTF-32LE | 458,752 | 573,440 | 524,288 |
| decode | UTF-8 | 286,720 | 675,840 | 450,560 |
| decode | UTF-16LE | 516,096 | 708,608 | 679,936 |
| decode | UTF-32LE | 974,848 | 1,282,048 | 1,138,688 |

The encoding peak is dominated by simultaneous ownership of the collected
`Vec<char>` and the conservatively sized output. UTF-8 output capacity is
229,376 bytes for a 57,344-byte ASCII result, and 286,720 bytes for a
135,168-byte multilingual result. The decode path similarly holds an
intermediate `Vec<char>` while constructing the final `String`; UTF-32 input
therefore reaches the largest peak.

Command:

```shell
cargo bench --bench string_adapters -- --noplot
```
