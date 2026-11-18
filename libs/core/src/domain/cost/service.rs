use std::collections::HashMap;

use capitainerie_common::{CardId, CoreError, ProjectId};

use super::{
    AgentRef, AgentRow, AgentSpend, CostReport, ModelSpend, Period, ProjectSpend, Spent, TaskSpend,
    UserSpend, ports::CostRepository,
};
use crate::domain::authorization::{
    AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require,
};

pub struct CostService<'a, R, A> {
    costs: R,
    authorizer: &'a A,
}

impl<'a, R: CostRepository, A: Authorizer> CostService<'a, R, A> {
    pub fn new(costs: R, authorizer: &'a A) -> Self {
        Self { costs, authorizer }
    }

    pub async fn projects(
        mut self,
        subject: Subject,
        period: Period,
    ) -> Result<CostReport<ProjectSpend>, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject).await?;
        let rows = self.costs.project_rows(organization_id, &period).await?;
        let mut groups: Grouped<Option<ProjectId>, ProjectSpend> = Grouped::default();
        for row in rows {
            let group = groups.entry(row.project_id, || ProjectSpend {
                project_id: row.project_id,
                name: row.project_name.clone(),
                spent: Spent::default(),
                agents: Vec::new(),
            });
            group.spent.add(&row.spent);
            add_agent(&mut group.agents, row.agent, row.runs, &row.spent);
        }
        let mut rows = groups.into_vec();
        rows.iter_mut()
            .for_each(|group| sort_agents(&mut group.agents));
        rows.sort_by(|a, b| by_cost_then_name(&a.spent, &a.name, &b.spent, &b.name));
        Ok(report(&period, rows, |group| group.spent))
    }

    pub async fn agents(
        mut self,
        subject: Subject,
        period: Period,
    ) -> Result<CostReport<AgentSpend>, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject).await?;
        let rows = self.costs.agent_rows(organization_id, &period).await?;
        let mut agents = Vec::new();
        for AgentRow { agent, runs, spent } in rows {
            add_agent(&mut agents, agent, runs, &spent);
        }
        sort_agents(&mut agents);
        Ok(report(&period, agents, |agent| agent.spent))
    }

    pub async fn tasks(
        mut self,
        subject: Subject,
        period: Period,
        project: Option<ProjectId>,
    ) -> Result<CostReport<TaskSpend>, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject).await?;
        if let Some(project_id) = project
            && !self
                .costs
                .project_exists(organization_id, project_id)
                .await?
        {
            return Err(CoreError::NotFound);
        }
        let rows = self
            .costs
            .task_rows(organization_id, &period, project)
            .await?;
        let mut groups: Grouped<CardId, TaskSpend> = Grouped::default();
        for row in rows {
            let group = groups.entry(row.card_id, || TaskSpend {
                card_id: row.card_id,
                title: row.title.clone(),
                project_id: row.project_id,
                spent: Spent::default(),
                agents: Vec::new(),
            });
            group.spent.add(&row.spent);
            add_agent(&mut group.agents, row.agent, row.runs, &row.spent);
        }
        let mut rows = groups.into_vec();
        rows.iter_mut()
            .for_each(|group| sort_agents(&mut group.agents));
        rows.sort_by(|a, b| by_cost_then_name(&a.spent, &a.title, &b.spent, &b.title));
        Ok(report(&period, rows, |group| group.spent))
    }

    pub async fn users(
        mut self,
        subject: Subject,
        period: Period,
    ) -> Result<CostReport<UserSpend>, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject).await?;
        let rows = self.costs.user_rows(organization_id, &period).await?;
        let mut groups: Grouped<String, UserSpend> = Grouped::default();
        for row in rows {
            let group = groups.entry(row.user.clone(), || UserSpend {
                user: row.user.clone(),
                spent: Spent::default(),
                models: Vec::new(),
            });
            group.spent.add(&row.spent);
            match group.models.iter_mut().find(|m| m.model == row.model) {
                Some(model) => model.spent.add(&row.spent),
                None => group.models.push(ModelSpend {
                    model: row.model,
                    spent: row.spent,
                }),
            }
        }
        let mut rows = groups.into_vec();
        for group in &mut rows {
            group
                .models
                .sort_by(|a, b| by_cost_then_name(&a.spent, &a.model, &b.spent, &b.model));
        }
        rows.sort_by(|a, b| by_cost_then_name(&a.spent, &a.user, &b.spent, &b.user));
        Ok(report(&period, rows, |group| group.spent))
    }

    async fn require(&self, subject: Subject) -> Result<(), CoreError> {
        require(
            self.authorizer,
            AccessRequest::new(subject, Action::AuditRead, ResourceKind::Audit, None),
        )
        .await
    }
}

struct Grouped<K, V> {
    index: HashMap<K, usize>,
    items: Vec<V>,
}

impl<K, V> Default for Grouped<K, V> {
    fn default() -> Self {
        Self {
            index: HashMap::new(),
            items: Vec::new(),
        }
    }
}

impl<K: std::hash::Hash + Eq, V> Grouped<K, V> {
    fn entry(&mut self, key: K, new: impl FnOnce() -> V) -> &mut V {
        let position = *self.index.entry(key).or_insert_with(|| {
            self.items.push(new());
            self.items.len() - 1
        });
        &mut self.items[position]
    }

    fn into_vec(self) -> Vec<V> {
        self.items
    }
}

fn add_agent(agents: &mut Vec<AgentSpend>, agent: AgentRef, runs: i64, spent: &Spent) {
    match agents.iter_mut().find(|a| a.agent == agent) {
        Some(existing) => {
            existing.runs += runs;
            existing.spent.add(spent);
        }
        None => agents.push(AgentSpend {
            agent,
            runs,
            spent: *spent,
        }),
    }
}

fn sort_agents(agents: &mut [AgentSpend]) {
    agents.sort_by(|a, b| by_cost_then_name(&a.spent, &a.agent.name, &b.spent, &b.agent.name));
}

fn by_cost_then_name<N: Ord>(a: &Spent, a_name: &N, b: &Spent, b_name: &N) -> std::cmp::Ordering {
    b.cost_micros
        .cmp(&a.cost_micros)
        .then_with(|| a_name.cmp(b_name))
}

fn report<T>(period: &Period, rows: Vec<T>, spent: impl Fn(&T) -> Spent) -> CostReport<T> {
    let mut total = Spent::default();
    rows.iter().for_each(|row| total.add(&spent(row)));
    CostReport {
        from: period.from(),
        to: period.to(),
        total,
        rows,
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_common::{AgentId, CardId, CoreError, OrganizationId, ProjectId};
    use chrono::Utc;

    use super::*;
    use crate::domain::{
        authorization::{Decision, Subject, ports::MockAuthorizer},
        cost::{
            AgentRef, AgentRow, Period, ProjectAgentRow, Spent, TaskAgentRow, UserModelRow,
            ports::MockCostRepository,
        },
    };

    fn admin() -> Subject {
        Subject {
            id: "admin-1".into(),
            organization_id: OrganizationId::new(),
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

    fn period() -> Period {
        Period::new(None, None, Utc::now()).unwrap()
    }

    fn spent(cost_micros: i64) -> Spent {
        Spent {
            input_tokens: cost_micros * 2,
            output_tokens: cost_micros,
            cost_micros,
        }
    }

    fn agent(name: &str) -> (AgentId, AgentRef) {
        let id = AgentId::new();
        (
            id,
            AgentRef {
                id: Some(id),
                name: name.into(),
            },
        )
    }

    fn project_row(
        project: Option<(ProjectId, &str)>,
        agent: &AgentRef,
        cost: i64,
    ) -> ProjectAgentRow {
        ProjectAgentRow {
            project_id: project.map(|(id, _)| id),
            project_name: project.map(|(_, name)| name.into()),
            agent: agent.clone(),
            runs: 1,
            spent: spent(cost),
        }
    }

    #[tokio::test]
    async fn projects_group_agents_and_sum_to_the_total() {
        let p1 = ProjectId::new();
        let (_, a1) = agent("alpha");
        let (_, a2) = agent("beta");
        let rows = vec![
            project_row(Some((p1, "P1")), &a1, 100),
            project_row(Some((p1, "P1")), &a2, 50),
            project_row(None, &a1, 10),
        ];
        let mut repository = MockCostRepository::new();
        repository.expect_project_rows().returning(move |_, _| {
            let rows = rows.clone();
            Box::pin(async move { Ok(rows) })
        });
        let authorizer = allow();
        let report = CostService::new(repository, &authorizer)
            .projects(admin(), period())
            .await
            .unwrap();
        assert_eq!(report.total, spent(160));
        assert_eq!(report.rows.len(), 2);
        let first = &report.rows[0];
        assert_eq!(first.project_id, Some(p1));
        assert_eq!(first.name.as_deref(), Some("P1"));
        assert_eq!(first.spent, spent(150));
        assert_eq!(first.agents.len(), 2);
        let none = &report.rows[1];
        assert_eq!(none.project_id, None);
        assert_eq!(none.spent, spent(10));
        for group in &report.rows {
            let mut sum = Spent::default();
            group.agents.iter().for_each(|a| sum.add(&a.spent));
            assert_eq!(group.spent, sum);
        }
    }

    #[tokio::test]
    async fn groups_are_ordered_by_cost_then_name() {
        let (_, a) = agent("a");
        let (_, b) = agent("b");
        let (_, c) = agent("c");
        let rows = vec![
            AgentRow {
                agent: b.clone(),
                runs: 1,
                spent: spent(10),
            },
            AgentRow {
                agent: c.clone(),
                runs: 1,
                spent: spent(30),
            },
            AgentRow {
                agent: a.clone(),
                runs: 1,
                spent: spent(10),
            },
        ];
        let mut repository = MockCostRepository::new();
        repository.expect_agent_rows().returning(move |_, _| {
            let rows = rows.clone();
            Box::pin(async move { Ok(rows) })
        });
        let authorizer = allow();
        let report = CostService::new(repository, &authorizer)
            .agents(admin(), period())
            .await
            .unwrap();
        let names: Vec<_> = report.rows.iter().map(|r| r.agent.name.as_str()).collect();
        assert_eq!(names, ["c", "a", "b"]);
    }

    #[tokio::test]
    async fn tasks_group_agents_and_sum_to_the_total() {
        let card = CardId::new();
        let (_, a1) = agent("alpha");
        let (_, a2) = agent("beta");
        let row = |agent: &AgentRef, cost| TaskAgentRow {
            card_id: card,
            title: "Fix".into(),
            project_id: None,
            agent: agent.clone(),
            runs: 2,
            spent: spent(cost),
        };
        let rows = vec![row(&a1, 5), row(&a2, 7)];
        let mut repository = MockCostRepository::new();
        repository.expect_task_rows().returning(move |_, _, _| {
            let rows = rows.clone();
            Box::pin(async move { Ok(rows) })
        });
        let authorizer = allow();
        let report = CostService::new(repository, &authorizer)
            .tasks(admin(), period(), None)
            .await
            .unwrap();
        assert_eq!(report.total, spent(12));
        assert_eq!(report.rows.len(), 1);
        assert_eq!(report.rows[0].spent, spent(12));
        assert_eq!(report.rows[0].agents[0].agent.name, "beta");
    }

    #[tokio::test]
    async fn tasks_with_an_unknown_project_are_not_found() {
        let mut repository = MockCostRepository::new();
        repository
            .expect_project_exists()
            .returning(|_, _| Box::pin(async { Ok(false) }));
        repository.expect_task_rows().never();
        let authorizer = allow();
        let error = CostService::new(repository, &authorizer)
            .tasks(admin(), period(), Some(ProjectId::new()))
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::NotFound));
    }

    #[tokio::test]
    async fn users_group_models_and_sum_to_the_total() {
        let row = |user: &str, model: &str, cost| UserModelRow {
            user: user.into(),
            model: model.into(),
            spent: spent(cost),
        };
        let rows = vec![
            row("ann", "m1", 20),
            row("ann", "m2", 30),
            row("bob", "m1", 40),
        ];
        let mut repository = MockCostRepository::new();
        repository.expect_user_rows().returning(move |_, _| {
            let rows = rows.clone();
            Box::pin(async move { Ok(rows) })
        });
        let authorizer = allow();
        let report = CostService::new(repository, &authorizer)
            .users(admin(), period())
            .await
            .unwrap();
        assert_eq!(report.total, spent(90));
        assert_eq!(report.rows[0].user, "ann");
        assert_eq!(report.rows[0].spent, spent(50));
        assert_eq!(report.rows[0].models[0].model, "m2");
        assert_eq!(report.rows[1].user, "bob");
        assert_eq!(report.rows[1].models.len(), 1);
    }

    #[tokio::test]
    async fn an_empty_organization_reports_zero() {
        let mut repository = MockCostRepository::new();
        repository
            .expect_project_rows()
            .returning(|_, _| Box::pin(async { Ok(vec![]) }));
        let authorizer = allow();
        let report = CostService::new(repository, &authorizer)
            .projects(admin(), period())
            .await
            .unwrap();
        assert!(report.rows.is_empty());
        assert_eq!(report.total, Spent::default());
    }

    #[tokio::test]
    async fn a_refused_subject_reads_nothing() {
        let mut authorizer = MockAuthorizer::new();
        authorizer.expect_evaluate().returning(|_| {
            Box::pin(async {
                Ok(Decision::Deny {
                    reason: "no".into(),
                })
            })
        });
        let repository = MockCostRepository::new();
        let error = CostService::new(repository, &authorizer)
            .projects(admin(), period())
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Forbidden { .. }));
    }
}
