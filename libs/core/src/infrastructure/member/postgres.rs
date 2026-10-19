use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_macros::repository;

use crate::{
    domain::member::{Member, ports::MemberRepository},
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = Member, backend = Postgres)]
pub struct PgMemberRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgMemberRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl MemberRepository for PgMemberRepository<'_> {
    async fn upsert(&mut self, member: &Member) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO members (organization_id, subject, name, email, last_seen_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (organization_id, subject)
            DO UPDATE SET name = EXCLUDED.name, email = EXCLUDED.email, last_seen_at = EXCLUDED.last_seen_at
            "#,
            member.organization_id.0,
            member.subject,
            member.name,
            member.email,
            member.last_seen_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        subject: &str,
    ) -> Result<Option<Member>, CoreError> {
        let mut tx = self.tx.lock().await;
        Ok(sqlx::query!(
            r#"
            SELECT organization_id, subject, name, email, last_seen_at
            FROM members
            WHERE organization_id = $1 AND subject = $2
            "#,
            organization_id.0,
            subject,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(|row| Member {
            organization_id: OrganizationId(row.organization_id),
            subject: row.subject,
            name: row.name,
            email: row.email,
            last_seen_at: row.last_seen_at,
        }))
    }

    async fn list(&mut self, organization_id: OrganizationId) -> Result<Vec<Member>, CoreError> {
        let mut tx = self.tx.lock().await;
        Ok(sqlx::query!(
            r#"
            SELECT organization_id, subject, name, email, last_seen_at
            FROM members
            WHERE organization_id = $1
            ORDER BY lower(name), subject
            "#,
            organization_id.0,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(|row| Member {
            organization_id: OrganizationId(row.organization_id),
            subject: row.subject,
            name: row.name,
            email: row.email,
            last_seen_at: row.last_seen_at,
        })
        .collect())
    }
}
