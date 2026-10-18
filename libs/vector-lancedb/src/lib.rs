use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};

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
    Connection, DistanceType, Table, connect,
    query::{ExecutableQuery, QueryBase},
};

const TABLE_PREFIX: &str = "cards__";
const DISTANCE: &str = "_distance";

#[derive(Debug, thiserror::Error)]
pub enum VectorError {
    #[error("cannot open the vector store at {uri}: {reason}")]
    Open { uri: String, reason: String },
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

/// `cards__bge_m3` for `bge-m3`: one table per embedding model, because the
/// vectors of two models can neither be compared nor share a dimension.
pub fn table_name(embedding_model: &str) -> String {
    let mut name = String::from(TABLE_PREFIX);
    let mut separated = true;
    for character in embedding_model.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            name.push(character);
            separated = false;
        } else if !separated {
            name.push('_');
            separated = true;
        }
    }
    while name.ends_with('_') && name.len() > TABLE_PREFIX.len() {
        name.pop();
    }
    if name.len() == TABLE_PREFIX.len() {
        name.push_str("default");
    }
    name
}

#[derive(Clone)]
struct Indexed {
    table: Table,
    dimensions: usize,
}

/// Card embeddings in LanceDB tables, one per embedding model: a local
/// directory in development, an object store later.
#[derive(Clone)]
pub struct LanceVectorStore {
    connection: Connection,
    tables: Arc<Mutex<HashMap<String, Indexed>>>,
}

impl LanceVectorStore {
    pub async fn open(uri: &str) -> Result<Self, VectorError> {
        let connection = connect(uri)
            .execute()
            .await
            .map_err(|error| VectorError::Open {
                uri: uri.to_owned(),
                reason: error.to_string(),
            })?;
        Ok(Self {
            connection,
            tables: Arc::default(),
        })
    }

    fn cached(&self, name: &str) -> Option<Indexed> {
        self.tables
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(name)
            .cloned()
    }

    fn remember(&self, name: &str, indexed: &Indexed) {
        self.tables
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(name.to_owned(), indexed.clone());
    }

    async fn existing(&self, name: &str) -> Result<Option<Indexed>, CoreError> {
        if let Some(indexed) = self.cached(name) {
            return Ok(Some(indexed));
        }
        let names = self
            .connection
            .table_names()
            .execute()
            .await
            .map_err(internal)?;
        if !names.iter().any(|existing| existing == name) {
            return Ok(None);
        }
        let table = self
            .connection
            .open_table(name)
            .execute()
            .await
            .map_err(internal)?;
        let dimensions = stored_dimensions(&table)
            .await
            .map_err(internal)?
            .ok_or_else(|| internal(format!("table {name} has no vector column")))?;
        let indexed = Indexed { table, dimensions };
        self.remember(name, &indexed);
        Ok(Some(indexed))
    }

    async fn for_write(&self, name: &str, dimensions: usize) -> Result<Indexed, CoreError> {
        let indexed = match self.existing(name).await? {
            Some(indexed) => indexed,
            None => {
                let created = self
                    .connection
                    .create_empty_table(name, schema(dimensions)?)
                    .execute()
                    .await;
                match created {
                    Ok(table) => {
                        let indexed = Indexed { table, dimensions };
                        self.remember(name, &indexed);
                        indexed
                    }
                    Err(error) => self.existing(name).await?.ok_or_else(|| internal(error))?,
                }
            }
        };
        check(&indexed, dimensions)?;
        Ok(indexed)
    }
}

fn check(indexed: &Indexed, dimensions: usize) -> Result<(), CoreError> {
    if indexed.dimensions != dimensions {
        return Err(CoreError::Invalid(format!(
            "the embedding has {dimensions} dimensions, the index of this model expects {}",
            indexed.dimensions
        )));
    }
    Ok(())
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
    async fn upsert(&self, embedding_model: &str, card: CardVector) -> Result<(), CoreError> {
        let dimensions = card.embedding.0.len();
        let indexed = self
            .for_write(&table_name(embedding_model), dimensions)
            .await?;
        let width = i32::try_from(dimensions).map_err(internal)?;
        let vectors = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
            [Some(
                card.embedding
                    .0
                    .iter()
                    .copied()
                    .map(Some)
                    .collect::<Vec<_>>(),
            )],
            width,
        );
        let schema = schema(dimensions)?;
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
        let mut merge = indexed.table.merge_insert(&["card_id"]);
        merge
            .when_matched_update_all(None)
            .when_not_matched_insert_all();
        merge
            .execute(Box::new(RecordBatchIterator::new(vec![Ok(batch)], schema)))
            .await
            .map(|_| ())
            .map_err(internal)
    }

    async fn holds_cards(
        &self,
        embedding_model: &str,
        organization_id: OrganizationId,
    ) -> Result<bool, CoreError> {
        let Some(indexed) = self.existing(&table_name(embedding_model)).await? else {
            return Ok(false);
        };
        let rows = indexed
            .table
            .count_rows(Some(format!("organization_id = '{}'", organization_id.0)))
            .await
            .map_err(internal)?;
        Ok(rows > 0)
    }

    async fn nearest(
        &self,
        embedding_model: &str,
        organization_id: OrganizationId,
        embedding: &Embedding,
        limit: usize,
    ) -> Result<Vec<SimilarCard>, CoreError> {
        let Some(indexed) = self.existing(&table_name(embedding_model)).await? else {
            return Ok(Vec::new());
        };
        check(&indexed, embedding.0.len())?;
        let filter = format!(
            "organization_id = '{}' AND card_column != '{}'",
            organization_id.0,
            Column::Rejected.as_str()
        );
        let batches: Vec<RecordBatch> = indexed
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

    const MODEL: &str = "bge-m3";

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
        LanceVectorStore::open(dir.uri()).await.unwrap()
    }

    fn texts(found: &[SimilarCard]) -> Vec<&str> {
        found.iter().map(|f| f.text.as_str()).collect()
    }

    #[test]
    fn a_table_is_named_after_its_sanitized_embedding_model() {
        assert_eq!(table_name("bge-m3"), "cards__bge_m3");
        assert_eq!(
            table_name("nomic-embed-text:v1.5"),
            "cards__nomic_embed_text_v1_5"
        );
        assert_eq!(table_name("Mistral-Embed"), "cards__mistral_embed");
        assert_eq!(table_name("--"), "cards__default");
        assert_eq!(table_name("../etc"), "cards__etc");
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
            store.upsert(MODEL, (*c).clone()).await.unwrap();
        }

        let found = store
            .nearest(MODEL, org, &Embedding(vec![2.0, 0.0, 0.0]), 3)
            .await
            .unwrap();

        assert_eq!(texts(&found), ["same", "near", "far"]);
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
                .upsert(
                    MODEL,
                    card(org, "c", [1.0, i as f32, 0.0], Column::Proposed),
                )
                .await
                .unwrap();
        }
        let found = store
            .nearest(MODEL, org, &Embedding(vec![1.0, 0.0, 0.0]), 2)
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
        store.upsert(MODEL, c.clone()).await.unwrap();
        c.text = "after".into();
        c.embedding = Embedding(vec![0.0, 1.0, 0.0]);
        store.upsert(MODEL, c.clone()).await.unwrap();

        let found = store
            .nearest(MODEL, org, &Embedding(vec![0.0, 1.0, 0.0]), 5)
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
            .upsert(MODEL, card(mine, "mine", [1.0, 0.0, 0.0], Column::Proposed))
            .await
            .unwrap();
        store
            .upsert(
                MODEL,
                card(theirs, "theirs", [1.0, 0.0, 0.0], Column::Proposed),
            )
            .await
            .unwrap();

        let found = store
            .nearest(MODEL, mine, &Embedding(vec![1.0, 0.0, 0.0]), 5)
            .await
            .unwrap();

        assert_eq!(texts(&found), ["mine"]);
        assert!(store.holds_cards(MODEL, mine).await.unwrap());
        assert!(
            !store
                .holds_cards(MODEL, OrganizationId::new())
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn a_rejected_card_is_indexed_but_not_returned() {
        let dir = TempDir::new();
        let store = store(&dir).await;
        let org = OrganizationId::new();
        let mut c = card(org, "idea", [1.0, 0.0, 0.0], Column::Proposed);
        store.upsert(MODEL, c.clone()).await.unwrap();
        c.column = Column::Rejected;
        store.upsert(MODEL, c).await.unwrap();

        assert!(store.holds_cards(MODEL, org).await.unwrap());
        let found = store
            .nearest(MODEL, org, &Embedding(vec![1.0, 0.0, 0.0]), 5)
            .await
            .unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn reopening_keeps_the_data() {
        let dir = TempDir::new();
        let org = OrganizationId::new();
        {
            let store = store(&dir).await;
            assert!(!store.holds_cards(MODEL, org).await.unwrap());
            store
                .upsert(MODEL, card(org, "kept", [1.0, 0.0, 0.0], Column::Todo))
                .await
                .unwrap();
        }
        let reopened = store(&dir).await;
        assert!(reopened.holds_cards(MODEL, org).await.unwrap());
        let found = reopened
            .nearest(MODEL, org, &Embedding(vec![1.0, 0.0, 0.0]), 1)
            .await
            .unwrap();
        assert_eq!(found[0].text, "kept");
    }

    #[tokio::test]
    async fn each_embedding_model_has_its_own_index_and_dimension() {
        let dir = TempDir::new();
        let store = store(&dir).await;
        let org = OrganizationId::new();
        store
            .upsert("model-a", card(org, "in a", [1.0, 0.0, 0.0], Column::Todo))
            .await
            .unwrap();
        let in_b = CardVector {
            embedding: Embedding(vec![1.0, 0.0]),
            ..card(org, "in b", [0.0; 3], Column::Todo)
        };
        store.upsert("model-b", in_b).await.unwrap();

        let a = store
            .nearest("model-a", org, &Embedding(vec![1.0, 0.0, 0.0]), 5)
            .await
            .unwrap();
        let b = store
            .nearest("model-b", org, &Embedding(vec![1.0, 0.0]), 5)
            .await
            .unwrap();
        let unknown = store
            .nearest("model-c", org, &Embedding(vec![1.0]), 5)
            .await
            .unwrap();

        assert_eq!(texts(&a), ["in a"]);
        assert_eq!(texts(&b), ["in b"]);
        assert!(unknown.is_empty());
        assert!(!store.holds_cards("model-c", org).await.unwrap());
    }

    #[tokio::test]
    async fn a_model_does_not_see_the_cards_indexed_for_another() {
        let dir = TempDir::new();
        let store = store(&dir).await;
        let (mine, theirs) = (OrganizationId::new(), OrganizationId::new());
        store
            .upsert("model-a", card(mine, "mine", [1.0, 0.0, 0.0], Column::Todo))
            .await
            .unwrap();
        store
            .upsert(
                "model-b",
                card(theirs, "theirs", [1.0, 0.0, 0.0], Column::Todo),
            )
            .await
            .unwrap();

        assert!(store.holds_cards("model-a", mine).await.unwrap());
        assert!(!store.holds_cards("model-b", mine).await.unwrap());
        let found = store
            .nearest("model-b", mine, &Embedding(vec![1.0, 0.0, 0.0]), 5)
            .await
            .unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn a_wrong_dimension_for_a_model_is_refused() {
        let dir = TempDir::new();
        let store = store(&dir).await;
        let org = OrganizationId::new();
        store
            .upsert(MODEL, card(org, "c", [1.0, 0.0, 0.0], Column::Todo))
            .await
            .unwrap();
        let error = store
            .nearest(MODEL, org, &Embedding(vec![1.0, 0.0]), 1)
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)));
        let wrong = CardVector {
            embedding: Embedding(vec![1.0, 0.0]),
            ..card(org, "d", [0.0; 3], Column::Todo)
        };
        let error = store.upsert(MODEL, wrong).await.unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)));
    }
}
