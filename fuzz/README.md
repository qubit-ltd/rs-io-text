# Fuzz targets

`charset_stream` exercises the synchronous charset stream paths. `async_charset_state` drives the asynchronous charset reader and writer with four header-selected scenarios: writer progress and finish, character batches, line reads, and rejected-line recovery.

The asynchronous target caps each input at 4096 bytes. Six header bytes select the scenario, capacity (1–8), line-ending set, operation count, event count, and poll/cancellation parameter. The operation and event count bytes are each reduced modulo 65, selecting 0–64 bytes; the target then reads up to that many operation bytes, up to that many scheduled I/O events, and uses the remainder as payload. Payload bytes become valid source text through lossy UTF-8 conversion. Reader operations choose one-character or three-character-batch reads; normal-line and rejected-line scenarios keep using their selected line operation. Writer operations choose a single string step, character-slice step, explicit flush after a step, or entry into finish. Readers receive the full event schedule; writer events at even original positions feed write and odd positions feed flush, so events are never cloned. A nonzero cancellation setting drops a Future at a Pending poll and retries that operation on the same adapter. Pending and injected I/O errors do not transfer data. Errors are retried only when their source is the model's `InjectedFault`.

Each input is bounded to 64 logical operations, 64 I/O events, and 65,536 polls. Event schedules become ready after their finite events are consumed. The model checks accepted/delivered writer prefixes, reader text prefixes, line-record prefixes, and rejected-line recovery against independent text expectations. The unit tests assert that targeted scheduler branches are reached; a fuzz smoke run alone is not a coverage claim.

Run a deterministic smoke from the repository root:

```sh
cargo +nightly-2026-06-05 fuzz run async_charset_state -- -max_total_time=10 -max_len=4096 -seed=1
```

When fuzzing finds a failure, preserve the input and seed, minimize the input, then convert the reduced case into a focused regression test.
