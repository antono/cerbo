//! The pinned embedding model: `intfloat/multilingual-e5-small`, 384 dimensions,
//! mean pooling (from the model's `1_Pooling/config.json`).
//!
//! Cerbo downloads exactly these files at exactly this revision and refuses any
//! file whose SHA-256 differs, so every machine running the same release embeds
//! with byte-identical model files.

use super::SemanticError;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// Hugging Face repository of the model.
pub const MODEL_REPO: &str = "intfloat/multilingual-e5-small";

/// Pinned repository commit. Changing it requires new checksums below.
pub const MODEL_COMMIT: &str = "614241f622f53c4eeff9890bdc4f31cfecc418b3";

/// Embedding width of the model.
pub const MODEL_DIM: usize = 384;

/// Token window of the model; longer passages are truncated.
pub const MODEL_MAX_TOKENS: usize = 512;

/// A model file at [`MODEL_COMMIT`] and the SHA-256 it must have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelFile {
    /// Path inside the repository, also used as the relative path on disk.
    pub path: &'static str,
    /// Lower-case hex SHA-256 of the file contents.
    pub sha256: &'static str,
}

pub const MODEL_ONNX: ModelFile = ModelFile {
    path: "onnx/model.onnx",
    sha256: "ca456c06b3a9505ddfd9131408916dd79290368331e7d76bb621f1cba6bc8665",
};
pub const MODEL_TOKENIZER: ModelFile = ModelFile {
    path: "onnx/tokenizer.json",
    sha256: "0b44a9d7b51c3c62626640cda0e2c2f70fdacdc25bbbd68038369d14ebdf4c39",
};
pub const MODEL_CONFIG: ModelFile = ModelFile {
    path: "onnx/config.json",
    sha256: "bbb7c1333fc4b3e27fbc9cd5d2070aabcc1d4dfb99917c3633e772f97545a6b6",
};
pub const MODEL_SPECIAL_TOKENS_MAP: ModelFile = ModelFile {
    path: "onnx/special_tokens_map.json",
    sha256: "d05497f1da52c5e09554c0cd874037a083e1dc1b9cfd48034d1c717f1afc07a7",
};
pub const MODEL_TOKENIZER_CONFIG: ModelFile = ModelFile {
    path: "onnx/tokenizer_config.json",
    sha256: "a1d6bc8734a6f635dc158508bef000f8e2e5a759c7d92f984b2c86e5ff53425b",
};

/// Every file the model needs, in download order.
pub const MODEL_FILES: [ModelFile; 5] = [
    MODEL_ONNX,
    MODEL_TOKENIZER,
    MODEL_CONFIG,
    MODEL_SPECIAL_TOKENS_MAP,
    MODEL_TOKENIZER_CONFIG,
];

/// Download URL of `file` at the pinned revision.
pub fn model_file_url(file: &ModelFile) -> String {
    format!(
        "https://huggingface.co/{}/resolve/{}/{}",
        MODEL_REPO, MODEL_COMMIT, file.path
    )
}

/// Environment variable naming a directory with the pinned model files. When
/// set, the files are verified there and nothing is ever downloaded.
pub const MODEL_DIR_ENV: &str = "CERBO_MODEL_DIR";

/// `<cache>/models/<commit>/`: where downloaded model files are kept.
pub fn cached_model_dir(cache_dir: &Path) -> PathBuf {
    cache_dir.join("models").join(MODEL_COMMIT)
}

/// Downloads one model file into an open temporary file. Injected so the
/// verification logic is testable without a network.
pub type Fetch<'a> = &'a dyn Fn(&ModelFile, &mut File) -> Result<(), String>;

/// Lower-case hex SHA-256 of the file at `path`, read in chunks.
pub fn file_sha256(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn unavailable(message: String) -> SemanticError {
    SemanticError::ModelUnavailable(message)
}

fn mismatch(path: &Path, file: &ModelFile, actual: &str) -> String {
    format!(
        "{} checksum mismatch: SHA-256 is {actual}, expected {}",
        path.display(),
        file.sha256
    )
}

/// Check every file of `files` in a directory the user provided
/// ([`MODEL_DIR_ENV`]). A missing or mismatching file is an error naming it;
/// nothing is downloaded and nothing is deleted.
pub fn verify_model_dir(dir: &Path, files: &[ModelFile]) -> Result<(), SemanticError> {
    for file in files {
        let path = dir.join(file.path);
        let actual = file_sha256(&path).map_err(|e| {
            unavailable(format!("{} ({MODEL_DIR_ENV}): {e}", path.display()))
        })?;
        if actual != file.sha256 {
            return Err(unavailable(format!("{} ({MODEL_DIR_ENV})", mismatch(&path, file, &actual))));
        }
    }
    Ok(())
}

/// Make sure every file of `files` is in the cache directory `dir` with its
/// pinned checksum. A missing file is fetched into a temporary file, verified,
/// and only then renamed into place. A cached or downloaded file whose
/// checksum differs is deleted and reported by name: it is never used.
pub fn ensure_cached_model(dir: &Path, files: &[ModelFile], fetch: Fetch<'_>) -> Result<(), SemanticError> {
    for file in files {
        let path = dir.join(file.path);
        if path.exists() {
            let actual = file_sha256(&path).map_err(|e| unavailable(format!("{}: {e}", path.display())))?;
            if actual == file.sha256 {
                continue;
            }
            let _ = std::fs::remove_file(&path);
            return Err(unavailable(format!(
                "{}; the file was deleted, run the command again to download it",
                mismatch(&path, file, &actual)
            )));
        }

        let parent = path.parent().unwrap_or(dir);
        std::fs::create_dir_all(parent).map_err(|e| unavailable(format!("{}: {e}", parent.display())))?;
        let mut tmp = tempfile::Builder::new()
            .prefix(crate::fsio::TEMP_PREFIX)
            .tempfile_in(parent)
            .map_err(|e| unavailable(format!("{}: {e}", parent.display())))?;
        fetch(file, tmp.as_file_mut())
            .map_err(|e| unavailable(format!("cannot download {}: {e}", file.path)))?;
        tmp.as_file()
            .sync_data()
            .map_err(|e| unavailable(format!("{}: {e}", tmp.path().display())))?;
        let actual = file_sha256(tmp.path()).map_err(|e| unavailable(format!("{}: {e}", tmp.path().display())))?;
        if actual != file.sha256 {
            // Dropping the temporary file deletes it.
            drop(tmp);
            return Err(unavailable(format!(
                "downloaded {}; the file was deleted",
                mismatch(&path, file, &actual)
            )));
        }
        tmp.persist(&path)
            .map_err(|e| unavailable(format!("{}: {}", path.display(), e.error)))?;
    }
    Ok(())
}

/// The directory holding the verified model files: [`MODEL_DIR_ENV`] when
/// `env_dir` is set (verified, never downloaded), otherwise the cache under
/// `cache_dir`, filled through `fetch` on first use.
pub fn resolve_model_dir(
    cache_dir: &Path,
    env_dir: Option<PathBuf>,
    files: &[ModelFile],
    fetch: Fetch<'_>,
) -> Result<PathBuf, SemanticError> {
    match env_dir {
        Some(dir) => {
            verify_model_dir(&dir, files)?;
            Ok(dir)
        }
        None => {
            let dir = cached_model_dir(cache_dir);
            ensure_cached_model(&dir, files, fetch)?;
            Ok(dir)
        }
    }
}

/// Download `file` at the pinned revision into `out`, with progress on stderr.
#[cfg(feature = "embeddings")]
pub fn http_fetch(file: &ModelFile, out: &mut File) -> Result<(), String> {
    use std::io::Write;
    use std::time::Duration;

    let url = model_file_url(file);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(30)))
        .build()
        .into();
    let response = agent.get(&url).call().map_err(|e| format!("{url}: {e}"))?;
    let total = response.body().content_length();
    let mut reader = response.into_body().into_reader();

    let mut buffer = vec![0u8; 1 << 16];
    let mut done: u64 = 0;
    let mut reported = 0;
    eprintln!("Downloading embedding model file {} …", file.path);
    loop {
        let n = reader.read(&mut buffer).map_err(|e| format!("{url}: {e}"))?;
        if n == 0 {
            break;
        }
        out.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        if let Some(total) = total.filter(|t| *t > 0) {
            let percent = done * 100 / total;
            if percent >= reported + 10 {
                reported = percent - percent % 10;
                eprintln!("  {}: {percent}% of {:.1} MB", file.path, total as f64 / 1e6);
            }
        }
    }
    out.flush().map_err(|e| e.to_string())?;
    eprintln!("  {}: done ({:.1} MB)", file.path, done as f64 / 1e6);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksums_are_lowercase_sha256_hex() {
        for file in MODEL_FILES {
            assert_eq!(file.sha256.len(), 64, "{}", file.path);
            assert!(
                file.sha256.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
                "{}",
                file.path
            );
        }
    }

    #[test]
    fn url_names_pinned_commit() {
        assert_eq!(
            model_file_url(&MODEL_ONNX),
            "https://huggingface.co/intfloat/multilingual-e5-small/resolve/\
             614241f622f53c4eeff9890bdc4f31cfecc418b3/onnx/model.onnx"
        );
    }

    use std::cell::RefCell;
    use std::io::Write;

    /// A model file with these contents: the pinned checksum is theirs.
    fn fake_file(path: &'static str, contents: &str) -> ModelFile {
        let sha: String = Sha256::digest(contents.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
        ModelFile { path, sha256: Box::leak(sha.into_boxed_str()) }
    }

    /// Lists every file in `dir`, recursively, as relative paths.
    fn files_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = walkdir::WalkDir::new(dir)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().strip_prefix(dir).unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn first_use_downloads_verifies_and_keeps_the_files() {
        let tmp = tempfile::tempdir().unwrap();
        let files = [fake_file("onnx/model.onnx", "weights 137"), fake_file("onnx/tokenizer.json", "{}")];
        let fetched = RefCell::new(Vec::new());
        let fetch = |file: &ModelFile, out: &mut File| {
            fetched.borrow_mut().push(file.path);
            let body = if file.path.ends_with("onnx") { "weights 137" } else { "{}" };
            out.write_all(body.as_bytes()).map_err(|e| e.to_string())
        };

        let dir = resolve_model_dir(tmp.path(), None, &files, &fetch).unwrap();

        assert_eq!(dir, tmp.path().join("models").join(MODEL_COMMIT));
        assert_eq!(files_in(&dir), ["onnx/model.onnx", "onnx/tokenizer.json"]);
        assert_eq!(*fetched.borrow(), ["onnx/model.onnx", "onnx/tokenizer.json"]);

        // Later runs reuse the cache: no download at all.
        let no_fetch = |file: &ModelFile, _: &mut File| -> Result<(), String> {
            panic!("downloaded {} although it is cached", file.path)
        };
        resolve_model_dir(tmp.path(), None, &files, &no_fetch).unwrap();
    }

    #[test]
    fn tampered_download_is_deleted_and_named() {
        let tmp = tempfile::tempdir().unwrap();
        let files = [fake_file("onnx/model.onnx", "weights 137")];
        let fetch = |_: &ModelFile, out: &mut File| out.write_all(b"tampered").map_err(|e| e.to_string());

        let err = resolve_model_dir(tmp.path(), None, &files, &fetch).unwrap_err();

        let SemanticError::ModelUnavailable(message) = &err else { panic!("{err:?}") };
        assert!(message.contains("onnx/model.onnx"), "{message}");
        assert!(message.contains("checksum mismatch"), "{message}");
        assert!(err.to_string().contains("embedding model unavailable"), "{err}");
        assert!(files_in(&cached_model_dir(tmp.path())).is_empty(), "the bad download was kept");
    }

    #[test]
    fn tampered_cached_file_is_deleted_and_named() {
        let tmp = tempfile::tempdir().unwrap();
        let files = [fake_file("onnx/model.onnx", "weights 137")];
        let path = cached_model_dir(tmp.path()).join("onnx/model.onnx");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "corrupted").unwrap();
        let no_fetch = |_: &ModelFile, _: &mut File| -> Result<(), String> { panic!("must not download") };

        let err = resolve_model_dir(tmp.path(), None, &files, &no_fetch).unwrap_err();

        assert!(err.to_string().contains("onnx/model.onnx"), "{err}");
        assert!(err.to_string().contains("checksum mismatch"), "{err}");
        assert!(!path.exists(), "a mismatching cached file must be deleted");
    }

    #[test]
    fn download_failure_says_the_model_is_unavailable() {
        let tmp = tempfile::tempdir().unwrap();
        let files = [fake_file("onnx/model.onnx", "weights 137")];
        let offline = |_: &ModelFile, _: &mut File| -> Result<(), String> { Err("network unreachable".into()) };

        let err = resolve_model_dir(tmp.path(), None, &files, &offline).unwrap_err();

        assert!(err.to_string().contains("embedding model unavailable"), "{err}");
        assert!(err.to_string().contains("network unreachable"), "{err}");
        assert!(files_in(&cached_model_dir(tmp.path())).is_empty());
    }

    #[test]
    fn model_dir_with_wrong_files_fails_without_downloading() {
        let tmp = tempfile::tempdir().unwrap();
        let provided = tmp.path().join("provided");
        std::fs::create_dir_all(provided.join("onnx")).unwrap();
        std::fs::write(provided.join("onnx/model.onnx"), "weights 137").unwrap();
        std::fs::write(provided.join("onnx/tokenizer.json"), "not the pinned tokenizer").unwrap();
        let files = [fake_file("onnx/model.onnx", "weights 137"), fake_file("onnx/tokenizer.json", "{}")];
        let no_fetch = |_: &ModelFile, _: &mut File| -> Result<(), String> { panic!("must not download") };

        let err = resolve_model_dir(tmp.path(), Some(provided.clone()), &files, &no_fetch).unwrap_err();

        assert!(err.to_string().contains("onnx/tokenizer.json"), "{err}");
        assert!(err.to_string().contains("checksum mismatch"), "{err}");
        assert!(err.to_string().contains(MODEL_DIR_ENV), "{err}");
        // The user's directory is left alone, and no cache was created.
        assert!(provided.join("onnx/tokenizer.json").exists());
        assert!(!tmp.path().join("models").exists());

        // A matching directory is used as it is.
        std::fs::write(provided.join("onnx/tokenizer.json"), "{}").unwrap();
        assert_eq!(resolve_model_dir(tmp.path(), Some(provided.clone()), &files, &no_fetch).unwrap(), provided);
    }

    /// Downloads the small pinned files from Hugging Face over the network
    /// and verifies them.
    #[cfg(feature = "embeddings")]
    #[test]
    #[ignore = "needs network access to huggingface.co"]
    fn real_download_of_the_small_files() {
        let tmp = tempfile::tempdir().unwrap();
        let files = [MODEL_CONFIG, MODEL_SPECIAL_TOKENS_MAP, MODEL_TOKENIZER_CONFIG];
        let dir = resolve_model_dir(tmp.path(), None, &files, &http_fetch).unwrap();
        assert_eq!(files_in(&dir), ["onnx/config.json", "onnx/special_tokens_map.json", "onnx/tokenizer_config.json"]);
    }

    /// Loads the real model from `CERBO_MODEL_DIR` (needs ONNX Runtime through
    /// `ORT_DYLIB_PATH`) and checks Russian and English match by meaning.
    #[cfg(feature = "embeddings")]
    #[test]
    #[ignore = "needs the model files in CERBO_MODEL_DIR and ONNX Runtime"]
    fn real_model_matches_across_languages() {
        use fastembed::{
            InitOptionsUserDefined, Pooling, TextEmbedding, TokenizerFiles,
            UserDefinedEmbeddingModel,
        };
        use sha2::{Digest, Sha256};

        let dir = std::path::PathBuf::from(
            std::env::var("CERBO_MODEL_DIR").expect("CERBO_MODEL_DIR is not set"),
        );
        let read = |file: ModelFile| {
            let bytes = std::fs::read(dir.join(file.path)).unwrap();
            let digest: String = Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect();
            assert_eq!(digest, file.sha256, "{}", file.path);
            bytes
        };
        let tokenizer_files = TokenizerFiles {
            tokenizer_file: read(MODEL_TOKENIZER),
            config_file: read(MODEL_CONFIG),
            special_tokens_map_file: read(MODEL_SPECIAL_TOKENS_MAP),
            tokenizer_config_file: read(MODEL_TOKENIZER_CONFIG),
        };
        let model = UserDefinedEmbeddingModel::new(read(MODEL_ONNX), tokenizer_files)
            .with_pooling(Pooling::Mean);
        let options = InitOptionsUserDefined::new().with_max_length(MODEL_MAX_TOKENS);
        let mut embedder = TextEmbedding::try_new_from_user_defined(model, options).unwrap();

        let vectors = embedder
            .embed(["query: поездка в Берлин", "query: trip to Berlin"], None)
            .unwrap();
        assert_eq!(vectors[0].len(), MODEL_DIM);
        let norm = |v: &[f32]| v.iter().map(|x| x * x).sum::<f32>().sqrt();
        let dot: f32 = vectors[0].iter().zip(&vectors[1]).map(|(a, b)| a * b).sum();
        let cosine = dot / (norm(&vectors[0]) * norm(&vectors[1]));
        eprintln!("cosine(поездка в Берлин, trip to Berlin) = {cosine}");
        assert!(cosine > 0.8, "cosine {cosine}");
    }
}
