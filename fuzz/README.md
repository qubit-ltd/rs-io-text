# Fuzz targets

`charset_stream` exercises the synchronous charset stream paths. `async_charset_state` drives the asynchronous charset reader and writer with four header-selected scenarios: writer progress and finish, character batches, line reads, and limited-line reads with rejection recovery.

The asynchronous target caps each input at 4096 bytes. Its six-byte header keeps the existing layout: byte 0 selects the legacy scenario in its low two bits and independently selects the UTF-8 raw companion mode in bits 2–3 (`% 3`); byte 1 selects capacity (1–8); byte 2 selects the line-ending set and independently selects the multibyte companion decode policy in bits 2–3 (`% 3`); bytes 3 and 4 select operation and event counts; byte 5 selects the poll/cancellation parameter. The ending index is `byte2 % 4 + 4 * ((byte2 / 12) % 2)`:

| Index | Accepted line endings |
| --- | --- |
| 0 | LF, CRLF, CR |
| 1 | LF |
| 2 | CRLF |
| 3 | CR |
| 4 | none |
| 5 | LF, CRLF |
| 6 | LF, CR |
| 7 | CRLF, CR |

For deterministic inputs, choose an ending index `e` (0–7) and multibyte policy `p` (0–2) with `byte2 = 12 * (e / 4) + 4 * p + e % 4`. This preserves the policy expression `(byte2 >> 2) % 3`, including its existing high-bit behavior. The first byte, capacity, operation/event counts, six-byte header boundary, operation and event segments, and payload boundary are unchanged. Original byte-2 values 0–11 retain their previous ending interpretation. Some older corpus inputs with higher byte-2 values now select one of the additional sets; corpus files are not rewritten.

The operation and event count bytes are each reduced modulo 65, selecting 0–64 bytes; the target then reads up to that many operation bytes, up to that many scheduled I/O events, and uses the remainder as payload. The four original scenarios retain their meaning: writer progress/finish, character batches, normal lines, and limited lines. Those scenarios still convert payload bytes to valid source text with lossy UTF-8 conversion.

After the selected legacy scenario, the target runs an independent UTF-8 raw-byte reader companion on the original payload. It never uses the lossy text as this reader's input. The low two bits of byte 0 retain the legacy scene, and `scene % 3` selects Report, Replace (default U+FFFD), or Ignore; bits 2–3 of byte 0 independently select the raw companion read mode (`% 3`): character/batch reads, normal-line reads, or limited-line reads. Report always reads one character at a time so decode errors cannot be confused with limited-line errors. The selected byte-2 ending index configures the line-ending set. The companion receives a copied event schedule and its own input position. A standard-library `from_utf8` segmented reference model computes expected output and the first malformed-byte offset; valid U+FFFD characters remain ordinary text. Report stops on the first non-injected decode error and never retries it. Replace/Ignore check complete output at natural EOF, including records formed when ignored bytes join CRLF. Limited reads use decoded UTF-8 byte lengths and retain the existing per-call limit and rejection recovery checks.

The target then runs one multibyte raw-byte companion on that same original payload, independently of both the lossy UTF-8 source text and the UTF-8 companion's reader state. `scene % 4` selects UTF-16LE, UTF-16BE, UTF-32LE, or UTF-32BE; bits 2–3 of byte 2 independently select Report, Replace (default U+FFFD), or Ignore; and the first operation byte modulo 3 selects character/batch reads, normal-line reads, or limited-line reads. Report is always constrained to character reads. The selected encoding, policy, and read mode are combined independently, and the companion receives its own copied event schedule and input position. Its reference output comes from the separate reference decoder over the original payload bytes, not from the production reader or a re-encoded/lossy string. Report checks the expected text prefix and must observe the malformed input before natural EOF; Replace and Ignore require complete expected text at natural EOF. Valid U+FFFD characters remain ordinary text. Line reads use the selected byte-2 ending index, decoded UTF-8 byte-length limits, and rejection recovery checks.

In each scenario, record reads use the selected line operation. In limited-line scene 3, each record uses its corresponding operation byte modulo 33 as the per-call append limit (0–32 bytes); missing operation bytes mean zero. Retries of a record keep the same limit, while a retained prefix from an earlier call does not reduce the next call's allowance. The scene checks target prefix preservation, rollback on rejection, and natural EOF completion. Writer operations choose a single string step, character-slice step, explicit flush after a step, or entry into finish. Readers receive the full event schedule; writer events at even original positions feed write and odd positions feed flush, so events are never cloned. A nonzero cancellation setting drops a Future at a Pending poll and retries that operation on the same adapter. Pending and injected I/O errors do not transfer data. Errors are retried only when their source is the model's `InjectedFault`.

Each reader scenario (legacy, UTF-8 raw companion, and multibyte raw-byte companion) is bounded to 64 logical operations, 64 I/O events, and 65,536 polls; the whole input remains capped at 4096 bytes. Event schedules become ready after their finite events are consumed. The model checks accepted/delivered writer prefixes and reader prefixes when the operation budget ends; when a reader observes natural EOF, it requires the complete expected text, records, or rejected-record count. Deterministic unit tests enumerate all 256 values of byte 2, verify all 8 × 3 ending/policy combinations, check hand-written reference records for each set, and exercise normal and limited async reads, cancellation, injected errors, and rejection recovery. These tests establish those finite selector and scenario cases. A bounded fuzz smoke checks that the target runs without a crash for the selected duration and seed; it does not establish exhaustive coverage. Existing corpus byte boundaries and legacy scene selection remain unchanged. Nonzero high bits in byte 2 may now select different ending sets, while the policy expression and payload layout stay stable.

Run a deterministic smoke from the repository root:

```sh
cargo +nightly-2026-06-05 fuzz run async_charset_state -- -max_total_time=10 -max_len=4096 -seed=1
```

When fuzzing finds a failure, preserve the input and seed, minimize the input, then convert the reduced case into a focused regression test.
