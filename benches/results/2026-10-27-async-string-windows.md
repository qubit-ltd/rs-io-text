# Asynchronous string-window benchmark

## Environment

- Baseline completed: 2026-10-27, approximately 06:25 UTC; command exit code 0.
- Host: x86_64 Linux, Ubuntu 24.04 kernel `7.0.0-38-generic`.
- CPU: Intel Core i5-9600K @ 3.70 GHz, 6 cores / 6 threads.
- Compiler: `rustc 1.94.0 (4a4ef493e 2026-03-02)`, LLVM 21.1.8,
  `x86_64-unknown-linux-gnu`.
- Profile: Cargo `bench` (optimized).
- Criterion: 20 samples, 2-second warm-up, 5-second measurement per ID.
- Elapsed: approximately 3 minutes 54 seconds including the initial build.

Criterion's slope estimate and 95% confidence interval are shown below. The
sample output also reported some outliers; confidence intervals are retained
from Criterion's estimates files under `target/criterion`.

## Method

Command: `cargo bench --locked --bench string_adapters -- async_string_windows --noplot`

The same harness measures complete string writes, single-step string loops,
and complete character-slice writes for ASCII, mixed Unicode, and emoji input
at capacities 4, 16, and 8192. Input and character vectors are prepared before
timing. Each measured operation constructs one `AsyncUtf8TextWriter`, writes
the selected input, and calls `finish_async`; a ready counting sink receives
the output. Throughput is normalized to the source UTF-8 byte length. The
character-slice path is a comparison path, not a before/after speedup baseline.

## Pre-change results

| ID | Estimate | 95% CI |
| --- | ---: | ---: |
| ascii_4/full_string | 857.919 µs | 851.945–863.599 µs |
| ascii_4/step_string | 953.450 µs | 869.559–1040.315 µs |
| ascii_4/full_chars | 94.291 µs | 83.242–106.296 µs |
| ascii_16/full_string | 306.224 µs | 275.501–339.740 µs |
| ascii_16/step_string | 229.394 µs | 223.722–242.591 µs |
| ascii_16/full_chars | 20.421 µs | 19.887–21.080 µs |
| ascii_8192/full_string | 21.862 µs | 20.499–22.998 µs |
| ascii_8192/step_string | 19.904 µs | 19.811–20.004 µs |
| ascii_8192/full_chars | 8.089 µs | 7.313–8.767 µs |
| mixed_4/full_string | 4.824 ms | 4.630–5.074 ms |
| mixed_4/step_string | 4.443 ms | 4.369–4.570 ms |
| mixed_4/full_chars | 164.130 µs | 162.071–167.853 µs |
| mixed_16/full_string | 1.156 ms | 1.103–1.206 ms |
| mixed_16/step_string | 1.310 ms | 1.260–1.375 ms |
| mixed_16/full_chars | 51.691 µs | 50.708–53.397 µs |
| mixed_8192/full_string | 44.340 µs | 44.086–44.605 µs |
| mixed_8192/step_string | 44.565 µs | 44.305–44.804 µs |
| mixed_8192/full_chars | 17.010 µs | 16.883–17.141 µs |
| emoji_4/full_string | 9.284 ms | 9.234–9.344 ms |
| emoji_4/step_string | 9.230 ms | 9.183–9.282 ms |
| emoji_4/full_chars | 243.300 µs | 240.574–246.586 µs |
| emoji_16/full_string | 2.319 ms | 2.310–2.326 ms |
| emoji_16/step_string | 2.319 ms | 2.307–2.333 ms |
| emoji_16/full_chars | 74.412 µs | 74.018–74.888 µs |
| emoji_8192/full_string | 58.979 µs | 58.655–59.322 µs |
| emoji_8192/step_string | 60.044 µs | 59.654–60.419 µs |
| emoji_8192/full_chars | 21.419 µs | 21.253–21.594 µs |

## Post-change results

The same command completed with exit code 0 on the same host and Criterion
settings. Each ID used 20 samples, a 2-second warm-up, and a 5-second
measurement. The `full_chars` path does not use either changed string path and
serves as an environment control.

| ID | Estimate | 95% CI |
| --- | ---: | ---: |
| ascii_4/full_string | 69.534 µs | 69.191–69.954 µs |
| ascii_4/step_string | 615.674 µs | 605.128–623.770 µs |
| ascii_4/full_chars | 62.651 µs | 62.261–63.070 µs |
| ascii_16/full_string | 30.840 µs | 30.669–31.048 µs |
| ascii_16/step_string | 190.299 µs | 184.555–196.926 µs |
| ascii_16/full_chars | 24.899 µs | 23.958–26.110 µs |
| ascii_8192/full_string | 20.715 µs | 20.532–20.989 µs |
| ascii_8192/step_string | 27.220 µs | 27.028–27.350 µs |
| ascii_8192/full_chars | 11.813 µs | 11.719–11.906 µs |
| mixed_4/full_string | 179.463 µs | 178.130–180.747 µs |
| mixed_4/step_string | 694.178 µs | 626.297–765.999 µs |
| mixed_4/full_chars | 189.387 µs | 181.369–198.118 µs |
| mixed_16/full_string | 74.389 µs | 71.373–79.482 µs |
| mixed_16/step_string | 261.622 µs | 234.630–290.052 µs |
| mixed_16/full_chars | 51.170 µs | 50.724–51.600 µs |
| mixed_8192/full_string | 34.556 µs | 34.239–34.937 µs |
| mixed_8192/step_string | 41.830 µs | 41.708–41.996 µs |
| mixed_8192/full_chars | 17.010 µs | 16.910–17.110 µs |
| emoji_4/full_string | 266.000 µs | 264.389–267.546 µs |
| emoji_4/step_string | 629.686 µs | 622.248–635.879 µs |
| emoji_4/full_chars | 242.735 µs | 241.099–244.251 µs |
| emoji_16/full_string | 98.631 µs | 98.243–99.117 µs |
| emoji_16/step_string | 179.788 µs | 178.338–181.354 µs |
| emoji_16/full_chars | 75.135 µs | 74.743–75.538 µs |
| emoji_8192/full_string | 45.155 µs | 44.986–45.376 µs |
| emoji_8192/step_string | 50.167 µs | 49.797–50.556 µs |
| emoji_8192/full_chars | 21.444 µs | 21.324–21.612 µs |

Criterion's in-run comparisons reported these >10% statistically significant
regressions: `ascii_16/full_chars` (+20.0%), `ascii_8192/step_string` (+36.1%),
`ascii_8192/full_chars` (+60.3%), and `mixed_4/full_chars` (+20.8%). Three are
unchanged character-slice control paths. The default-capacity ASCII single-step
path is the one changed string path with a reported regression. The baseline
and modified runs were not paired closely enough to distinguish code cost from
system drift, so paired measurements are required before accepting or
attributing these deltas.

## Paired remeasurement

The temporary baseline worktree was detached at `f006f63`; only the benchmark
harness was copied into it. Both commands used the current worktree's
`target` directory serially. Before the baseline run, load average was
1.77/3.06/3.26 and no other high-CPU compiler process was visible. The old
implementation completed all IDs with exit code 0 in approximately 3 minutes
55 seconds using `--save-baseline paired-old`.

| ID | Paired old estimate | 95% CI |
| --- | ---: | ---: |
| ascii_4/full_string | 854.909 µs | 850.034–860.196 µs |
| ascii_4/step_string | 866.162 µs | 860.956–871.662 µs |
| ascii_4/full_chars | 60.916 µs | 60.601–61.284 µs |
| ascii_16/full_string | 218.360 µs | 216.290–220.836 µs |
| ascii_16/step_string | 218.427 µs | 217.573–219.340 µs |
| ascii_16/full_chars | 19.406 µs | 19.231–19.559 µs |
| ascii_8192/full_string | 19.354 µs | 19.182–19.565 µs |
| ascii_8192/step_string | 19.458 µs | 19.327–19.605 µs |
| ascii_8192/full_chars | 5.979 µs | 5.939–6.020 µs |
| mixed_4/full_string | 4.341 ms | 4.308–4.370 ms |
| mixed_4/step_string | 4.301 ms | 4.273–4.327 ms |
| mixed_4/full_chars | 166.116 µs | 163.744–168.523 µs |
| mixed_16/full_string | 1.109 ms | 1.097–1.128 ms |
| mixed_16/step_string | 1.103 ms | 1.083–1.132 ms |
| mixed_16/full_chars | 50.630 µs | 50.227–51.067 µs |
| mixed_8192/full_string | 43.964 µs | 43.703–44.329 µs |
| mixed_8192/step_string | 44.078 µs | 43.834–44.345 µs |
| mixed_8192/full_chars | 17.043 µs | 16.880–17.165 µs |
| emoji_4/full_string | 9.198 ms | 9.137–9.281 ms |
| emoji_4/step_string | 9.511 ms | 9.402–9.619 ms |
| emoji_4/full_chars | 252.875 µs | 248.158–258.917 µs |
| emoji_16/full_string | 2.342 ms | 2.334–2.353 ms |
| emoji_16/step_string | 2.338 ms | 2.315–2.360 ms |
| emoji_16/full_chars | 78.723 µs | 78.544–78.907 µs |
| emoji_8192/full_string | 59.391 µs | 58.991–59.815 µs |
| emoji_8192/step_string | 59.558 µs | 59.278–59.819 µs |
| emoji_8192/full_chars | 21.424 µs | 21.270–21.613 µs |

### Verified optimized build (control drift, not accepted)

The optimized build used the verified independent target and completed all 27
IDs with exit code 0. It ran from approximately 07:05:43 to 07:09:50 UTC with
20 samples, 2-second warm-up, and 5-second measurement per ID. At start, load
average was 2.62/4.31/5.47 and an external `validation_test` process used
96.7% CPU. Other compiler/test processes appeared during the run. End load was
3.69/3.79/4.95. Estimates are recorded below, but the run is not accepted for
performance sign-off because ASCII character-slice controls drifted materially.

| ID | Estimate | 95% CI |
| --- | ---: | ---: |
| ascii_4/full_string | 70.407 µs | 70.004–70.743 µs |
| ascii_4/step_string | 600.126 µs | 594.829–607.290 µs |
| ascii_4/full_chars | 61.561 µs | 61.205–61.900 µs |
| ascii_16/full_string | 32.843 µs | 31.417–34.621 µs |
| ascii_16/step_string | 167.287 µs | 162.783–172.367 µs |
| ascii_16/full_chars | 23.095 µs | 22.975–23.224 µs |
| ascii_8192/full_string | 20.450 µs | 20.295–20.633 µs |
| ascii_8192/step_string | 27.097 µs | 27.008–27.182 µs |
| ascii_8192/full_chars | 11.779 µs | 11.730–11.826 µs |
| mixed_4/full_string | 182.122 µs | 181.567–182.644 µs |
| mixed_4/step_string | 669.403 µs | 640.427–707.231 µs |
| mixed_4/full_chars | 165.354 µs | 164.178–166.350 µs |
| mixed_16/full_string | 68.429 µs | 68.099–68.742 µs |
| mixed_16/step_string | 221.441 µs | 220.347–222.529 µs |
| mixed_16/full_chars | 52.044 µs | 51.296–52.588 µs |
| mixed_8192/full_string | 34.788 µs | 34.607–34.931 µs |
| mixed_8192/step_string | 42.150 µs | 41.971–42.337 µs |
| mixed_8192/full_chars | 17.132 µs | 17.051–17.219 µs |
| emoji_4/full_string | 267.625 µs | 266.341–269.026 µs |
| emoji_4/step_string | 619.618 µs | 616.981–622.441 µs |
| emoji_4/full_chars | 244.430 µs | 243.271–245.531 µs |
| emoji_16/full_string | 118.445 µs | 101.975–130.665 µs |
| emoji_16/step_string | 181.884 µs | 181.446–182.274 µs |
| emoji_16/full_chars | 76.587 µs | 75.737–77.403 µs |
| emoji_8192/full_string | 45.703 µs | 45.583–45.853 µs |
| emoji_8192/step_string | 50.210 µs | 49.907–50.503 µs |
| emoji_8192/full_chars | 21.631 µs | 21.542–21.711 µs |

Against paired old, complete strings improved 91.8%/95.8%/97.1% at capacity 4
for ASCII/mixed/emoji, and 85.0%/93.8%/94.9% at capacity 16. At capacity 8192,
mixed and emoji complete strings improved 20.9% and 23.0%; ASCII changed by
5.7%. Single-step paths improved substantially at capacities 4 and 16 for all
three fixtures. At capacity 8192, mixed and emoji improved 4.4% and 15.7%, while
ASCII was 39.3% slower. The unaffected `full_chars` controls were close to
paired old for most fixtures, but `ascii_16/full_chars` drifted +19.0% and
`ascii_8192/full_chars` drifted +97.0%. This makes the default-capacity ASCII
single-step comparison inconclusive; report it as a measured regression pending
a cleaner repeat rather than attributing the entire delta to the code change.

### Focused default-capacity diagnostic (performance not accepted)

To isolate the default-capacity ASCII single-step result, both old and optimized
binaries ran the same filter, covering `step_string` and `full_chars` for ASCII,
mixed Unicode, and emoji at capacity 8192. Each side used 20 samples, 2-second
warm-up, and 5-second measurement. The old side ran first around 07:25:34 UTC
in a separate target (start load 7.82/6.43/5.70; no external process above
10% CPU at the start; estimates finished by 07:26:28 UTC). The optimized side
then ran in its verified independent target around 07:27:45–07:28:38 UTC
(start load 6.42/6.47/5.80; no external high-CPU compiler/test process at the
start or mid-run). The old controls were within 10% of the prior paired-old
baseline. The optimized controls did not stay within 10%, and the emoji control
had a wide interval, so these measurements do not meet performance acceptance.

| ID | Old estimate (95% CI) | Optimized estimate (95% CI) | Step / control ratio, old → optimized |
| --- | ---: | ---: | ---: |
| ascii_8192/step_string | 21.462 µs (20.574–22.389) | 28.381 µs (27.821–29.014) | 3.32 → 2.40 |
| ascii_8192/full_chars | 6.458 µs (6.404–6.520) | 11.825 µs (11.746–11.882) | — |
| mixed_8192/step_string | 44.581 µs (44.362–44.863) | 42.194 µs (42.007–42.427) | 2.63 → 2.49 |
| mixed_8192/full_chars | 16.948 µs (16.804–17.077) | 16.977 µs (16.839–17.114) | — |
| emoji_8192/step_string | 59.439 µs (59.161–59.722) | 50.114 µs (49.794–50.500) | 2.79 → 1.19 |
| emoji_8192/full_chars | 21.351 µs (21.254–21.483) | 41.966 µs (33.489–53.618) | — |

The raw ASCII step time rose 32.2%, while its `full_chars` control rose 83.1%;
the normalized step/control ratio fell about 27.7%. Mixed Unicode's ratio fell
about 5.5%. The emoji ratio is inconclusive because the optimized control
interval is wide. Report the default-capacity ASCII step result as unresolved
and the performance gate as not accepted; do not treat the ratio alone as a
passing benchmark.

The first paired after-run using the shared target was invalid: Cargo printed
`Finished` without `Compiling qubit-io-text`, and its small-capacity string
timings matched the old implementation. That run was interrupted (exit code
130) and is excluded. `cargo clean --manifest-path Cargo.toml -p qubit-io-text`
exited 0 and preserved all 27 `paired-old` estimates, but did not force the
expected compile. The source mtime was touched without changing contents; the
subsequent output still did not show a compile.

A fresh target was then prepared at
`/tmp/rs-io-text-t1-corrected-target-20261027`. The no-run command
`CARGO_TARGET_DIR=/tmp/rs-io-text-t1-corrected-target-20261027 cargo bench
--locked --bench string_adapters --no-run` exited 0 and explicitly compiled
`qubit-io-text` from the optimized worktree. Two sampling attempts have been
excluded in full:

- The first started around 06:53:30 UTC with no external process above 50% CPU;
  it was interrupted around 06:54:12 UTC after an unrelated `llvm-cov` process
  reached 140% CPU (exit code 130).
- The second started around 06:54:57 UTC after a fresh process check showed no
  external process above 10% CPU. It was interrupted after external `rustc`
  processes reached 100% CPU (exit code 130).

Each attempt completed six IDs before interruption; none of those diagnostic
measurements are used as results. No competing processes were stopped. A
third attempt used the verified independent optimized target. It started around
06:56:20 UTC after the coordinating agent's workspace test process exited; one
external rustc process was already active. Additional compilers and tests then
started during sampling. The control path showed clear drift (for example,
`ascii_16/full_chars` was 24.169 µs vs 19.406 µs paired old, and
`ascii_8192/full_chars` was 13.470 µs vs 5.979 µs paired old). It was
interrupted around 06:57 UTC with exit code 130 and is excluded in full. No
competing processes were stopped. A complete optimized run remains pending a
period with stable paired control measurements.
