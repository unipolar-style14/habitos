-- Add embedding storage to ai_memories.
-- For V1 we store embeddings as raw little-endian f32 bytes and compute cosine
-- similarity in Rust. This is fine up to a few thousand entries. When scale
-- exceeds that, swap in sqlite-vec (extension load + vec0 virtual table) —
-- the search function in memory.rs is the only thing that needs to change.

ALTER TABLE ai_memories ADD COLUMN embedding BLOB;
ALTER TABLE ai_memories ADD COLUMN model TEXT;

-- Lookup by source ref is hot during backfill ("does this entry already have an embedding?").
CREATE UNIQUE INDEX ai_memories_kind_source ON ai_memories (kind, source_ref);
