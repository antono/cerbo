//! Text embeddings (design D5): the [`Embedder`] trait, the E5 text
//! conventions, and the pinned `multilingual-e5-small` model behind it.
//!
//! Page text never leaves the machine: the model runs in-process through ONNX
//! Runtime, and the only network access is the one-time model download.

use super::SemanticError;
use sha2::{Digest, Sha256};

/// Turns texts into vectors of one fixed width. Implementations return
/// unit-length vectors, so a dot product is the cosine similarity.
pub trait Embedder {
    /// Width of every vector this embedder returns.
    fn dim(&self) -> usize;

    /// Embed `texts` exactly as given; use [`passage_text`] and [`query_text`]
    /// to build them.
    fn embed(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>, SemanticError>;
}

/// Reports `(done, total)` pages while embeddings are computed.
pub type EmbedProgress<'a> = Option<&'a dyn Fn(usize, usize)>;

/// Characters of a page passed to the tokenizer. The model reads only its
/// first 512 tokens anyway; the cap only bounds tokenizer work on huge pages.
const PASSAGE_CHAR_LIMIT: usize = 32 * 1024;

/// The text embedded for a page: E5 wants `passage: ` before documents.
pub fn passage_text(title: &str, body: &str) -> String {
    let body: String = body.chars().take(PASSAGE_CHAR_LIMIT).collect();
    format!("passage: {title}\n{body}")
}

/// The text embedded for a search: E5 wants `query: ` before queries.
pub fn query_text(text: &str) -> String {
    format!("query: {text}")
}

/// Hash of what a page's embedding is computed from, its title and `page.md`
/// (design D6). A page is re-embedded only when this changes.
pub fn content_hash(title: &str, body: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(title.as_bytes());
    hasher.update(b"\n");
    hasher.update(body.as_bytes());
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Scale `v` to unit length; a zero vector stays zero.
pub(super) fn normalise(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter_mut().for_each(|x| *x /= norm);
    }
}

/// The pinned `multilingual-e5-small` through fastembed, with mean pooling and
/// the model's 512-token window.
#[cfg(feature = "embeddings")]
pub struct FastEmbedder {
    model: fastembed::TextEmbedding,
}

#[cfg(feature = "embeddings")]
impl FastEmbedder {
    /// Load the pinned model from `CERBO_MODEL_DIR` (verified, never
    /// downloaded) or from `<cache_dir>/models/<commit>/`, downloading it there
    /// on first use with progress on stderr.
    pub fn load(cache_dir: &std::path::Path) -> Result<Self, SemanticError> {
        use super::model;
        let env_dir = std::env::var_os(model::MODEL_DIR_ENV)
            .filter(|v| !v.is_empty())
            .map(std::path::PathBuf::from);
        let dir = model::resolve_model_dir(cache_dir, env_dir, &model::MODEL_FILES, &model::http_fetch)?;
        Self::load_verified(&dir)
    }

    /// Load the model files from `dir`, which [`super::model::resolve_model_dir`]
    /// has verified.
    fn load_verified(dir: &std::path::Path) -> Result<Self, SemanticError> {
        use super::model::{
            ModelFile, MODEL_CONFIG, MODEL_MAX_TOKENS, MODEL_ONNX, MODEL_SPECIAL_TOKENS_MAP, MODEL_TOKENIZER,
            MODEL_TOKENIZER_CONFIG,
        };
        use fastembed::{InitOptionsUserDefined, Pooling, TextEmbedding, TokenizerFiles, UserDefinedEmbeddingModel};

        let read = |file: ModelFile| {
            let path = dir.join(file.path);
            std::fs::read(&path).map_err(|e| SemanticError::ModelUnavailable(format!("{}: {e}", path.display())))
        };
        let tokenizer_files = TokenizerFiles {
            tokenizer_file: read(MODEL_TOKENIZER)?,
            config_file: read(MODEL_CONFIG)?,
            special_tokens_map_file: read(MODEL_SPECIAL_TOKENS_MAP)?,
            tokenizer_config_file: read(MODEL_TOKENIZER_CONFIG)?,
        };
        let model = UserDefinedEmbeddingModel::new(read(MODEL_ONNX)?, tokenizer_files).with_pooling(Pooling::Mean);
        let options = InitOptionsUserDefined::new().with_max_length(MODEL_MAX_TOKENS);

        // ONNX Runtime is loaded from ORT_DYLIB_PATH here; a missing or too old
        // library may panic inside `ort` rather than return an error.
        let loaded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            TextEmbedding::try_new_from_user_defined(model, options)
        }));
        match loaded {
            Ok(Ok(model)) => Ok(Self { model }),
            Ok(Err(e)) => Err(SemanticError::ModelUnavailable(format!(
                "cannot load the model with ONNX Runtime (ORT_DYLIB_PATH): {e}"
            ))),
            Err(_) => Err(SemanticError::ModelUnavailable(
                "cannot load ONNX Runtime: set ORT_DYLIB_PATH to libonnxruntime 1.24 or newer".into(),
            )),
        }
    }
}

#[cfg(feature = "embeddings")]
impl Embedder for FastEmbedder {
    fn dim(&self) -> usize {
        super::model::MODEL_DIM
    }

    fn embed(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>, SemanticError> {
        // fastembed already returns unit-length vectors.
        self.model
            .embed(texts, None)
            .map_err(|e| SemanticError::ModelUnavailable(format!("embedding failed: {e}")))
    }
}

/// A deterministic stand-in for the model in tests: a bag of words, each word
/// hashed to one dimension. Texts sharing words are similar; the E5 prefixes
/// are ignored. Every embedded text is recorded in [`MockEmbedder::embedded`].
#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, Clone)]
pub struct MockEmbedder {
    dim: usize,
    pub embedded: Vec<String>,
}

#[cfg(any(test, feature = "test-utils"))]
impl MockEmbedder {
    pub fn new(dim: usize) -> Self {
        Self { dim, embedded: Vec::new() }
    }

    /// The vector of `text`, without recording it.
    pub fn vector(&self, text: &str) -> Vec<f32> {
        let mut v = vec![0.0f32; self.dim];
        for word in text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_lowercase)
            .filter(|w| w != "passage" && w != "query")
        {
            // FNV-1a: stable across runs and platforms.
            let hash = word
                .bytes()
                .fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3));
            v[(hash % self.dim as u64) as usize] += 1.0;
        }
        normalise(&mut v);
        v
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl Default for MockEmbedder {
    fn default() -> Self {
        Self::new(1024)
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl Embedder for MockEmbedder {
    fn dim(&self) -> usize {
        self.dim
    }

    fn embed(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>, SemanticError> {
        self.embedded.extend(texts.iter().cloned());
        Ok(texts.iter().map(|t| self.vector(t)).collect())
    }
}

/// Loads the real embedder on demand.
type EmbedderLoader = Box<dyn FnOnce() -> Result<Box<dyn Embedder>, SemanticError>>;

/// An embedder that loads its model on the first [`Embedder::embed`] call, so
/// a command that ends before it needs the model (no embeddings yet, unknown
/// page, nothing changed) never downloads or loads it.
pub struct LazyEmbedder {
    dim: usize,
    loader: Option<EmbedderLoader>,
    loaded: Option<Box<dyn Embedder>>,
}

impl LazyEmbedder {
    /// `dim` must be the width of the vectors `loader`'s embedder returns.
    pub fn new(dim: usize, loader: impl FnOnce() -> Result<Box<dyn Embedder>, SemanticError> + 'static) -> Self {
        Self { dim, loader: Some(Box::new(loader)), loaded: None }
    }

    /// Whether the model has been loaded.
    pub fn is_loaded(&self) -> bool {
        self.loaded.is_some()
    }
}

impl Embedder for LazyEmbedder {
    fn dim(&self) -> usize {
        self.dim
    }

    fn embed(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>, SemanticError> {
        if self.loaded.is_none() {
            // A failed load is not retried: the error names what to fix.
            let loader = self
                .loader
                .take()
                .ok_or_else(|| SemanticError::ModelUnavailable("the model failed to load earlier".into()))?;
            self.loaded = Some(loader()?);
        }
        let embedder = self.loaded.as_mut().expect("loaded above");
        debug_assert_eq!(embedder.dim(), self.dim);
        embedder.embed(texts)
    }
}

/// Environment variable that swaps the model for [`MockEmbedder`] in
/// [`default_embedder`]. Only builds with the `test-utils` feature (the CLI's
/// integration tests) honour it.
pub const MOCK_EMBEDDER_ENV: &str = "CERBO_TEST_MOCK_EMBEDDER";

/// The embedder the commands use: the pinned model from `CERBO_MODEL_DIR` or
/// `<cache_dir>/models/`, loaded (and downloaded on first use) only when a
/// text is actually embedded.
#[cfg(feature = "embeddings")]
pub fn default_embedder(cache_dir: &std::path::Path) -> LazyEmbedder {
    #[cfg(feature = "test-utils")]
    if std::env::var_os(MOCK_EMBEDDER_ENV).is_some_and(|v| !v.is_empty()) {
        let dim = MockEmbedder::default().dim();
        return LazyEmbedder::new(dim, || Ok(Box::new(MockEmbedder::default()) as Box<dyn Embedder>));
    }
    let cache_dir = cache_dir.to_path_buf();
    LazyEmbedder::new(super::model::MODEL_DIM, move || {
        Ok(Box::new(FastEmbedder::load(&cache_dir)?) as Box<dyn Embedder>)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lazy_embedder_loads_once_on_first_embed() {
        use std::cell::Cell;
        use std::rc::Rc;

        let loads = Rc::new(Cell::new(0));
        let counter = Rc::clone(&loads);
        let mut lazy = LazyEmbedder::new(137, move || {
            counter.set(counter.get() + 1);
            Ok(Box::new(MockEmbedder::new(137)) as Box<dyn Embedder>)
        });
        assert_eq!(lazy.dim(), 137);
        assert!(!lazy.is_loaded(), "asking for the width must not load the model");

        let mock = MockEmbedder::new(137);
        let text = query_text("Berlin");
        assert_eq!(lazy.embed(std::slice::from_ref(&text)).unwrap(), vec![mock.vector(&text)]);
        lazy.embed(&[query_text("trip")]).unwrap();
        assert_eq!(loads.get(), 1);
    }

    #[test]
    fn lazy_embedder_reports_a_failed_load_every_time() {
        let mut lazy =
            LazyEmbedder::new(137, || Err(SemanticError::ModelUnavailable("model.onnx: checksum mismatch".into())));
        let first = lazy.embed(&[query_text("Berlin")]).unwrap_err().to_string();
        assert!(first.contains("model.onnx"), "{first}");
        let second = lazy.embed(&[query_text("Berlin")]).unwrap_err().to_string();
        assert!(second.contains("embedding model unavailable"), "{second}");
    }

    #[test]
    fn e5_prefixes_and_truncation() {
        assert_eq!(passage_text("Trip to Berlin", "Museums."), "passage: Trip to Berlin\nMuseums.");
        assert_eq!(query_text("поездка в Берлин"), "query: поездка в Берлин");
        let long = "я".repeat(PASSAGE_CHAR_LIMIT + 137);
        assert_eq!(passage_text("T", &long).chars().count(), "passage: T\n".len() + PASSAGE_CHAR_LIMIT);
    }

    #[test]
    fn content_hash_covers_title_and_body() {
        let base = content_hash("Berlin", "museums");
        assert_eq!(base, content_hash("Berlin", "museums"));
        assert_ne!(base, content_hash("Berlin!", "museums"));
        assert_ne!(base, content_hash("Berlin", "museums."));
        assert_eq!(base.len(), 64);
    }

    #[test]
    fn mock_embedder_is_deterministic_unit_length_and_records_texts() {
        let mut mock = MockEmbedder::new(137);
        let texts = vec![passage_text("Berlin", "city city"), query_text("city")];
        let vectors = mock.embed(&texts).unwrap();

        assert_eq!(mock.embedded, texts);
        assert_eq!(vectors[0], mock.vector(&texts[0]));
        for v in &vectors {
            assert_eq!(v.len(), 137);
            assert!((v.iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 1e-5);
        }
        let dot: f32 = vectors[0].iter().zip(&vectors[1]).map(|(a, b)| a * b).sum();
        assert!(dot > 0.5, "shared words must be similar, got {dot}");
    }

    /// The real pinned model through [`FastEmbedder::load`]: a Russian query
    /// ranks the English page about Berlin above an unrelated page.
    #[cfg(feature = "embeddings")]
    #[test]
    #[ignore = "needs the model (CERBO_MODEL_DIR or a network) and ONNX Runtime"]
    fn real_model_ranks_by_meaning_across_languages() {
        let cache = tempfile::tempdir().unwrap();
        let mut model = FastEmbedder::load(cache.path()).unwrap();
        let texts = vec![
            query_text("поездка в Берлин"),
            passage_text("Trip to Berlin", "Museums, the Spree and a weekend in Germany."),
            passage_text("Sourdough", "Feed the starter 137 g of flour every morning."),
        ];
        let v = model.embed(&texts).unwrap();
        assert_eq!(v[0].len(), model.dim());
        let dot = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
        let (berlin, bread) = (dot(&v[0], &v[1]), dot(&v[0], &v[2]));
        eprintln!("berlin {berlin}, bread {bread}");
        assert!(berlin > bread, "berlin {berlin} <= bread {bread}");
    }
}
