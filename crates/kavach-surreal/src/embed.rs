// Local text embeddings via fastembed (BGE-small-en-v1.5, 384-dim).
// Embedder wraps the blocking ONNX call in tokio::task::spawn_blocking.
// See decision/crate-fastembed-bge-small for crate selection rationale.
use crate::error::{Error, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::Arc;

pub const EMBED_DIM: usize = 384;

#[derive(Clone)]
pub struct Embedder {
    inner: Arc<TextEmbedding>,
}

impl std::fmt::Debug for Embedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Embedder")
            .field("inner", &"TextEmbedding(opaque)")
            .finish()
    }
}

impl Embedder {
    /// Initialise the BGE-small embedder. Downloads the ONNX model on first
    /// call and caches it under the fastembed default model dir.
    ///
    /// # Errors
    /// Returns `Error::Migration` when fastembed cannot fetch or initialise
    /// the model (network failure, disk-full, corrupted cache).
    pub fn try_new() -> Result<Self> {
        let opts = InitOptions::new(EmbeddingModel::BGESmallENV15);
        let model = TextEmbedding::try_new(opts)
            .map_err(|e| Error::Migration(format!("fastembed init: {e}")))?;
        Ok(Self {
            inner: Arc::new(model),
        })
    }

    /// Embed a single string into a 384-dim BGE vector.
    ///
    /// # Errors
    /// `Error::Migration` for spawn-blocking join failure, fastembed
    /// inference failure, or empty result.
    pub async fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let model = Arc::<TextEmbedding>::clone(&self.inner);
        let owned = text.to_owned();
        let mut vecs = tokio::task::spawn_blocking(move || model.embed(vec![owned], None))
            .await
            .map_err(|e| Error::Migration(format!("embed join: {e}")))?
            .map_err(|e| Error::Migration(format!("embed: {e}")))?;
        vecs.pop()
            .ok_or_else(|| Error::Migration("embed returned empty".into()))
    }

    /// Embed a batch of strings into 384-dim BGE vectors in one ONNX call.
    ///
    /// # Errors
    /// `Error::Migration` for spawn-blocking join or fastembed inference
    /// failure.
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let model = Arc::<TextEmbedding>::clone(&self.inner);
        tokio::task::spawn_blocking(move || model.embed(texts, None))
            .await
            .map_err(|e| Error::Migration(format!("embed_batch join: {e}")))?
            .map_err(|e| Error::Migration(format!("embed_batch: {e}")))
    }
}

/// Cosine similarity between two BGE-small (384-dim) f32 vectors.
/// Returns 0.0 on dim mismatch, empty input, or zero-norm vectors.
///
/// f32 arithmetic is required by the BGE embedding contract — vectors are
/// L2-normalised f32; integer fixed-point loses recall on the (-1.0, 1.0)
/// similarity range. SOURCE: <https://huggingface.co/BAAI/bge-small-en-v1.5>.
#[must_use]
#[expect(
    clippy::float_arithmetic,
    reason = "BGE-small embedding similarity requires f32 cosine per model card"
)]
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (dot, na, nb) = a
        .iter()
        .zip(b.iter())
        .fold((0.0f32, 0.0f32, 0.0f32), |(d, sa, sb), (x, y)| {
            (d + x * y, sa + x * x, sb + y * y)
        });
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}
