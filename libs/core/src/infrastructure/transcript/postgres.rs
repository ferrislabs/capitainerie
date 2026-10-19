use capitainerie_common::{ClientId, CoreError, OrganizationId, ProjectId, TranscriptId};
use capitainerie_macros::repository;
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        client::{ClientRef, links::Links},
        project::ProjectRef,
        transcript::{
            MeetingMetadata, Transcript, TranscriptBody, TranscriptTitle,
            ports::TranscriptRepository,
        },
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = Transcript, backend = Postgres)]
pub struct PgTranscriptRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgTranscriptRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl TranscriptRepository for PgTranscriptRepository<'_> {
    async fn insert(&mut self, transcript: &Transcript) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO transcripts (
                id, organization_id, submitted_by, title, body, created_at,
                meeting_date, participants, repository, client_id, project_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            transcript.id.0,
            transcript.organization_id.0,
            transcript.submitted_by,
            transcript.title.as_str(),
            transcript.body.as_str(),
            transcript.created_at,
            transcript.metadata.date,
            &transcript.metadata.participants,
            transcript
                .metadata
                .repository
                .as_ref()
                .map(ToString::to_string),
            transcript.links.client_id().map(|id| id.0),
            transcript.links.project_id().map(|id| id.0),
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        id: TranscriptId,
    ) -> Result<Option<Transcript>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            TranscriptRow,
            r#"
            SELECT t.id, t.organization_id, t.submitted_by, t.title, t.body, t.created_at,
                   t.meeting_date, t.participants, t.repository,
                   t.client_id, cl.name AS "client_name?", t.project_id, p.name AS "project_name?"
            FROM transcripts t
            LEFT JOIN clients cl ON cl.id = t.client_id
            LEFT JOIN projects p ON p.id = t.project_id
            WHERE t.organization_id = $1 AND t.id = $2
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(Transcript::try_from)
        .transpose()
    }
}

struct TranscriptRow {
    id: Uuid,
    organization_id: Uuid,
    submitted_by: String,
    title: String,
    body: String,
    created_at: DateTime<Utc>,
    meeting_date: Option<NaiveDate>,
    participants: Vec<String>,
    repository: Option<String>,
    client_id: Option<Uuid>,
    client_name: Option<String>,
    project_id: Option<Uuid>,
    project_name: Option<String>,
}

impl TryFrom<TranscriptRow> for Transcript {
    type Error = CoreError;

    fn try_from(row: TranscriptRow) -> Result<Self, Self::Error> {
        let stored = |error: CoreError| {
            CoreError::Internal(format!("stored transcript {}: {error}", row.id))
        };
        Ok(Self {
            id: TranscriptId(row.id),
            organization_id: OrganizationId(row.organization_id),
            submitted_by: row.submitted_by,
            title: TranscriptTitle::try_from(row.title).map_err(stored)?,
            body: TranscriptBody::try_from(row.body).map_err(stored)?,
            metadata: MeetingMetadata::new(
                row.meeting_date,
                Some(row.participants),
                row.repository,
            )
            .map_err(stored)?,
            links: Links::stored(
                row.client_id
                    .zip(row.client_name)
                    .map(|(id, name)| ClientRef {
                        id: ClientId(id),
                        name,
                    }),
                row.project_id
                    .zip(row.project_name)
                    .map(|(id, name)| ProjectRef {
                        id: ProjectId(id),
                        name,
                    }),
            ),
            created_at: row.created_at,
        })
    }
}
