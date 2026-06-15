# cargo-fuzz Setup for Framed Message Protocol

This directory contains coverage-guided fuzzing targets for the core `Frame` codec.

## Prerequisites

```bash
rustup install nightly
cargo install cargo-fuzz
```

## Running the Fuzzers

```bash
# From the rust/ directory
cd rust

# Fuzz raw decoding (most important for security)
cargo +nightly fuzz run decode_raw

# Fuzz roundtrip / serialization stability
cargo +nightly fuzz run roundtrip

# With more cores and longer timeout
cargo +nightly fuzz run decode_raw -- -jobs=8 -max_len=65536 -timeout=10
```

## Recommended Flags

- `-max_len=1024` or `4096` — focus on header + small payloads first
- `-rss_limit_mb=512` — prevent OOM from the fuzzer itself
- `-timeout=5` — catch infinite loops early
- `-dict=...` — you can add a dictionary of interesting byte sequences later

## Corpus Management

`cargo-fuzz` automatically maintains a corpus in `fuzz/artifacts/` and `fuzz/corpus/`.

Good practice:
- Commit interesting minimal crashing inputs (they live in `artifacts/crash-*`)
- Periodically minimize the corpus: `cargo +nightly fuzz cmin decode_raw`

## Integration with Existing Tests

This `cargo-fuzz` setup complements the `proptest` tests in `tests/proptest_frame.rs`:
- `proptest` = fast, deterministic, runs on stable in normal `cargo test`
- `cargo-fuzz` = deep, coverage-guided, finds complex bugs over long runs (use nightly)

Run both:
```bash
cargo test --test proptest_frame
cargo +nightly fuzz run decode_raw -- -max_total_time=300
```

## Adding New Targets

1. Create `fuzz_targets/your_target.rs`
2. Use the `fuzz_target!` macro
3. Run with `cargo +nightly fuzz run your_target`

## Security Focus Areas

- Header parsing edge cases
- Huge `length` values (DoS)
- Truncated / overlong frames
- Checksum validation bypasses
- Unknown `version` / `flags` / `type` combinations

See `../docs/fuzzing-strategy.md` for the full strategic context.