# AFL++ Integration for Go Fuzzing

This document explains how to use **AFL++** with the Go reference implementation of Framed Message Protocol.

## Why AFL++?

While Go's built-in fuzzer (`go test -fuzz`) is excellent and easy to use, **AFL++** offers:

- More advanced mutation strategies
- Better performance in some cases
- Persistent mode (very fast)
- Custom mutators and dictionaries
- Excellent for security research and deep protocol fuzzing

## Recommended Approach (2026)

### Option 1: AFL++ + Native Go Fuzzer (Simplest)

You can run AFL++ against Go's native fuzzer binary:

```bash
# Build an instrumented test binary
cd go
go test -c -o fuzzer.test

# Run with AFL++
AFLplusplus/afl-fuzz -i corpus -o afl-out ./fuzzer.test -test.fuzz=FuzzDecodeRaw
```

> Note: This works but is not the most efficient.

### Option 2: go-fuzz (dvyukov) + AFL++ (Recommended for AFL++ power users)

The classic `go-fuzz` tool has excellent AFL++ support.

```bash
# Install go-fuzz
go install github.com/dvyukov/go-fuzz/go-fuzz@latest

# Build fuzzer
cd go
go-fuzz-build .

# Run with AFL++
afl-fuzz -i corpus -o afl-out ./fuzzer
```

## Persistent Mode Example

For maximum performance with AFL++ we recommend writing a persistent mode fuzzer.

See `persistent_fuzz.go` in this directory for an example.

## Corpus

Reuse the same corpus as the native fuzzer or generate one with:

```bash
go test -fuzz=FuzzDecodeRaw -fuzztime=1m
```

Then copy interesting inputs to `corpus/`.

## Integration with Project CI

Currently the project uses native Go fuzzing in CI. AFL++ is intended for local deep security research and manual longer runs.