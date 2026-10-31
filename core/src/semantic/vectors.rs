//! Page vectors and exact nearest-neighbour search (design D4).
//!
//! The flat store keeps one unit-length `f32` row per page and scans all of
//! them for a query: at 384 dimensions that is fast enough for personal vaults
//! and has no recall trade-off. An approximate index can replace it behind
//! [`VectorIndex`] once a vault outgrows the scan.

use super::embed::normalise;
use super::SemanticError;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io;
use std::path::Path;

/// Page vectors keyed by object UUID.
pub trait VectorIndex {
    /// Width of every vector.
    fn dim(&self) -> usize;

    /// Number of pages with a vector.
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The stored, unit-length vector of `uuid`.
    fn get(&self, uuid: &str) -> Option<&[f32]>;

    /// Every UUID with a vector, in no particular order.
    fn uuids(&self) -> Vec<String>;

    /// Store `vector` (normalised here) for `uuid`, replacing any previous one.
    fn upsert(&mut self, uuid: &str, vector: &[f32]);

    /// Drop the vector of `uuid`; true if there was one.
    fn remove(&mut self, uuid: &str) -> bool;

    /// The `k` pages most similar to `query` by cosine, best first, ties by
    /// UUID; `exclude` is left out.
    fn top_k(&self, query: &[f32], k: usize, exclude: Option<&str>) -> Vec<(String, f32)>;

    /// Every page scoring at least `min_score` against `query`, best first,
    /// ties by UUID; `exclude` is left out.
    fn above(&self, query: &[f32], min_score: f32, exclude: Option<&str>) -> Vec<(String, f32)>;
}

/// Header of `vectors-<gen>.bin`, with its format version.
const MAGIC: &[u8; 4] = b"CVE1";

/// [`VectorIndex`] as a flat row-major matrix plus a UUID table, saved to
/// `vectors-<gen>.bin`: the header, the UUIDs (length-prefixed), then the rows
/// as little-endian `f32`.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatVectors {
    dim: usize,
    uuids: Vec<String>,
    rows: Vec<f32>,
    positions: HashMap<String, usize>,
}

impl FlatVectors {
    pub fn new(dim: usize) -> Self {
        Self { dim, uuids: Vec::new(), rows: Vec::new(), positions: HashMap::new() }
    }

    fn row(&self, i: usize) -> &[f32] {
        &self.rows[i * self.dim..(i + 1) * self.dim]
    }

    /// Read `path`. A missing file is `None`; a damaged one is an error.
    pub fn load(path: &Path) -> Result<Option<Self>, SemanticError> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(SemanticError::Io(format!("{}: {e}", path.display()))),
        };
        decode(&bytes)
            .map(Some)
            .ok_or_else(|| SemanticError::Io(format!("{}: damaged vector file", path.display())))
    }

    /// Replace `path` with the durable atomic write.
    pub fn save(&self, path: &Path) -> Result<(), SemanticError> {
        crate::fsio::write_atomic(path, &self.encode())
            .map_err(|e| SemanticError::Io(format!("{}: {e}", path.display())))
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(16 + self.uuids.len() * 37 + self.rows.len() * 4);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&(self.dim as u32).to_le_bytes());
        out.extend_from_slice(&(self.uuids.len() as u64).to_le_bytes());
        for uuid in &self.uuids {
            out.push(uuid.len() as u8);
            out.extend_from_slice(uuid.as_bytes());
        }
        for x in &self.rows {
            out.extend_from_slice(&x.to_le_bytes());
        }
        out
    }
}

fn decode(bytes: &[u8]) -> Option<FlatVectors> {
    let rest = bytes.strip_prefix(MAGIC)?;
    let (dim, rest) = rest.split_first_chunk::<4>()?;
    let (count, mut rest) = rest.split_first_chunk::<8>()?;
    let dim = u32::from_le_bytes(*dim) as usize;
    let count = usize::try_from(u64::from_le_bytes(*count)).ok()?;

    let mut vectors = FlatVectors::new(dim);
    for i in 0..count {
        let (&len, tail) = rest.split_first()?;
        let (uuid, tail) = tail.split_at_checked(len as usize)?;
        let uuid = std::str::from_utf8(uuid).ok()?.to_string();
        vectors.positions.insert(uuid.clone(), i);
        vectors.uuids.push(uuid);
        rest = tail;
    }
    if rest.len() != count.checked_mul(dim)?.checked_mul(4)? {
        return None;
    }
    vectors.rows = rest
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes(c.try_into().expect("chunks of 4")))
        .collect();
    Some(vectors)
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Best score first, then UUID, so equal scores come out in a stable order.
fn by_rank(a: &(String, f32), b: &(String, f32)) -> Ordering {
    b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal).then_with(|| a.0.cmp(&b.0))
}

impl VectorIndex for FlatVectors {
    fn dim(&self) -> usize {
        self.dim
    }

    fn len(&self) -> usize {
        self.uuids.len()
    }

    fn get(&self, uuid: &str) -> Option<&[f32]> {
        self.positions.get(uuid).map(|&i| self.row(i))
    }

    fn uuids(&self) -> Vec<String> {
        self.uuids.clone()
    }

    fn upsert(&mut self, uuid: &str, vector: &[f32]) {
        assert_eq!(vector.len(), self.dim, "vector width");
        let mut vector = vector.to_vec();
        normalise(&mut vector);
        match self.positions.get(uuid) {
            Some(&i) => self.rows[i * self.dim..(i + 1) * self.dim].copy_from_slice(&vector),
            None => {
                self.positions.insert(uuid.to_string(), self.uuids.len());
                self.uuids.push(uuid.to_string());
                self.rows.extend_from_slice(&vector);
            }
        }
    }

    fn remove(&mut self, uuid: &str) -> bool {
        let Some(i) = self.positions.remove(uuid) else {
            return false;
        };
        // Move the last row into the hole.
        let last = self.uuids.len() - 1;
        if i != last {
            let (head, tail) = self.rows.split_at_mut(last * self.dim);
            head[i * self.dim..(i + 1) * self.dim].copy_from_slice(&tail[..self.dim]);
            self.uuids.swap(i, last);
            self.positions.insert(self.uuids[i].clone(), i);
        }
        self.uuids.pop();
        self.rows.truncate(last * self.dim);
        true
    }

    fn top_k(&self, query: &[f32], k: usize, exclude: Option<&str>) -> Vec<(String, f32)> {
        let mut scored: Vec<(String, f32)> = self
            .uuids
            .iter()
            .enumerate()
            .filter(|(_, uuid)| Some(uuid.as_str()) != exclude)
            .map(|(i, uuid)| (uuid.clone(), dot(query, self.row(i))))
            .collect();
        if k < scored.len() {
            scored.select_nth_unstable_by(k, by_rank);
            scored.truncate(k);
        }
        scored.sort_by(by_rank);
        scored
    }

    fn above(&self, query: &[f32], min_score: f32, exclude: Option<&str>) -> Vec<(String, f32)> {
        let mut scored: Vec<(String, f32)> = self
            .uuids
            .iter()
            .enumerate()
            .filter(|(_, uuid)| Some(uuid.as_str()) != exclude)
            .filter_map(|(i, uuid)| {
                let score = dot(query, self.row(i));
                (score >= min_score).then(|| (uuid.clone(), score))
            })
            .collect();
        scored.sort_by(by_rank);
        scored
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "a0000000-0000-4000-8000-000000000137";
    const B: &str = "b0000000-0000-4000-8000-000000000137";
    const C: &str = "c0000000-0000-4000-8000-000000000137";
    const D: &str = "d0000000-0000-4000-8000-000000000137";

    fn sample() -> FlatVectors {
        let mut v = FlatVectors::new(3);
        v.upsert(A, &[1.0, 0.0, 0.0]);
        v.upsert(B, &[0.6, 0.8, 0.0]);
        v.upsert(C, &[0.0, 1.0, 0.0]);
        // Not unit length: stored normalised, so it ties with A.
        v.upsert(D, &[137.0, 0.0, 0.0]);
        v
    }

    fn ids(hits: &[(String, f32)]) -> Vec<&str> {
        hits.iter().map(|(u, _)| u.as_str()).collect()
    }

    #[test]
    fn top_k_is_exact_and_ordered_by_score_then_uuid() {
        let v = sample();
        let hits = v.top_k(&[1.0, 0.0, 0.0], 10, None);

        assert_eq!(ids(&hits), [A, D, B, C]);
        assert!((hits[0].1 - 1.0).abs() < 1e-6 && (hits[1].1 - 1.0).abs() < 1e-6);
        assert!((hits[2].1 - 0.6).abs() < 1e-6);
        assert!(hits[3].1.abs() < 1e-6);

        assert_eq!(ids(&v.top_k(&[1.0, 0.0, 0.0], 2, None)), [A, D]);
        assert_eq!(ids(&v.top_k(&[0.0, 1.0, 0.0], 2, Some(C))), [B, A]);
        assert!(v.top_k(&[1.0, 0.0, 0.0], 0, None).is_empty());

        assert_eq!(ids(&v.above(&[1.0, 0.0, 0.0], 0.5, Some(D))), [A, B]);
        assert!(v.above(&[0.0, 0.0, 1.0], 0.5, None).is_empty());
    }

    #[test]
    fn delete_removes_only_that_vector() {
        let mut v = sample();
        assert!(v.remove(A));
        assert!(!v.remove(A));

        assert_eq!(v.len(), 3);
        assert_eq!(v.get(A), None);
        assert_eq!(v.get(D), Some(&[1.0f32, 0.0, 0.0][..]), "the moved row must keep its vector");
        assert_eq!(ids(&v.top_k(&[1.0, 0.0, 0.0], 10, None)), [D, B, C]);

        // Removing the last row and replacing a row also keep the table right.
        assert!(v.remove(D));
        v.upsert(B, &[0.0, 0.0, 1.0]);
        assert_eq!(ids(&v.top_k(&[0.0, 0.0, 1.0], 10, None)), [B, C]);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn reload_gives_the_same_vectors_and_answers() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("vectors-137.bin");
        assert_eq!(FlatVectors::load(&path).unwrap(), None);

        let mut v = sample();
        v.remove(B);
        v.save(&path).unwrap();
        let loaded = FlatVectors::load(&path).unwrap().unwrap();

        assert_eq!(loaded.dim(), 3);
        let mut uuids = loaded.uuids();
        uuids.sort();
        assert_eq!(uuids, [A, C, D]);
        for uuid in [A, C, D] {
            assert_eq!(loaded.get(uuid), v.get(uuid));
        }
        let query = [0.3, 0.7, 0.1];
        assert_eq!(loaded.top_k(&query, 10, None), v.top_k(&query, 10, None));

        std::fs::write(&path, &v.encode()[..20]).unwrap();
        assert!(FlatVectors::load(&path).is_err(), "a truncated file must not load");
    }
}
