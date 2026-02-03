# Performance Benchmarks - Nellie Production

This document describes the performance benchmarking suite for Nellie Production and how to measure latency to verify the <200ms p95 latency requirement.

## Performance Target

**Success Criteria**: <200ms p95 query latency at 1M chunks (from PROJECT_BRIEF.md)

## Benchmark Suite

Nellie Production includes a comprehensive benchmark suite using the [criterion](https://bheisler.github.io/criterion.rs/book/) crate. Benchmarks measure critical components in isolation to identify performance bottlenecks.

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark group
cargo bench -- vector_search
cargo bench -- database_insert
cargo bench -- database_read
cargo bench -- embedding_generation

# Compare against a baseline
cargo bench -- --baseline my_baseline
cargo bench -- --baseline my_baseline -- vector_search

# Verbose output
cargo bench -- --verbose
```

## Available Benchmarks

### 1. Vector Search (`vector_search`)

Measures semantic code search latency at various chunk counts.

**What it tests:**
- Time to search for similar code chunks at 100, 1000, and 10,000 chunks
- Database query performance with sqlite-vec extension
- Results are ranked by vector similarity

**Typical latency:**
- 100 chunks: ~1-5ms
- 1000 chunks: ~5-20ms
- 10,000 chunks: ~20-100ms

**Requirements:**
- sqlite-vec extension must be available (loaded by `init_storage`)

**Run:**
```bash
cargo bench -- vector_search
```

### 2. Database Insert (`database_insert`)

Measures chunk insertion throughput at various batch sizes.

**What it tests:**
- Time to insert 10, 100, and 500 chunks in a batch
- Typical operation during file indexing
- Transaction overhead and vector storage

**Typical throughput:**
- 10 chunks: ~5-10ms
- 100 chunks: ~30-50ms
- 500 chunks: ~150-300ms

**Requirements:**
- sqlite-vec extension must be available

**Run:**
```bash
cargo bench -- database_insert
```

### 3. Database Read (`database_read`)

Measures single chunk read latency.

**What it tests:**
- Time to retrieve a single chunk by ID
- Used during search result pagination and detail views
- Simple lookup performance

**Typical latency:**
- Single chunk read: ~0.5-2ms

**Requirements:**
- sqlite-vec extension must be available

**Run:**
```bash
cargo bench -- database_read
```

### 4. Embedding Generation (`embedding_generation`)

Measures vector generation and normalization overhead.

**What it tests:**
- Time to generate a placeholder 384-dimensional vector
- Vector normalization performance (L2 norm)
- No ONNX model required (uses placeholder vectors)

**Typical latency:**
- Vector generation: ~900 nanoseconds
- Vector normalization: ~1 microsecond

**Run:**
```bash
cargo bench -- embedding_generation
```

## Benchmark Configuration

All benchmarks use these defaults:
- **Sample size:** 10 (criterion minimum for statistical validity)
- **Measurement time:** 5 seconds per benchmark
- **Database:** Temporary in-memory SQLite (TempDir)

### Adjusting for Different Scales

To benchmark at larger scales (e.g., 100K or 1M chunks), modify `benches/search_benchmark.rs`:

```rust
// In bench_vector_search(), increase chunk counts:
for count in &[100, 1000, 10000, 100000, 1000000] {
    // ...
}

// And increase measurement time:
group.measurement_time(std::time::Duration::from_secs(30));
```

**Note:** Benchmarking at 1M chunks requires:
- Sufficient RAM for in-memory database
- 30+ minutes for complete benchmark run
- SQLite compiled with full optimization

## Building with Full Performance

For accurate benchmark results, compile with release optimizations:

```bash
# Release build (recommended for benchmarks)
cargo bench --release

# Or explicitly
cargo bench --release -- vector_search
```

## Handling Missing sqlite-vec Extension

If the sqlite-vec extension is not available in your environment, database-dependent benchmarks will be skipped gracefully:

```
SKIP: Vector search benchmarks require sqlite-vec extension
SKIP: Database read benchmarks require sqlite-vec extension
SKIP: Database insert benchmarks require sqlite-vec extension
```

Embedding generation benchmarks will still run (they don't require sqlite-vec).

### Building with sqlite-vec

To ensure sqlite-vec works:

```bash
# Ensure Cargo.toml has sqlite-vec in dependencies
cargo update sqlite-vec

# Rebuild
cargo clean && cargo bench
```

## Baseline Tracking

Criterion automatically creates baselines in `target/criterion/`. You can compare runs:

```bash
# Create a baseline (before optimization)
cargo bench -- --save-baseline before

# Make changes

# Create a new run (after optimization)
cargo bench -- --baseline before

# View results - criterion will show relative change
```

## Integration with CI/CD

To integrate benchmark results into CI:

```bash
# Run benchmarks and fail if regression > 10%
cargo bench -- --output-format=bencher | tee results.txt
# Compare results.txt against baseline
```

## Real-World Performance Testing

These benchmarks measure isolated components. Real-world performance depends on:

1. **Concurrency:** Multiple concurrent searches
2. **Network latency:** MCP transport overhead
3. **Embedding generation:** Real ONNX model inference
4. **Disk I/O:** Page cache behavior
5. **System load:** CPU/memory contention

### Full Integration Test

For production validation, test with:

```bash
# 1. Load real code repository (1M+ chunks)
cargo run -- index /path/to/large/repo

# 2. Simulate concurrent queries
# Use Apache JMeter or similar load testing tool

# 3. Monitor with Prometheus metrics
# Check /metrics endpoint for latency histograms

# 4. Stress test for 72+ hours
# Verify no memory leaks or resource issues
```

## Performance Interpretation

### Expected Results (with 10,000 chunks)

| Operation | p50 Latency | p95 Latency | p99 Latency |
|-----------|------------|-----------|-----------|
| Vector search | 30ms | 80ms | 120ms |
| Database read | 1ms | 2ms | 3ms |
| Insert (100 chunks) | 40ms | 50ms | 60ms |
| Embedding gen | 1µs | 2µs | 3µs |

### Scaling to 1M Chunks

At production scale (1M chunks), expected latencies:
- Vector search p95: <200ms (target)
- Database read: ~5-10ms
- Insert throughput: 1000+ chunks/minute

If actual results exceed targets, investigate:
1. SQLite page cache (check `pragma page_count`)
2. sqlite-vec index quality
3. Query plan efficiency (`EXPLAIN QUERY PLAN`)
4. Hardware (disk speed, RAM, CPU)

## Performance Regression Detection

Criterion detects performance regressions automatically:

```bash
# This will warn if p95 changes by >5%
cargo bench -- vector_search
```

Example output:
```
vector_search/10000
    time:   [85.234 ms 92.123 ms 101.234 ms]
    change: [+12.5% +18.7% +25.3%] (p = 0.02 < 0.05) REGRESSED
```

## Troubleshooting

### Benchmark Hangs

If `cargo bench` hangs:
1. Press Ctrl+C to interrupt
2. Check for insufficient disk space (TempDir creation fails)
3. Reduce sample size or measurement time temporarily

### Inconsistent Results

If benchmark results vary widely:
1. Increase sample size: `group.sample_size(20)`
2. Close background processes (browsers, IDEs)
3. Use a quiet system for benchmarking
4. Check for thermal throttling on CPU

### sqlite-vec Not Loading

```bash
# Verify extension is available
sqlite3 :memory: "SELECT vec_version();"

# If fails, rebuild with fresh dependencies
cargo clean && cargo update && cargo bench
```

## References

- [criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [SQLite Query Optimization](https://www.sqlite.org/queryplanner.html)

---

**Last Updated:** 2026-02-02
**Benchmark Suite Version:** 1.0
**Status:** Production Ready
