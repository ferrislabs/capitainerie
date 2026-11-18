use capitainerie_common::CoreError;
use chrono::{DateTime, Utc};

use super::{BudgetOverview, BudgetSettings, BudgetStatus, month_of, ports::BudgetRepository};
use crate::domain::authorization::{
    AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require,
};

pub struct BudgetService<'a, R, A> {
    budgets: R,
    authorizer: &'a A,
}

impl<'a, R: BudgetRepository, A: Authorizer> BudgetService<'a, R, A> {
    pub fn new(budgets: R, authorizer: &'a A) -> Self {
        Self {
            budgets,
            authorizer,
        }
    }

    pub async fn overview(
        mut self,
        subject: Subject,
        now: DateTime<Utc>,
    ) -> Result<BudgetOverview, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::AuditRead).await?;
        self.assemble(organization_id, now).await
    }

    pub async fn save(
        mut self,
        subject: Subject,
        settings: BudgetSettings,
        now: DateTime<Utc>,
    ) -> Result<BudgetOverview, CoreError> {
        let organization_id = subject.organization_id;
        let by = subject.id.clone();
        self.require(subject, Action::BudgetManage).await?;
        let agents = settings.agent_ids();
        if !agents.is_empty() {
            let known = self.budgets.count_agents(organization_id, &agents).await?;
            if usize::try_from(known).unwrap_or_default() != agents.len() {
                return Err(CoreError::Invalid(
                    "a budget names an agent the organization does not have".into(),
                ));
            }
        }
        self.budgets
            .replace(organization_id, &settings, &by, now)
            .await?;
        self.assemble(organization_id, now).await
    }

    async fn assemble(
        &mut self,
        organization_id: capitainerie_common::OrganizationId,
        now: DateTime<Utc>,
    ) -> Result<BudgetOverview, CoreError> {
        let (month_start, month_end) = month_of(now);
        let spend = self
            .budgets
            .spend(organization_id, month_start, month_end)
            .await?;
        let mut budgets = self.budgets.list(organization_id).await?;
        budgets.sort_by(|a, b| {
            (a.agent_id.is_some(), &a.agent_name).cmp(&(b.agent_id.is_some(), &b.agent_name))
        });
        Ok(BudgetOverview {
            month_start,
            month_end,
            spent_micros: spend.total_micros,
            items: budgets
                .into_iter()
                .map(|budget| BudgetStatus::of(budget, &spend))
                .collect(),
        })
    }

    async fn require(&self, subject: Subject, action: Action) -> Result<(), CoreError> {
        require(
            self.authorizer,
            AccessRequest::new(subject, action, ResourceKind::Budget, None),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_common::{AgentId, OrganizationId};

    use super::*;
    use crate::domain::{
        authorization::{Decision, ports::MockAuthorizer},
        budget::{Limit, Spend, ports::MockBudgetRepository},
    };

    fn admin(organization_id: OrganizationId) -> Subject {
        Subject {
            id: "admin-1".into(),
            organization_id,
            actor: None,
            is_admin: true,
        }
    }

    fn allow() -> MockAuthorizer {
        let mut authorizer = MockAuthorizer::new();
        authorizer
            .expect_evaluate()
            .returning(|_| Box::pin(async { Ok(Decision::Allow) }));
        authorizer
    }

    #[tokio::test]
    async fn a_budget_on_a_foreign_agent_is_refused_before_anything_is_saved() {
        let mut repository = MockBudgetRepository::new();
        repository
            .expect_count_agents()
            .returning(|_, _| Box::pin(async { Ok(0) }));
        repository.expect_replace().never();
        let authorizer = allow();
        let settings =
            BudgetSettings::new(None, vec![(AgentId::new(), Limit::new(100, None).unwrap())])
                .unwrap();
        let error = BudgetService::new(repository, &authorizer)
            .save(admin(OrganizationId::new()), settings, Utc::now())
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)));
    }

    #[tokio::test]
    async fn saving_replaces_the_set_and_returns_the_month() {
        let mut repository = MockBudgetRepository::new();
        repository
            .expect_replace()
            .times(1)
            .withf(|_, settings, by, _| settings.organization.is_some() && by == "admin-1")
            .returning(|_, _, _, _| Box::pin(async { Ok(()) }));
        repository.expect_spend().returning(|_, _, _| {
            Box::pin(async {
                Ok(Spend {
                    total_micros: 42,
                    ..Spend::default()
                })
            })
        });
        repository
            .expect_list()
            .returning(|_| Box::pin(async { Ok(vec![]) }));
        let authorizer = allow();
        let settings =
            BudgetSettings::new(Some(Limit::new(40_000, None).unwrap()), vec![]).unwrap();
        let overview = BudgetService::new(repository, &authorizer)
            .save(admin(OrganizationId::new()), settings, Utc::now())
            .await
            .unwrap();
        assert_eq!(overview.spent_micros, 42);
        assert!(overview.month_start < overview.month_end);
    }
}
