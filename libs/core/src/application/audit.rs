use capitainerie_common::CoreError;
use capitainerie_macros::transactional;
use chrono::Utc;

use super::CapitainerieUseCase;
use crate::domain::{
    audit::{AuditFilter, AuditPage, service::AuditService},
    authorization::Subject,
    budget::{BudgetOverview, BudgetSettings, service::BudgetService},
};

impl CapitainerieUseCase {
    #[transactional(audit, authz)]
    pub async fn list_audit_events(
        &self,
        subject: Subject,
        filter: AuditFilter,
    ) -> Result<AuditPage, CoreError> {
        AuditService::new(audit_repository, authz)
            .list(subject, filter)
            .await
    }

    #[transactional(budget, authz)]
    pub async fn budgets(&self, subject: Subject) -> Result<BudgetOverview, CoreError> {
        BudgetService::new(budget_repository, authz)
            .overview(subject, Utc::now())
            .await
    }

    #[transactional(budget, authz)]
    pub async fn save_budgets(
        &self,
        subject: Subject,
        settings: BudgetSettings,
    ) -> Result<BudgetOverview, CoreError> {
        BudgetService::new(budget_repository, authz)
            .save(subject, settings, Utc::now())
            .await
    }
}
