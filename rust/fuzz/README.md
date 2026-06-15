# cargo-fuzz Setup for Framed Message Protocol

This directory contains coverage-guided fuzzing targets for the core `Frame` codec.

## GitHub Actions Integration

There are two workflows:

- **`.github/workflows/ci.yml`** — Runs on every push/PR:
  - Formatting + Clippy
  - All tests (including `proptest` layer)
  - Example builds
  - Quick check that fuzz targets still compile on nightly

- **`.github/workflows/fuzz.yml`** — Scheduled + manual:
  - Every Sunday at 03:00 UTC the `decode_raw` target runs for ~45 minutes
  - `roundtrip` target runs afterwards
  - Crashing inputs are automatically uploaded as GitHub Artifacts
  - Can also be triggered manually via "Run workflow" button

This gives you continuous light fuzzing without blocking normal development.

## Local Usage (recommended)

```bash
./fuzz.sh decode_raw          # Main security target
./fuzz.sh decode_raw 30     # Run for 30 minutes
./fuzz.sh roundtrip
./fuzz.sh both 20           # Both targets sequentially
```

See the script `../fuzz.sh` for a convenient colored wrapper.

## Recommended local flags

```bash
cargo +nightly fuzz run decode_raw -- \
  -jobs=8 \
  -max_len=4096 \
  -timeout=5 \
  -rss_limit_mb=1024
```

## Corpus & Artifacts

- Crashes found locally or in CI are stored in `artifacts/`
- Use `cargo +nightly fuzz cmin decode_raw` to minimize the corpus
- Interesting minimal crashing inputs should be committed (they are very valuable)