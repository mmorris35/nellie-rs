//! Prometheus metrics definitions.

use once_cell::sync::Lazy;
use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge, HistogramVec,
    IntCounterVec, IntGauge,
};

/// Total chunks indexed.
pub static CHUNKS_TOTAL: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!("nellie_chunks_total", "Total number of indexed code chunks").unwrap()
});

/// Total lessons stored.
pub static LESSONS_TOTAL: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!("nellie_lessons_total", "Total number of lessons stored").unwrap()
});

/// Total files tracked.
pub static FILES_TOTAL: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!("nellie_files_total", "Total number of tracked files").unwrap()
});

/// Request latency histogram.
pub static REQUEST_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "nellie_request_duration_seconds",
        "Request latency in seconds",
        &["endpoint", "method"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    )
    .unwrap()
});

/// Request counter.
pub static REQUEST_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "nellie_requests_total",
        "Total number of requests",
        &["endpoint", "method", "status"]
    )
    .unwrap()
});

/// Embedding queue depth.
pub static EMBEDDING_QUEUE_DEPTH: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "nellie_embedding_queue_depth",
        "Number of items waiting for embedding"
    )
    .unwrap()
});

/// Initialize all metrics (call once at startup).
pub fn init_metrics() {
    // Access lazy statics to register them
    let _ = &*CHUNKS_TOTAL;
    let _ = &*LESSONS_TOTAL;
    let _ = &*FILES_TOTAL;
    let _ = &*REQUEST_LATENCY;
    let _ = &*REQUEST_COUNT;
    let _ = &*EMBEDDING_QUEUE_DEPTH;

    tracing::debug!("Prometheus metrics initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_init() {
        init_metrics();

        CHUNKS_TOTAL.set(100);
        assert_eq!(CHUNKS_TOTAL.get(), 100);

        LESSONS_TOTAL.set(50);
        assert_eq!(LESSONS_TOTAL.get(), 50);
    }
}
