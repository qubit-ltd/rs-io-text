# UTF-8 writer optimization baseline (before IOT-001/IOT-002)

Date: 2026-10-26

This is the **unoptimized** baseline for IOT-001 (UTF-8 line fast-path reuse)
and IOT-002 (lazy character staging allocation). No production source was
changed for this measurement.

## Baseline and environment

- Repository: `qubit-io-text`, version `0.9.0`.
- Branch: `codex/iot-sync-writer-20261026`.
- Baseline revision: `8e395b477cbdf2d5239800f00eef1b6baa3421b7`.
- Worktree state before T1 edits: clean (`git status --short --branch` showed
  only the branch line). The two planning documents are kept in the original
  repository and are not part of this worktree.
- Toolchain: `rustc 1.94.0 (4a4ef493e 2026-03-02)`, host
  `x86_64-unknown-linux-gnu`, LLVM `21.1.8`.
- OS: Ubuntu 24.04.4 LTS, Linux `7.0.0-38-generic`, x86_64.
- CPU: Intel Core i5-9600K @ 3.70 GHz, 6 cores/6 threads; reported frequency
  range 800–4600 MHz.
- Locked direct dependency resolution (`cargo tree --locked --depth 1`):
  `qubit-codec 0.17.0`, `qubit-codec-text 0.9.0`, `qubit-io 0.19.0`,
  `qubit-utils 0.3.1`, and dev dependency `criterion 0.8.2`.
- `Cargo.lock` SHA-256:
  `dc1c6e4dea62629c830b64110d6cd5ae5e2a91dc33ac5094ee4235f2d3c6142a`.
- Criterion settings for the new and existing comparison groups: 20 samples,
  2-second warm-up, 5-second measurement. Release bench profile; locked
  dependencies.

## Commands and measurement boundaries

Compilation succeeded with:

```shell
cargo bench --locked --bench string_adapters --no-run
```

The baseline run used:

```shell
cargo bench --locked --bench string_adapters -- 'utf8_line|writer_allocation|streaming_charset_utf8' --noplot
```

`utf8_line` uses short ASCII (9 body bytes), long ASCII (6,400 body bytes),
multilingual (1,040 body bytes), and supplementary-plane (688 body bytes) input.
For each fixture it compares one `write_line` with two explicit `write_str`
calls across LF, CRLF, and CR. Input and writer setup are outside Criterion's
timed routine. Both variants start with an output `Vec` preallocated to the
same expected output length and use the same 8 KiB writer byte-buffer
capacity. `finish()` and output consumption are timed. Throughput counts body
plus line-ending bytes. Before timing, both paths were checked against the
expected bytes and against each other for all fixtures and endings.

`writer_allocation` uses a `std::io::sink()` output and 16-byte internal buffer
capacity. Its dry run measures construction plus `finish`, UTF-8 `write_str`,
UTF-8 `write_line`, direct UTF-8 character write, generic UTF-16LE first and
repeated string writes, and generic UTF-16LE direct character write. Inputs are
static literals; there is no input or output `Vec` in the measured operation.
The allocator counter tracks successful `alloc`, `alloc_zeroed`, and `realloc`
requests only while enabled. Two consecutive empty measurements returned
zero peak bytes and zero requests.

The existing `streaming_charset_utf8` results are included as contextual
comparators. The complete terminal output was captured during the run at
`/tmp/rs-io-text-iot-t1-bench.log` in the execution environment; tables below
preserve Criterion's reported interval summaries.

## Dry-run allocation results

| Operation | Successful allocation requests | Peak live bytes |
| --- | ---: | ---: |
| construct + finish | 2 | 1,040 |
| UTF-8 `write_str` | 2 | 1,040 |
| UTF-8 `write_line` | 3 | 1,056 |
| UTF-8 direct character | 2 | 1,040 |
| UTF-16LE first `write_str` | 3 | 1,056 |
| UTF-16LE repeated `write_str` | 3 | 1,056 |
| UTF-16LE direct character | 2 | 1,040 |

The two empty operation checks both passed with `(peak_bytes, requests) =
(0, 0)`. These are baseline allocation counts; they do not imply that the
underlying allocations are avoidable, and the allocator instrumentation is
single-threaded by design.

## Criterion summaries

Intervals and throughput are copied from Criterion output as
`[low, estimate, high]`; they are not claims of guaranteed performance.

### `utf8_line`

| Fixture / ending | Path | Time [low, estimate, high] | Throughput [low, estimate, high] |
| --- | --- | --- | --- |
| short ASCII / LF | `write_line` | 2.0017–2.6299 µs (2.2830) | 3.6263–4.7643 MiB/s (4.1773) |
| short ASCII / LF | two `write_str` | 2.2110–2.8483 µs (2.5061) | 3.3482–4.3133 MiB/s (3.8054) |
| short ASCII / CRLF | `write_line` | 2.3931–3.2475 µs (2.8265) | 3.2303–4.3837 MiB/s (3.7115) |
| short ASCII / CRLF | two `write_str` | 1.7355–2.4382 µs (2.0261) | 4.3026–6.0447 MiB/s (5.1777) |
| short ASCII / CR | `write_line` | 1.3507–1.7238 µs (1.4957) | 5.5324–7.0608 MiB/s (6.3762) |
| short ASCII / CR | two `write_str` | 1.4754–1.9024 µs (1.6420) | 5.0130–6.4637 MiB/s (5.8080) |
| long ASCII / LF | `write_line` | 40.889–49.178 µs (45.121) | 124.13–149.29 MiB/s (135.29) |
| long ASCII / LF | two `write_str` | 7.0538–9.7392 µs (8.3962) | 626.79–865.42 MiB/s (727.06) |
| long ASCII / CRLF | `write_line` | 58.329–94.504 µs (74.424) | 64.605–104.67 MiB/s (82.036) |
| long ASCII / CRLF | two `write_str` | 6.9923–10.827 µs (8.6802) | 563.91–873.16 MiB/s (703.38) |
| long ASCII / CR | `write_line` | 44.592–62.953 µs (52.471) | 96.969–136.90 MiB/s (116.34) |
| long ASCII / CR | two `write_str` | 5.2091–6.5966 µs (5.7923) | 925.39 MiB/s–1.1444 GiB/s (1.0292 GiB/s) |
| multilingual / LF | `write_line` | 4.2444–5.4741 µs (4.7679) | 181.36–233.90 MiB/s (208.22) |
| multilingual / LF | two `write_str` | 1.1107–1.4699 µs (1.2664) | 675.39–893.84 MiB/s (783.95) |
| multilingual / CRLF | `write_line` | 3.7182–4.3103 µs (3.9797) | 230.55–267.26 MiB/s (249.70) |
| multilingual / CRLF | two `write_str` | 1.2862–1.5778 µs (1.4181) | 629.82–772.60 MiB/s (700.76) |
| multilingual / CR | `write_line` | 4.5634–5.9271 µs (5.3135) | 167.50–217.55 MiB/s (186.84) |
| multilingual / CR | two `write_str` | 1.8567–2.6050 µs (2.2843) | 381.11–534.70 MiB/s (434.62) |
| supplementary / LF | `write_line` | 4.5201–7.2228 µs (6.0247) | 90.974–145.37 MiB/s (109.06) |
| supplementary / LF | two `write_str` | 1.3680–1.9434 µs (1.6501) | 338.11–480.32 MiB/s (398.21) |
| supplementary / CRLF | `write_line` | 3.1752–4.6870 µs (3.9740) | 140.40–207.24 MiB/s (165.58) |
| supplementary / CRLF | two `write_str` | 1.2320–1.5230 µs (1.3634) | 432.07–534.13 MiB/s (482.64) |
| supplementary / CR | `write_line` | 3.1604–4.0439 µs (3.5371) | 162.49–207.91 MiB/s (185.77) |
| supplementary / CR | two `write_str` | 1.2918–1.6375 µs (1.4414) | 401.28–508.64 MiB/s (455.86) |

### Existing `streaming_charset_utf8` contextual results

| Fixture | Path | Time [low, estimate, high] | Throughput [low, estimate, high] |
| --- | --- | --- | --- |
| ASCII | read | 1.3089–1.8005 ms (1.4996) | 30.373–41.781 MiB/s (36.469) |
| ASCII | charset write | 626.50–912.94 µs (783.63) | 59.902–87.290 MiB/s (69.788) |
| ASCII | fast UTF-8 write | 40.475–71.311 µs (53.815) | 766.89 MiB/s–1.3195 GiB/s (1016.2 MiB/s) |
| multilingual | read | 968.89 µs–1.6911 ms (1.3449 ms) | 76.224–133.04 MiB/s (95.851) |
| multilingual | charset write | 988.88 µs–1.2982 ms (1.1710 ms) | 99.297–130.36 MiB/s (110.08) |
| multilingual | fast UTF-8 write | 132.02–208.76 µs (163.85) | 617.47–976.43 MiB/s (786.76) |
| supplementary | read | 1.5711–2.6919 ms (2.1279 ms) | 31.925–54.699 MiB/s (40.385) |
| supplementary | charset write | 947.40 µs–1.3881 ms (1.1446 ms) | 61.911–90.709 MiB/s (75.082) |
| supplementary | fast UTF-8 write | 65.600–93.931 µs (77.829) | 914.90 MiB/s–1.2793 GiB/s (1.0783 GiB/s) |

### `writer_allocation` timing

| Operation | Time [low, estimate, high] |
| --- | --- |
| construct + finish | 87.943–110.33 ns (98.574) |
| UTF-8 `write_str` | 62.018–78.831 ns (68.117) |
| UTF-8 `write_line` | 239.74–297.88 ns (267.53) |
| UTF-8 direct character | 72.135–81.310 ns (75.946) |
| UTF-16LE first `write_str` | 284.62–434.06 ns (345.27) |
| UTF-16LE repeated `write_str` | 453.41–613.42 ns (534.25) |
| UTF-16LE direct character | 146.64–209.10 ns (181.31) |

## Baseline interpretation and limitations

The output checks passed for every line fixture and ending. The current
pre-optimization `write_line` path is substantially slower than two explicit
UTF-8 `write_str` calls for the longer fixtures, which gives T2 a direct
comparison point. The short fixture intervals overlap in some cases. The
allocation dry run reports an extra successful allocation request and 16 more
peak bytes for `write_line` than for a UTF-8 string or direct character write;
this is a measurement of the current behavior, not proof of the precise source
of each allocation.

Several intervals are wide, and Criterion reported outliers in multiple
samples. CPU frequency scaling is enabled and the host was not isolated from
other workloads. Results therefore establish the measured baseline and
benchmark entry point; they do not justify a claim of a stable speedup by
themselves. T3 must repeat the exact command and compare the same summaries.
