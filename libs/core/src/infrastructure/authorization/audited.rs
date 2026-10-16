use capitainerie_common::CoreError;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::authorization::{AccessRequest, Decision, ports::Authorizer},
    infrastructure::postgres::map_sqlx_error,
};

/// Records every decision of the wrapped PDP. Written on its own connection,
/// outside the business transaction, so a denial survives the rollback it
/// causes. A decision that cannot be recorded is not returned: no unaudited
/// write goes through.
#[derive(Clone)]
pub struct AuditedAuthorizer<A> {
    inner: A,
    pool: PgPool,
}

impl<A> AuditedAuthorizer<A> {
    pub fn new(inner: A, pool: PgPool) -> Self {
        Self { inner, pool }
    }
}

impl<A: Authorizer> Authorizer for AuditedAuthorizer<A> {
    async fn evaluate(&self, request: &AccessRequest) -> Result<Decision, CoreError> {
        let decision = self.inner.evaluate(request).await?;
        let (allowed, reason) = match &decision {
            Decision::Allow => (true, None),
            Decision::Deny { reason } => (false, Some(reason.as_str())),
        };
        sqlx::query!(
            r#"
            INSERT INTO authorization_decisions
                (id, organization_id, subject, actor, action, resource_kind, resource_id, allowed, reason, run_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            Uuid::now_v7(),
            request.subject.organization_id.0,
            request.subject.id,
            request.subject.actor,
            request.action.as_str(),
            request.resource.kind.as_str(),
            request.resource.id,
            allowed,
            reason,
            request.run_id.map(|id| id.0),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        tracing::info!(
            action = request.action.as_str(),
            actor = request.subject.actor.as_deref(),
            allowed,
            reason,
            "authorization decision"
        );
        Ok(decision)
    }
}
