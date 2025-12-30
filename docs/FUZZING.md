# Fuzzing Guide for RustLite

## Overview

This guide covers security hardening through comprehensive fuzzing of RustLite's parsers, file formats, and APIs. Fuzzing helps discover crashes, panics, memory safety issues, and edge cases that traditional testing might miss.

## What is Fuzzing?

Fuzzing (or fuzz testing) is an automated software testing technique that provides invalid, unexpected, or random data as inputs to a program. The goal is to find:

- **Crashes and panics** - Unexpected program termination
- **Memory safety violations** - Buffer overflows, use-after-free, etc.
- **Assertion failures** - Violated invariants
- **Infinite loops** - Non-terminating execution
- **Resource exhaustion** - Excessive memory/CPU usage

## Prerequisites

### Install cargo-fuzz

```bash
cargo install cargo-fuzz
```

**System Requirements:**
- Rust nightly toolchain (required for fuzzing)
- Linux/macOS (recommended) or Windows WSL
- At least 4GB RAM for fuzzing
- Sufficient disk space for corpus (~1GB)

### Install Rust Nightly

```bash
rustup install nightly
rustup default nightly  # Temporarily switch to nightly
```

## Fuzz Targets

RustLite includes 5 comprehensive fuzz targets:

### 1. Query Parser (`fuzz_query_parser`)

**What it tests:**
- SQL query parsing
- Lexer tokenization
- Parser error handling
- Invalid syntax handling

**Attack vectors:**
- Malformed SQL queries
- Deeply nested expressions
- Invalid operators
- Buffer overflow attempts in string literals
- Unicode edge cases

**Run it:**
```bash
cargo +nightly fuzz run fuzz_query_parser
```

### 2. WAL Record Parser (`fuzz_wal_record`)

**What it tests:**
- WAL record encoding/decoding
- CRC validation
- Binary format parsing
- Transaction markers

**Attack vectors:**
- Corrupted WAL files
- Invalid record types
- Truncated records
- Malformed transaction boundaries
- CRC manipulation

**Run it:**
```bash
cargo +nightly fuzz run fuzz_wal_record
```

### 3. SSTable Reader (`fuzz_sstable_reader`)

**What it tests:**
- SSTable file format parsing
- Footer validation
- Data block reading
- Index block parsing

**Attack vectors:**
- Corrupted SSTable files
- Invalid footer structures
- Malformed bloom filters
- Truncated files
- Out-of-bounds offsets

**Run it:**
```bash
cargo +nightly fuzz run fuzz_sstable_reader
```

### 4. Database Operations (`fuzz_database_ops`)

**What it tests:**
- Database API operations
- Operation sequencing
- State consistency
- Error recovery

**Attack vectors:**
- Random operation sequences
- Interleaved put/get/delete
- Flush at arbitrary times
- Concurrent-like patterns

**Run it:**
```bash
cargo +nightly fuzz run fuzz_database_ops
```

### 5. Key-Value Pairs (`fuzz_key_value`)

**What it tests:**
- Input validation
- Size limits
- Edge cases (empty keys/values)
- Unicode handling

**Attack vectors:**
- Oversized keys/values
- Empty inputs
- Non-UTF8 data
- Boundary conditions

**Run it:**
```bash
cargo +nightly fuzz run fuzz_key_value
```

## Running Fuzz Tests

### Quick Test (1 minute each)

Test all targets quickly to ensure they work:

```bash
cd d:/rustlite

# Test each target for 60 seconds
cargo +nightly fuzz run fuzz_query_parser -- -max_total_time=60
cargo +nightly fuzz run fuzz_wal_record -- -max_total_time=60
cargo +nightly fuzz run fuzz_sstable_reader -- -max_total_time=60
cargo +nightly fuzz run fuzz_database_ops -- -max_total_time=60
cargo +nightly fuzz run fuzz_key_value -- -max_total_time=60
```

### Short Fuzzing Session (1 hour each)

```bash
cargo +nightly fuzz run fuzz_query_parser -- -max_total_time=3600
cargo +nightly fuzz run fuzz_wal_record -- -max_total_time=3600
cargo +nightly fuzz run fuzz_sstable_reader -- -max_total_time=3600
cargo +nightly fuzz run fuzz_database_ops -- -max_total_time=3600
cargo +nightly fuzz run fuzz_key_value -- -max_total_time=3600
```

### Production Fuzzing (24 hours each)

For serious security hardening, run each target for 24 hours:

```bash
# Run in background or separate terminals
cargo +nightly fuzz run fuzz_query_parser -- -max_total_time=86400 &
cargo +nightly fuzz run fuzz_wal_record -- -max_total_time=86400 &
cargo +nightly fuzz run fuzz_sstable_reader -- -max_total_time=86400 &
cargo +nightly fuzz run fuzz_database_ops -- -max_total_time=86400 &
cargo +nightly fuzz run fuzz_key_value -- -max_total_time=86400 &
```

### Continuous Fuzzing (7 days)

For v1.0 production readiness:

```bash
# Run each target for 7 days (604800 seconds)
cargo +nightly fuzz run fuzz_query_parser -- -max_total_time=604800
```

## Understanding Fuzz Output

### Successful Run

```text
INFO: Running with entropic power schedule (0xFF, 100).
INFO: Seed: 1234567890
INFO: Loaded 1 modules   (12345 inline 8-bit counters): 12345 [0x..., 0x...)
INFO: Loaded 1 PC tables (12345 PCs): 12345 [0x...,0x...)
INFO: -max_total_time=60 seconds
#2      INITED cov: 234 ft: 567 corp: 1/1b exec/s: 0 rss: 45Mb
#1000   NEW    cov: 456 ft: 890 corp: 23/45b lim: 4096 exec/s: 500 rss: 67Mb
#2000   NEW    cov: 567 ft: 1234 corp: 45/123b lim: 4096 exec/s: 666 rss: 89Mb
```

**Key metrics:**
- `cov`: Code coverage (higher is better)
- `ft`: Features (unique code paths)
- `corp`: Corpus size (interesting inputs found)
- `exec/s`: Executions per second (higher is better)
- `rss`: Memory usage

### Crash Found!

```text
==12345==ERROR: AddressSanitizer: heap-buffer-overflow
    #0 0x... in parse_query
    #1 0x... in Parser::parse
...
SUMMARY: AddressSanitizer: heap-buffer-overflow
```

**What to do:**
1. Crash artifact saved to `fuzz/artifacts/<target>/<hash>`
2. Reproduce locally: `cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<hash>`
3. Debug with standard tools
4. Fix the issue
5. Re-run fuzzing to verify fix

## Reproducing Crashes

When a crash is found:

```bash
# Reproduce the crash
cargo +nightly fuzz run fuzz_query_parser fuzz/artifacts/fuzz_query_parser/crash-abc123

# Run with debugger
rust-lldb -- target/x86_64-unknown-linux-gnu/release/fuzz_query_parser fuzz/artifacts/fuzz_query_parser/crash-abc123

# Or use gdb
rust-gdb --args target/x86_64-unknown-linux-gnu/release/fuzz_query_parser fuzz/artifacts/fuzz_query_parser/crash-abc123
```

## Minimizing Crash Inputs

LibFuzzer can minimize crash-causing inputs:

```bash
# Find the smallest input that still causes the crash
cargo +nightly fuzz cmin fuzz_query_parser

# Minimize a specific crash
cargo +nightly fuzz tmin fuzz_query_parser fuzz/artifacts/fuzz_query_parser/crash-abc123
```

## Coverage Analysis

Check which code is being tested:

```bash
# Generate coverage report
cargo +nightly fuzz coverage fuzz_query_parser

# View coverage with llvm-cov
llvm-cov show target/x86_64-unknown-linux-gnu/coverage/fuzz_query_parser \
    --format=html \
    --instr-profile=fuzz/coverage/fuzz_query_parser/coverage.profdata \
    > coverage.html
```

## Advanced Options

### Parallel Fuzzing

Run multiple fuzzing jobs in parallel:

```bash
# Run 4 parallel jobs
cargo +nightly fuzz run fuzz_query_parser -- -jobs=4 -workers=4
```

### Dictionary

Create a dictionary file for better fuzzing:

```bash
cat > fuzz/dictionary.txt << EOF
"SELECT"
"FROM"
"WHERE"
"JOIN"
"INNER"
"LEFT"
"RIGHT"
EOF

cargo +nightly fuzz run fuzz_query_parser -- -dict=fuzz/dictionary.txt
```

### Custom Timeouts

Prevent timeouts for slow operations:

```bash
# 10 second timeout per input
cargo +nightly fuzz run fuzz_database_ops -- -timeout=10
```

### Memory Limit

Prevent excessive memory usage:

```bash
# Limit to 2GB RSS
cargo +nightly fuzz run fuzz_sstable_reader -- -rss_limit_mb=2048
```

## Continuous Integration

Add fuzzing to CI pipeline:

```yaml
# .github/workflows/fuzz.yml
name: Fuzzing
on: [push, pull_request]

jobs:
  fuzz:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target:
          - fuzz_query_parser
          - fuzz_wal_record
          - fuzz_sstable_reader
          - fuzz_database_ops
          - fuzz_key_value
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: nightly
      - run: cargo install cargo-fuzz
      - run: cargo +nightly fuzz run ${{ matrix.target }} -- -max_total_time=300
```

## Interpreting Results

### Good Signs ✅

- No crashes after 1+ hours
- Coverage increasing steadily
- Corpus growing with new interesting inputs
- High exec/s rate (>1000)

### Red Flags 🚩

- Crashes within minutes
- Low coverage (<50%)
- Corpus not growing
- Very slow exec/s (<100)
- Memory leaks (RSS constantly increasing)

## Common Issues

### Issue: Fuzzing is very slow

**Solution:**
- Reduce input size limits in fuzz targets
- Use simpler operations
- Profile to find hotspots
- Add `#[inline]` to hot functions

### Issue: No new coverage

**Solution:**
- Add dictionary file with common patterns
- Seed corpus with valid inputs
- Simplify fuzz target to focus on specific code
- Check if code is actually reachable

### Issue: Timeouts

**Solution:**
- Add operation limits (max iterations)
- Increase timeout: `-timeout=30`
- Fix infinite loops in code

### Issue: Out of memory

**Solution:**
- Limit input sizes
- Add memory limit: `-rss_limit_mb=2048`
- Fix memory leaks

## Best Practices

### 1. Start Simple

Begin with short fuzzing sessions (1 minute) to ensure targets work:

```bash
cargo +nightly fuzz run fuzz_query_parser -- -max_total_time=60
```

### 2. Gradually Increase Duration

```bash
# 5 minutes
cargo +nightly fuzz run fuzz_query_parser -- -max_total_time=300

# 1 hour
cargo +nightly fuzz run fuzz_query_parser -- -max_total_time=3600

# Overnight (8 hours)
cargo +nightly fuzz run fuzz_query_parser -- -max_total_time=28800
```

### 3. Monitor Progress

Keep an eye on:
- Coverage growth
- Execution speed
- Memory usage
- Crash artifacts

### 4. Fix Issues Immediately

Don't accumulate technical debt:
- Reproduce crash locally
- Write regression test
- Fix the issue
- Verify with fuzzing

### 5. Seed Corpus

Add known-good inputs to speed up fuzzing:

```bash
mkdir -p fuzz/corpus/fuzz_query_parser
echo "SELECT * FROM users WHERE id = 1" > fuzz/corpus/fuzz_query_parser/valid1
echo "SELECT name, age FROM users" > fuzz/corpus/fuzz_query_parser/valid2
```

## Production Checklist

For v1.0 release, complete this checklist:

- [ ] All fuzz targets run for 24+ hours without crashes
- [ ] Code coverage >80% for fuzzed components
- [ ] No memory leaks detected
- [ ] No panics on invalid input
- [ ] Graceful error handling for all edge cases
- [ ] Corpus saved and version controlled
- [ ] Fuzzing integrated into CI/CD
- [ ] Regression tests added for all found issues

## Further Reading

- [LibFuzzer Documentation](https://llvm.org/docs/LibFuzzer.html)
- [cargo-fuzz Book](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [Fuzzing Rust Code](https://rust-fuzz.github.io/book/)
- [OSS-Fuzz](https://github.com/google/oss-fuzz) - Continuous fuzzing for open source

## Support

If you find crashes or have questions about fuzzing:
- File an issue: https://github.com/VIRTUMEM-AI-LABS/rustlite/issues
- Security issues: security@rustlite.dev
