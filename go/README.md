# Go Reference Implementation

Native Go implementation of the Framed Message Protocol (FMP).

## Running Tests

```bash
go test ./...
```

## Fuzzing

```bash
# Short local fuzzing
go test -fuzz=FuzzDecodeRaw -fuzztime=30s
go test -fuzz=FuzzRoundtrip -fuzztime=1m

# All fuzz targets
go test -fuzz=. -fuzztime=2m
```

Go's built-in fuzzer is coverage-guided and very effective for finding parser bugs.

## CI Integration

- Short fuzz sessions (30s per target) run on every push/PR in `ci.yml`
- Longer fuzz sessions run weekly in the scheduled `fuzz.yml` workflow
- Crashing inputs from CI are uploaded as artifacts