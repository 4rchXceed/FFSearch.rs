//! I DIDN'T WRITE THIS FILE, IT'S FROM:
//! - https://github.com/0xPlaygrounds/rig/blob/main/crates/rig-lancedb/examples/fixtures/lib.rs
//! LICENSE:
//! MIT License
//! See THIRD_PARTY_LICENSES.md for more information.
//! Note: The code has been modified to fit the context of this project, but the original code is from the above source.
use std::sync::Arc;

use arrow_array::{ArrayRef, FixedSizeListArray, RecordBatch, StringArray, types::Float64Type};
use lancedb::arrow::arrow_schema::ArrowError;
use rig_core::embeddings::Embedding;

use crate::ai::rig::embed_document::EmbedDocument;

// Convert Word objects and their embedding to a RecordBatch.
pub fn as_record_batch(
    records: Vec<(EmbedDocument, Vec<Embedding>)>,
    dims: usize,
) -> Result<RecordBatch, ArrowError> {
    let id = StringArray::from_iter_values(records.iter().map(|(d, _)| d.path.clone()));

    let definition = StringArray::from_iter_values(records.iter().map(|(d, _)| d.content.clone()));

    let embedding = FixedSizeListArray::from_iter_primitive::<Float64Type, _, _>(
        records.into_iter().map(|(_, embeddings)| {
            embeddings
                .into_iter()
                .next()
                .map(|embedding| embedding.vec.into_iter().map(Some).collect::<Vec<_>>())
        }),
        dims as i32,
    );

    return RecordBatch::try_from_iter(vec![
        ("id", Arc::new(id) as ArrayRef),
        ("definition", Arc::new(definition) as ArrayRef),
        ("embedding", Arc::new(embedding) as ArrayRef),
    ]);
}
