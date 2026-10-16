use std::sync::Arc;

use arrow_array::{
    FixedSizeListArray, RecordBatch, RecordBatchIterator, StringArray, cast::AsArray,
    types::Float32Type,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use capitainerie_common::{CardId, CoreError, OrganizationId};
use capitainerie_core::domain::{
    card::Column,
    similarity::{CardVector, Embedding, SimilarCard, ports::VectorStore},
};
use futures_util::TryStreamExt;
use lancedb::{
    DistanceType, Table, connect,
    query::{ExecutableQuery, QueryBase},
};

const TABLE: &str = "cards";
const DISTANCE: &str = "_distance";

#[derive(Debug, thiserror::Error)]
pub enum VectorError {
    #[error("cannot open the vector store at {uri}: {reason}")]
    Open { uri: String, reason: String },
    #[error(
        "the vector table at {uri} holds {found}-dimensional vectors, not {expected}: use another LANCEDB_URI or delete it to re-index"
    )]
    Dimensions {
        uri: String,
        found: usize,
        expected: usize,
    },
}

impl From<VectorError> for CoreError {
    fn from(error: VectorError) -> Self {
        CoreError::Internal(error.to_string())
    }
}

fn internal(error: impl std::fmt::Display) -> CoreError {
    CoreError::Internal(format!("vector store: {error}"))
}

fn schema(dimensions: usize) -> Result<SchemaRef, CoreError> {
    let dimensions = i32::try_from(dimensions).map_err(internal)?;
    Ok(Arc::new(Schema::new(vec![
        Field::new("card_id", DataType::Utf8, false),
        Field::new("organization_id", DataType::Utf8, false),
        Field::new("text", DataType::Utf8, false),
        Field::new("card_column", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                dimensions,
            ),
            false,
        ),
    ])))
}

/// Card embeddings in a LanceDB table: a local directory in development,
/// an object store later.
#[derive(Clone)]
pub struct LanceVectorStore {
    table: Table,
    dimensions: usize,
}

impl LanceVectorStore {
    pub async fn open(uri: &str, dimensions: usize) -> Result<Self, VectorError> {
        let open_error = |error: lancedb::Error| VectorError::Open {
            uri: uri.to_owned(),
            reason: error.to_string(),
        };
        let connection = connect(uri).execute().await.map_err(open_error)?;
        let existing = connection
            .table_names()
            .execute()
            .await
            .map_err(open_error)?;
        let table = if existing.iter().any(|name| name == TABLE) {
            let table = connection
                .open_table(TABLE)
                .execute()
                .await
                .map_err(open_error)?;
            let stored = stored_dimensions(&table).await.map_err(open_error)?;
            if stored != Some(dimensions) {
                return Err(VectorError::Dimensions {
                    uri: uri.to_owned(),
                    found: stored.unwrap_or(0),
                    expected: dimensions,
                });
            }
            table
        } else {
            let schema = schema(dimensions).map_err(|error| VectorError::Open {
                uri: uri.to_owned(),
                reason: error.to_string(),
            })?;
            connection
                .create_empty_table(TABLE, schema)
                .execute()
                .await
                .map_err(open_error)?
        };
        Ok(Self { table, dimensions })
    }

    /// Whether nothing is indexed yet: the signal to backfill.
    pub async fn is_empty(&self) -> Result<bool, CoreError> {
        let rows = self.table.count_rows(None).await.map_err(internal)?;
        Ok(rows == 0)
    }

    fn check(&self, embedding: &Embedding) -> Result<(), CoreError> {
        if embedding.0.len() != self.dimensions {
            return Err(CoreError::Invalid(format!(
                "the embedding has {} dimensions, the store expects {}",
                embedding.0.len(),
                self.dimensions
            )));
        }
        Ok(())
    }
}

async fn stored_dimensions(table: &Table) -> Result<Option<usize>, lancedb::Error> {
    let schema = table.schema().await?;
    Ok(schema
        .field_with_name("vector")
        .ok()
        .and_then(|field| match field.data_type() {
            DataType::FixedSizeList(_, size) => usize::try_from(*size).ok(),
            _ => None,
        }))
}

impl VectorStore for LanceVectorStore {
    async fn upsert(&self, card: CardVector) -> Result<(), CoreError> {
        self.check(&card.embedding)?;
        let dimensions = i32::try_from(self.dimensions).map_err(internal)?;
        let vectors = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
            [Some(
                card.embedding
                    .0
                    .iter()
                    .copied()
                    .map(Some)
                    .collect::<Vec<_>>(),
            )],
            dimensions,
        );
        let schema = schema(self.dimensions)?;
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(vec![card.card_id.0.to_string()])),
                Arc::new(StringArray::from(vec![card.organization_id.0.to_string()])),
                Arc::new(StringArray::from(vec![card.text])),
                Arc::new(StringArray::from(vec![card.column.as_str()])),
                Arc::new(vectors),
            ],
        )
        .map_err(internal)?;
        let mut merge = self.table.merge_insert(&["card_id"]);
        merge
            .when_matched_update_all(None)
            .when_not_matched_insert_all();
        merge
            .execute(Box::new(RecordBatchIterator::new(vec![Ok(batch)], schema)))
            .await
            .map(|_| ())
            .map_err(internal)
    }

    async fn nearest(
        &self,
        organization_id: OrganizationId,
        embedding: &Embedding,
        limit: usize,
    ) -> Result<Vec<SimilarCard>, CoreError> {
        self.check(embedding)?;
        let filter = format!(
            "organization_id = '{}' AND card_column != '{}'",
            organization_id.0,
            Column::Rejected.as_str()
        );
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .nearest_to(embedding.0.as_slice())
            .map_err(internal)?
            .distance_type(DistanceType::Cosine)
            .only_if(filter)
            .limit(limit)
            .execute()
            .await
            .map_err(internal)?
            .try_collect()
            .await
            .map_err(internal)?;
        let mut found = Vec::new();
        for batch in &batches {
            found.extend(similar_cards(batch)?);
        }
        found.sort_by(|a, b| b.score.total_cmp(&a.score));
        found.truncate(limit);
        Ok(found)
    }
}

fn similar_cards(batch: &RecordBatch) -> Result<Vec<SimilarCard>, CoreError> {
    let ids = batch
        .column_by_name("card_id")
        .and_then(|column| column.as_string_opt::<i32>())
        .ok_or_else(|| internal("the result has no card_id"))?;
    let texts = batch
        .column_by_name("text")
        .and_then(|column| column.as_string_opt::<i32>())
        .ok_or_else(|| internal("the result has no text"))?;
    let distances = batch
        .column_by_name(DISTANCE)
        .and_then(|column| column.as_primitive_opt::<Float32Type>())
        .ok_or_else(|| internal("the result has no distance"))?;
    (0..batch.num_rows())
        .map(|row| {
            let id = ids
                .value(row)
                .parse()
                .map(CardId)
                .map_err(|error| internal(format!("a stored card id is invalid: {error}")))?;
            Ok(SimilarCard {
                card_id: id,
                score: (1.0 - distances.value(row)).clamp(0.0, 1.0),
                text: texts.value(row).to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::PathBuf;

    use uuid::Uuid;

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("capitainerie-lance-{}", Uuid::now_v7()));
            Self(path)
        }

        fn uri(&self) -> &str {
            self.0.to_str().unwrap()
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn card(org: OrganizationId, text: &str, vector: [f32; 3], column: Column) -> CardVector {
        CardVector {
            card_id: CardId::new(),
            organization_id: org,
            text: text.into(),
            embedding: Embedding(vector.to_vec()),
            column,
        }
    }

    async fn store(dir: &TempDir) -> LanceVectorStore {
        LanceVectorStore::open(dir.uri(), 3).await.unwrap()
    }

    #[tokio::test]
    async fn results_come_best_score_first_with_a_cosine_score() {
        let dir = TempDir::new();
        let store = store(&dir).await;
        let org = OrganizationId::new();
        let same = card(org, "same", [1.0, 0.0, 0.0], Column::Proposed);
        let near = card(org, "near", [1.0, 1.0, 0.0], Column::Todo);
        let far = card(org, "far", [0.0, 0.0, 1.0], Column::Done);
        for c in [&far, &near, &same] {
            store.upsert((*c).clone()).await.unwrap();
        }

        let found = store
            .nearest(org, &Embedding(vec![2.0, 0.0, 0.0]), 3)
            .await
            .unwrap();

        let texts: Vec<_> = found.iter().map(|f| f.text.as_str()).collect();
        assert_eq!(texts, ["same", "near", "far"]);
        assert!((found[0].score - 1.0).abs() < 1e-4, "{}", found[0].score);
        assert!(
            (found[1].score - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-3,
            "{}",
            found[1].score
        );
        assert!(found[2].score < 1e-4);
        assert_eq!(found[0].card_id, same.card_id);
    }

    #[tokio::test]
    async fn the_limit_caps_the_results() {
        let dir = TempDir::new();
        let store = store(&dir).await;
        let org = OrganizationId::new();
        for i in 0..4 {
            store
                .upsert(card(org, "c", [1.0, i as f32, 0.0], Column::Proposed))
                .await
                .unwrap();
        }
        let found = store
            .nearest(org, &Embedding(vec![1.0, 0.0, 0.0]), 2)
            .await
            .unwrap();
        assert_eq!(found.len(), 2);
    }

    #[tokio::test]
    async fn upserting_a_card_again_replaces_its_vector() {
        let dir = TempDir::new();
        let store = store(&dir).await;
        let org = OrganizationId::new();
        let mut c = card(org, "before", [1.0, 0.0, 0.0], Column::Proposed);
        store.upsert(c.clone()).await.unwrap();
        c.text = "after".into();
        c.embedding = Embedding(vec![0.0, 1.0, 0.0]);
        store.upsert(c.clone()).await.unwrap();

        let found = store
            .nearest(org, &Embedding(vec![0.0, 1.0, 0.0]), 5)
            .await
            .unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "after");
        assert!((found[0].score - 1.0).abs() < 1e-4);
    }

    #[tokio::test]
    async fn another_organization_never_sees_the_card() {
        let dir = TempDir::new();
        let store = store(&dir).await;
        let (mine, theirs) = (OrganizationId::new(), OrganizationId::new());
        store
            .upsert(card(mine, "mine", [1.0, 0.0, 0.0], Column::Proposed))
            .await
            .unwrap();
        store
            .upsert(card(theirs, "theirs", [1.0, 0.0, 0.0], Column::Proposed))
            .await
            .unwrap();

        let found = store
            .nearest(mine, &Embedding(vec![1.0, 0.0, 0.0]), 5)
            .await
            .unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "mine");
    }

    #[tokio::test]
    async fn a_rejected_card_is_indexed_but_not_returned() {
        let dir = TempDir::new();
        let store = store(&dir).await;
        let org = OrganizationId::new();
        let mut c = card(org, "idea", [1.0, 0.0, 0.0], Column::Proposed);
        store.upsert(c.clone()).await.unwrap();
        c.column = Column::Rejected;
        store.upsert(c).await.unwrap();

        assert!(!store.is_empty().await.unwrap());
        let found = store
            .nearest(org, &Embedding(vec![1.0, 0.0, 0.0]), 5)
            .await
            .unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn a_new_store_is_empty_and_reopening_keeps_the_data() {
        let dir = TempDir::new();
        let org = OrganizationId::new();
        {
            let store = store(&dir).await;
            assert!(store.is_empty().await.unwrap());
            store
                .upsert(card(org, "kept", [1.0, 0.0, 0.0], Column::Todo))
                .await
                .unwrap();
        }
        let reopened = store(&dir).await;
        let found = reopened
            .nearest(org, &Embedding(vec![1.0, 0.0, 0.0]), 1)
            .await
            .unwrap();
        assert_eq!(found[0].text, "kept");
    }

    #[tokio::test]
    async fn a_wrong_dimension_is_refused() {
        let dir = TempDir::new();
        let store = store(&dir).await;
        let error = store
            .nearest(OrganizationId::new(), &Embedding(vec![1.0, 0.0]), 1)
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)));
        let error = LanceVectorStore::open(dir.uri(), 4).await.err().unwrap();
        assert!(matches!(error, VectorError::Dimensions { found: 3, .. }));
    }
}
