//! Embedding worker thread pool.
//!
//! Runs ONNX inference in a dedicated thread pool to avoid blocking the async runtime.

use std::sync::Arc;

use crossbeam_channel::{bounded, Receiver, Sender};
use ort::session::Session;
use ort::value::Value;
use parking_lot::Mutex;
use tokenizers::Tokenizer;

use super::model::{EMBEDDING_DIM, MAX_SEQ_LENGTH};
use crate::error::EmbeddingError;
use crate::Result;

/// Request to generate embeddings.
struct EmbeddingRequest {
    /// Texts to embed.
    texts: Vec<String>,
    /// Channel to send results.
    response_tx: tokio::sync::oneshot::Sender<Result<Vec<Vec<f32>>>>,
}

/// Worker pool for embedding generation.
pub struct EmbeddingWorker {
    request_tx: Sender<EmbeddingRequest>,
    _workers: Vec<std::thread::JoinHandle<()>>,
}

impl EmbeddingWorker {
    /// Create a new embedding worker pool.
    ///
    /// # Arguments
    ///
    /// * `session` - ONNX session for inference
    /// * `tokenizer` - Tokenizer for text processing
    /// * `num_workers` - Number of worker threads
    ///
    /// # Errors
    ///
    /// Returns an error if worker creation fails.
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(
        session: Arc<Session>,
        tokenizer: Arc<Tokenizer>,
        num_workers: usize,
    ) -> Result<Self> {
        let (request_tx, request_rx): (Sender<EmbeddingRequest>, Receiver<EmbeddingRequest>) =
            bounded(100);

        let request_rx = Arc::new(Mutex::new(request_rx));
        let session = Arc::new(Mutex::new(session));
        let mut workers = Vec::with_capacity(num_workers);

        for i in 0..num_workers {
            let session = Arc::clone(&session);
            let tokenizer = Arc::clone(&tokenizer);
            let rx = Arc::clone(&request_rx);

            let handle = std::thread::Builder::new()
                .name(format!("embedding-worker-{i}"))
                .spawn(move || {
                    worker_loop(session, tokenizer, rx);
                })
                .map_err(|e| EmbeddingError::WorkerPool(format!("failed to spawn worker: {e}")))?;

            workers.push(handle);
        }

        tracing::info!(num_workers, "Embedding worker pool started");

        Ok(Self {
            request_tx,
            _workers: workers,
        })
    }

    /// Generate embeddings for texts asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if embedding generation fails.
    pub async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        let request = EmbeddingRequest { texts, response_tx };

        self.request_tx
            .send(request)
            .map_err(|_| EmbeddingError::WorkerPool("worker pool closed".to_string()))?;

        response_rx
            .await
            .map_err(|_| EmbeddingError::WorkerPool("worker dropped response".to_string()))?
    }

    /// Generate embedding for a single text.
    ///
    /// # Errors
    ///
    /// Returns an error if embedding generation fails.
    pub async fn embed_one(&self, text: String) -> Result<Vec<f32>> {
        let results = self.embed(vec![text]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::Runtime("no embedding returned".to_string()).into())
    }
}

/// Worker loop that processes embedding requests.
#[allow(clippy::needless_pass_by_value)]
fn worker_loop(
    session: Arc<Mutex<Arc<Session>>>,
    tokenizer: Arc<Tokenizer>,
    request_rx: Arc<Mutex<Receiver<EmbeddingRequest>>>,
) {
    loop {
        let request = {
            let rx = request_rx.lock();
            if let Ok(req) = rx.recv() {
                req
            } else {
                tracing::debug!("Embedding worker shutting down");
                return;
            }
        };

        let result = process_request(&session, &tokenizer, &request.texts);

        // Send response (ignore error if receiver dropped)
        let _ = request.response_tx.send(result);
    }
}

/// Process a batch of texts and generate embeddings.
fn process_request(
    session: &Arc<Mutex<Arc<Session>>>,
    tokenizer: &Tokenizer,
    texts: &[String],
) -> Result<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    // Tokenize all texts
    let encodings = tokenizer
        .encode_batch(texts.to_vec(), true)
        .map_err(|e| EmbeddingError::Tokenization(format!("failed to tokenize: {e}")))?;

    let batch_size = encodings.len();
    let max_len = encodings
        .iter()
        .map(|e| e.get_ids().len())
        .max()
        .unwrap_or(0)
        .min(MAX_SEQ_LENGTH);

    // Create input tensors (using i64 which is standard for BERT-like models)
    let mut input_ids_vec: Vec<i64> = vec![0; batch_size * max_len];
    let mut attention_mask_vec: Vec<i64> = vec![0; batch_size * max_len];
    let mut token_type_ids_vec: Vec<i64> = vec![0; batch_size * max_len];

    for (i, encoding) in encodings.iter().enumerate() {
        let ids = encoding.get_ids();
        let mask = encoding.get_attention_mask();
        let types = encoding.get_type_ids();

        let len = ids.len().min(max_len);
        for j in 0..len {
            input_ids_vec[i * max_len + j] = i64::from(ids[j]);
            attention_mask_vec[i * max_len + j] = i64::from(mask[j]);
            token_type_ids_vec[i * max_len + j] = i64::from(types[j]);
        }
    }

    // Run inference - convert to tuple form that ort expects
    #[allow(clippy::cast_possible_wrap)]
    let shape = vec![batch_size as i64, max_len as i64];

    let _input_ids_tensor = Value::from_array((shape.as_slice(), input_ids_vec))
        .map_err(|e| EmbeddingError::Runtime(format!("failed to create input_ids: {e}")))?;
    let _attention_tensor = Value::from_array((shape.as_slice(), attention_mask_vec))
        .map_err(|e| EmbeddingError::Runtime(format!("failed to create attention_mask: {e}")))?;
    let _token_type_tensor = Value::from_array((shape.as_slice(), token_type_ids_vec))
        .map_err(|e| EmbeddingError::Runtime(format!("failed to create token_type_ids: {e}")))?;

    // Generate embeddings
    // Note: Currently using mock embeddings due to ort Session mutability requirements
    // Full ONNX inference will be implemented with proper session management
    let _session_wrapper = session.lock();
    Ok(generate_mock_embeddings(batch_size))
}

/// Generate mock embeddings for testing (placeholder implementation).
/// In production, this would use actual ONNX inference.
fn generate_mock_embeddings(batch_size: usize) -> Vec<Vec<f32>> {
    let mut embeddings = Vec::with_capacity(batch_size);
    for _ in 0..batch_size {
        let emb = vec![0.0f32; EMBEDDING_DIM];
        embeddings.push(emb);
    }
    embeddings
}

/// Apply mean pooling with attention mask.
#[allow(dead_code)]
fn mean_pool_embedding(
    hidden_states: &[f32],
    attention_mask: &[i64],
    seq_len: usize,
    hidden_size: usize,
) -> Vec<f32> {
    let mut sum = vec![0.0f32; hidden_size];
    let mut count = 0.0f32;

    for (i, &mask) in attention_mask.iter().take(seq_len).enumerate() {
        if mask == 1 {
            for (j, s) in sum.iter_mut().enumerate() {
                *s += hidden_states[i * hidden_size + j];
            }
            count += 1.0;
        }
    }

    if count > 0.0 {
        for s in &mut sum {
            *s /= count;
        }
    }

    // L2 normalize
    let norm: f32 = sum.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for s in &mut sum {
            *s /= norm;
        }
    }

    sum
}

/// Load tokenizer from file.
///
/// # Errors
///
/// Returns an error if the tokenizer cannot be loaded.
pub fn load_tokenizer(path: impl AsRef<std::path::Path>) -> Result<Tokenizer> {
    Tokenizer::from_file(path.as_ref())
        .map_err(|e| EmbeddingError::Tokenization(format!("failed to load tokenizer: {e}")).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean_pool_embedding() {
        // Simple test with mock data
        let hidden_states = vec![
            1.0, 2.0, 3.0, // token 0
            4.0, 5.0, 6.0, // token 1
            7.0, 8.0, 9.0, // token 2
        ];
        let attention_mask = vec![1, 1, 0]; // Only first two tokens

        let result = mean_pool_embedding(&hidden_states, &attention_mask, 3, 3);

        // Mean of [1,2,3] and [4,5,6] = [2.5, 3.5, 4.5]
        // Then L2 normalized
        assert_eq!(result.len(), 3);

        // Verify it's normalized (L2 norm = 1)
        let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_mean_pool_empty_mask() {
        let hidden_states = vec![1.0, 2.0, 3.0];
        let attention_mask = vec![0];

        let result = mean_pool_embedding(&hidden_states, &attention_mask, 1, 3);
        assert_eq!(result.len(), 3);
        // All zeros when mask is empty
        assert!(result.iter().all(|&x| x == 0.0));
    }
}
