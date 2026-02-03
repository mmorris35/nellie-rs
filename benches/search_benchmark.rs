//! Performance benchmarks for Nellie Production
//!
//! This suite measures critical performance metrics to verify the <200ms p95 latency requirement
//! specified in PROJECT_BRIEF.md. Benchmarks test database operations in isolation to identify
//! performance bottlenecks.
//!
//! **Benchmarks Included:**
//! - `vector_search`: Semantic code search latency at 100, 1000, and 10000 chunks
//! - `database_insert_batch`: Chunk insertion throughput (100 chunks per batch)
//! - `database_read_single_chunk`: Single chunk read latency
//! - `embedding_generation`: Placeholder embedding vector generation (384-dimensional)
//!
//! **Run benchmarks:**
//! ```bash
//! cargo bench                                    # Run all benchmarks
//! cargo bench -- vector_search                   # Vector search only
//! cargo bench -- database_insert_batch           # Database insert only
//! cargo bench -- --baseline baseline_name        # Compare to baseline
//! ```
//!
//! **Performance Target:**
//! - Vector search p95 latency: <200ms at 1M chunks (requires real scale testing)
//! - These benchmarks provide performance baseline at smaller scales
//!
//! **Notes:**
//! - Benchmarks use in-memory databases (TempDir) to isolate database performance
//! - Embeddings are placeholder normalized vectors (384 dimensions)
//! - Sample size is 10 (criterion minimum) for reasonable test duration
//! - Measurement time is 5 seconds per benchmark

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use nellie::storage::{
    get_chunk, init_storage, insert_chunk, insert_chunks_batch, search_chunks, ChunkRecord,
    Database, SearchOptions,
};
use tempfile::TempDir;

/// Create an in-memory test database for benchmarking.
///
/// Returns None if the sqlite-vec extension is not available (expected in some environments).
fn create_benchmark_db() -> Option<(TempDir, Database)> {
    let tmpdir = TempDir::new().expect("failed to create temp dir");
    let db_path = tmpdir.path().join("bench.db");
    let db = Database::open(&db_path).expect("failed to open database");

    // Initialize storage - this will fail if sqlite-vec is not available
    // In that case, return None so benchmarks can be skipped gracefully
    match init_storage(&db) {
        Ok(()) => Some((tmpdir, db)),
        Err(e) => {
            eprintln!(
                "Warning: sqlite-vec extension not available, skipping vector benchmarks: {}",
                e
            );
            None
        }
    }
}

/// Generate a simple placeholder embedding (384 dimensions for all-MiniLM-L6-v2).
fn generate_placeholder_embedding() -> Vec<f32> {
    // Generate a normalized random vector to simulate real embeddings
    // This isolates database performance from embedding generation
    let mut vec: Vec<f32> = (0..384).map(|i| (i as f32 * 0.1) % 1.0).collect();

    // Normalize the vector
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        vec.iter_mut().for_each(|x| *x /= norm);
    }

    vec
}

/// Create a test chunk with placeholder embedding.
fn create_test_chunk(file_path: &str, chunk_index: i32, content: &str) -> ChunkRecord {
    let mut chunk = ChunkRecord::new(
        file_path,
        chunk_index,
        chunk_index * 10 + 1,
        (chunk_index + 1) * 10,
        content,
        "abc123def456",
    );
    chunk.embedding = Some(generate_placeholder_embedding());
    chunk.language = Some("rust".to_string());
    chunk
}

/// Benchmark: Vector search at various chunk counts.
///
/// Requires sqlite-vec extension to be loaded. If not available, benchmarks are skipped
/// with a notice.
fn bench_vector_search(c: &mut Criterion) {
    // Check if sqlite-vec is available before setting up benchmarks
    if create_benchmark_db().is_none() {
        eprintln!("SKIP: Vector search benchmarks require sqlite-vec extension");
        return;
    }

    let mut group = c.benchmark_group("vector_search");
    // Criterion requires sample size >= 10, so use reasonable defaults
    group.sample_size(10); // Minimum allowed by criterion
    group.measurement_time(std::time::Duration::from_secs(5));

    for count in &[100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter_batched(
                || {
                    let (_tmpdir, db) =
                        create_benchmark_db().expect("sqlite-vec should be available");

                    // Populate with chunks
                    let mut chunks = Vec::new();
                    for i in 0..count {
                        let chunk = create_test_chunk(
                            &format!("src/file_{}.rs", i % 50),
                            (i / 50) as i32,
                            &format!("fn test_{}_() {{}}", i),
                        );
                        chunks.push(chunk);
                    }

                    // Batch insert for efficiency
                    db.with_conn(|conn| insert_chunks_batch(conn, &chunks))
                        .expect("batch insert failed");

                    (db, generate_placeholder_embedding())
                },
                |(db, query_embedding)| {
                    // Benchmark the search operation
                    db.with_conn(|conn| {
                        let _results = black_box(search_chunks(
                            conn,
                            &query_embedding,
                            &SearchOptions::new(10),
                        )?);
                        Ok(())
                    })
                    .expect("search failed");
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark: Single chunk read latency.
///
/// Measures the time to read a single chunk by ID. This operation is used when
/// retrieving specific chunks during search results or pagination.
fn bench_database_read(c: &mut Criterion) {
    // Check if database initialization works
    if create_benchmark_db().is_none() {
        eprintln!("SKIP: Database read benchmarks require sqlite-vec extension");
        return;
    }

    let mut group = c.benchmark_group("database_read");
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    group.bench_function("single_chunk_read", |b| {
        b.iter_batched(
            || {
                let (_tmpdir, db) = create_benchmark_db().expect("database should be available");

                // Insert a test chunk
                let chunk = create_test_chunk("src/test.rs", 0, "fn main() {}");
                let chunk_id = db
                    .with_conn(|conn| insert_chunk(conn, &chunk))
                    .expect("insert failed");

                (db, chunk_id)
            },
            |(db, chunk_id)| {
                // Benchmark reading it back
                db.with_conn(|conn| {
                    let _chunk = black_box(get_chunk(conn, chunk_id)?);
                    Ok::<(), nellie::Error>(())
                })
                .expect("read failed");
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark: Chunk insertion throughput.
///
/// Measures the time to insert chunks as batches (10, 100, 500). This is the typical
/// operation when indexing a file or batch of files.
fn bench_database_insert(c: &mut Criterion) {
    // Check if database initialization works
    if create_benchmark_db().is_none() {
        eprintln!("SKIP: Database insert benchmarks require sqlite-vec extension");
        return;
    }

    let mut group = c.benchmark_group("database_insert");
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    // Test batch sizes: 10, 100, and 500 chunks
    for batch_size in &[10, 100, 500] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_chunks", batch_size)),
            batch_size,
            |b, &batch_size| {
                b.iter_batched(
                    || create_benchmark_db().expect("database should be available"),
                    |(_tmpdir, db)| {
                        // Prepare chunks for insertion
                        let chunks: Vec<_> = (0..batch_size)
                            .map(|i| {
                                create_test_chunk(
                                    &format!("src/file_{}.rs", i),
                                    (i / 10) as i32,
                                    &format!("fn bench_test_{}_() {{}}", i),
                                )
                            })
                            .collect();

                        // Benchmark the batch insert
                        db.with_conn(|conn| {
                            let _ids = black_box(insert_chunks_batch(conn, &chunks)?);
                            Ok::<(), nellie::Error>(())
                        })
                        .expect("batch insert failed");
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark: Embedding generation time (simulated with placeholder vectors).
///
/// This measures the time to generate placeholder embeddings at various dimensions.
/// Real embedding generation requires an ONNX model and would measure actual inference time.
///
/// To benchmark real embeddings with an ONNX model in production:
/// 1. Place the embedding model file in the data directory
/// 2. Modify this function to use the actual embedding worker
/// 3. Run: `cargo bench -- embedding_generation`
fn bench_embedding_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("embedding_generation");
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(5));

    group.bench_function("placeholder_384_dim", |b| {
        b.iter(|| {
            let _vec = black_box(generate_placeholder_embedding());
        });
    });

    // Also measure vector normalization overhead
    group.bench_function("vector_normalization_384_dim", |b| {
        b.iter(|| {
            let mut vec: Vec<f32> = (0..384).map(|i| (i as f32 * 0.1) % 1.0).collect();
            let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                vec.iter_mut().for_each(|x| *x /= norm);
            }
            black_box(vec);
        });
    });

    group.finish();
}

// Define benchmark groups
criterion_group!(
    benches,
    bench_vector_search,
    bench_database_read,
    bench_database_insert,
    bench_embedding_generation,
);

criterion_main!(benches);
