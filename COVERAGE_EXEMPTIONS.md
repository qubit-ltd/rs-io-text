# Coverage exemptions

The crate keeps only the following source-level exemptions in
`.rs-ci-coverage.json`:

- `src/adapters/async_charset_text_reader.rs`: the async reader's cancellation
  and poll-state branches are driven by executor scheduling and several
  generated future-state paths cannot be deterministically exercised by the
  synchronous integration-test harness. Its observable decode, buffering,
  cancellation, line-ending, and error behavior is covered by the async
  adapter tests.
- `src/adapters/async_charset_text_writer.rs`: output backpressure and
  cancellation create generated poll-state paths that are not stable under
  deterministic coverage runs. Tests cover encoding, pending output,
  retries, cancellation, flush, finish, and error behavior; the exemption is
  limited to those compiler-generated scheduling paths.
- `src/adapters/async_utf8_text_reader.rs`: this forwarding adapter mostly
  consists of async trait delegation and generated future polling glue. Its
  behavior is exercised through the async UTF-8 forwarding tests, while the
  underlying buffered reader has full non-async coverage.

The async text trait files were removed from the exemption list after their
progress-contract tests raised them above the configured thresholds.
