use crate::error::CoreError;
use sqlx::{FromRow, Pool, Sqlite};

#[derive(Debug, Clone, FromRow)]
pub struct AiMemoryRow {
    pub id: i64,
    pub kind: String,
    pub source_ref: String,
    pub content: String,
    pub embedding: Option<Vec<u8>>,
    pub model: Option<String>,
    pub created_at: String,
}

pub struct MemoryRepo<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> MemoryRepo<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn upsert(
        &self,
        kind: &str,
        source_ref: &str,
        content: &str,
        embedding: &[f32],
        model: &str,
    ) -> Result<(), CoreError> {
        let bytes = encode_embedding(embedding);
        sqlx::query(
            "INSERT INTO ai_memories (kind, source_ref, content, embedding, model)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(kind, source_ref) DO UPDATE SET
               content = excluded.content,
               embedding = excluded.embedding,
               model = excluded.model",
        )
        .bind(kind)
        .bind(source_ref)
        .bind(content)
        .bind(&bytes)
        .bind(model)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn existing_source_refs(&self, kind: &str) -> Result<Vec<String>, CoreError> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT source_ref FROM ai_memories WHERE kind = ?")
                .bind(kind)
                .fetch_all(self.pool)
                .await?;
        Ok(rows.into_iter().map(|(s,)| s).collect())
    }

    pub async fn all_with_embeddings(&self) -> Result<Vec<(AiMemoryRow, Vec<f32>)>, CoreError> {
        let rows = sqlx::query_as::<_, AiMemoryRow>(
            "SELECT id, kind, source_ref, content, embedding, model, created_at
             FROM ai_memories WHERE embedding IS NOT NULL",
        )
        .fetch_all(self.pool)
        .await?;
        let out = rows
            .into_iter()
            .filter_map(|r| {
                let bytes = r.embedding.clone()?;
                let vec = decode_embedding(&bytes);
                Some((r, vec))
            })
            .collect();
        Ok(out)
    }
}

pub fn encode_embedding(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for f in v {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out
}

pub fn decode_embedding(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// Cosine similarity over equal-length vectors. Returns 0 for mismatched
/// lengths or zero-magnitude vectors (rather than NaN).
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0f32;
    let mut na = 0f32;
    let mut nb = 0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub source_ref: String,
    pub kind: String,
    pub content: String,
    pub score: f32,
}

/// Top-k retrieval by cosine similarity. Pure function — testable without a DB.
pub fn top_k(indexed: &[(AiMemoryRow, Vec<f32>)], query: &[f32], k: usize) -> Vec<SearchHit> {
    let mut scored: Vec<SearchHit> = indexed
        .iter()
        .map(|(row, emb)| SearchHit {
            source_ref: row.source_ref.clone(),
            kind: row.kind.clone(),
            content: row.content.clone(),
            score: cosine_similarity(query, emb),
        })
        .collect();
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(k);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_roundtrip() {
        let v = vec![0.1, -0.2, 7.5, -42.0, 0.0];
        let bytes = encode_embedding(&v);
        let back = decode_embedding(&bytes);
        assert_eq!(v.len(), back.len());
        for (a, b) in v.iter().zip(back.iter()) {
            assert!((a - b).abs() < 1e-6, "{a} != {b}");
        }
    }

    #[test]
    fn cosine_of_identical_vectors_is_one() {
        let v = vec![1.0, 2.0, 3.0];
        let s = cosine_similarity(&v, &v);
        assert!((s - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_of_orthogonal_vectors_is_zero() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn cosine_of_zero_vector_is_zero_not_nan() {
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 0.0];
        let s = cosine_similarity(&a, &b);
        assert!(s.is_finite());
        assert_eq!(s, 0.0);
    }

    #[test]
    fn top_k_ranks_by_similarity() {
        let q = vec![1.0, 0.0, 0.0];
        let memories = vec![
            (
                row("a", &[0.0, 1.0, 0.0]), // orthogonal
                vec![0.0, 1.0, 0.0],
            ),
            (
                row("b", &[0.9, 0.1, 0.0]), // close to q
                vec![0.9, 0.1, 0.0],
            ),
            (
                row("c", &[1.0, 0.0, 0.0]), // identical
                vec![1.0, 0.0, 0.0],
            ),
        ];
        let hits = top_k(&memories, &q, 2);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].source_ref, "c");
        assert_eq!(hits[1].source_ref, "b");
    }

    fn row(source_ref: &str, _emb: &[f32]) -> AiMemoryRow {
        AiMemoryRow {
            id: 0,
            kind: "journal".into(),
            source_ref: source_ref.into(),
            content: format!("content {source_ref}"),
            embedding: None,
            model: None,
            created_at: "2026-06-05T00:00:00.000Z".into(),
        }
    }
}
