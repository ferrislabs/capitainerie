use capitainerie_common::{CoreError, ProjectId};
use capitainerie_macros::transactional;
use chrono::Utc;

use super::CapitainerieUseCase;
use crate::domain::{
    audit::{AuditFilter, AuditPage, service::AuditService},
    authorization::Subject,
    budget::{BudgetOverview, BudgetSettings, service::BudgetService},
    cost::{
        AgentSpend, CostFilter, CostReport, DaySpend, Period, ProjectSpend, TaskSpend, UserSpend,
        service::CostService,
    },
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

    #[transactional(cost, authz)]
    pub async fn project_costs(
        &self,
        subject: Subject,
        period: Period,
        filter: CostFilter,
    ) -> Result<CostReport<ProjectSpend>, CoreError> {
        CostService::new(cost_repository, authz)
            .projects(subject, period, filter)
            .await
    }

    #[transactional(cost, authz)]
    pub async fn agent_costs(
        &self,
        subject: Subject,
        period: Period,
        filter: CostFilter,
    ) -> Result<CostReport<AgentSpend>, CoreError> {
        CostService::new(cost_repository, authz)
            .agents(subject, period, filter)
            .await
    }

    #[transactional(cost, authz)]
    pub async fn task_costs(
        &self,
        subject: Subject,
        period: Period,
        project: Option<ProjectId>,
        filter: CostFilter,
    ) -> Result<CostReport<TaskSpend>, CoreError> {
        CostService::new(cost_repository, authz)
            .tasks(subject, period, project, filter)
            .await
    }

    #[transactional(cost, authz)]
    pub async fn user_costs(
        &self,
        subject: Subject,
        period: Period,
        filter: CostFilter,
    ) -> Result<CostReport<UserSpend>, CoreError> {
        CostService::new(cost_repository, authz)
            .users(subject, period, filter)
            .await
    }

    #[transactional(cost, authz)]
    pub async fn day_costs(
        &self,
        subject: Subject,
        period: Period,
        filter: CostFilter,
    ) -> Result<CostReport<DaySpend>, CoreError> {
        CostService::new(cost_repository, authz)
            .days(subject, period, filter)
            .await
    }
}
