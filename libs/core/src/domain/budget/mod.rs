pub mod ports;
pub mod service;

use std::collections::{HashMap, HashSet};

use capitainerie_common::{AgentId, CoreError, OrganizationId};
use chrono::{DateTime, Datelike, TimeZone, Utc};
use uuid::Uuid;

pub const MAX_LIMIT_CENTS: i64 = 100_000_000_000;
pub const DEFAULT_ALERT_PERCENT: i16 = 80;
pub const MAX_AGENT_BUDGETS: usize = 200;
const MICROS_PER_CENT: i128 = 10_000;

/// A monthly limit and when to warn before reaching it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limit {
    pub monthly_limit_cents: i64,
    pub alert_threshold_percent: i16,
}

impl Limit {
    pub fn new(
        monthly_limit_cents: i64,
        alert_threshold_percent: Option<i16>,
    ) -> Result<Self, CoreError> {
        if !(1..=MAX_LIMIT_CENTS).contains(&monthly_limit_cents) {
            return Err(CoreError::Invalid(format!(
                "a monthly limit is between 1 and {MAX_LIMIT_CENTS} cents"
            )));
        }
        let alert_threshold_percent = alert_threshold_percent.unwrap_or(DEFAULT_ALERT_PERCENT);
        if !(1..=100).contains(&alert_threshold_percent) {
            return Err(CoreError::Invalid(
                "the alert threshold is a percentage between 1 and 100".into(),
            ));
        }
        Ok(Self {
            monthly_limit_cents,
            alert_threshold_percent,
        })
    }
}

/// Every budget of an organization, as a whole: saving it replaces the previous set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BudgetSettings {
    pub organization: Option<Limit>,
    pub agents: Vec<(AgentId, Limit)>,
}

impl BudgetSettings {
    pub fn new(
        organization: Option<Limit>,
        agents: Vec<(AgentId, Limit)>,
    ) -> Result<Self, CoreError> {
        if agents.len() > MAX_AGENT_BUDGETS {
            return Err(CoreError::Invalid(format!(
                "at most {MAX_AGENT_BUDGETS} agent budgets"
            )));
        }
        let mut seen = HashSet::new();
        if let Some((agent, _)) = agents.iter().find(|(agent, _)| !seen.insert(*agent)) {
            return Err(CoreError::Invalid(format!("agent {agent} has two budgets")));
        }
        Ok(Self {
            organization,
            agents,
        })
    }

    pub fn agent_ids(&self) -> Vec<AgentId> {
        self.agents.iter().map(|(agent, _)| *agent).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Budget {
    pub id: Uuid,
    pub organization_id: OrganizationId,
    /// `None` for the organization's own budget.
    pub agent_id: Option<AgentId>,
    pub agent_name: Option<String>,
    pub limit: Limit,
    pub updated_by: String,
    pub updated_at: DateTime<Utc>,
}

/// What the organization spent over a window, in micro-dollars.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Spend {
    pub total_micros: i64,
    pub by_agent: HashMap<AgentId, i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetStatus {
    pub budget: Budget,
    pub spent_micros: i64,
    /// Share of the limit spent, rounded down; above 100 once exceeded.
    pub percent: i64,
    pub alert: bool,
    pub exceeded: bool,
}

impl BudgetStatus {
    pub fn of(budget: Budget, spend: &Spend) -> Self {
        let spent_micros = match budget.agent_id {
            None => spend.total_micros,
            Some(agent) => spend.by_agent.get(&agent).copied().unwrap_or_default(),
        };
        let limit_micros = i128::from(budget.limit.monthly_limit_cents) * MICROS_PER_CENT;
        let percent = i64::try_from(i128::from(spent_micros.max(0)) * 100 / limit_micros.max(1))
            .unwrap_or(i64::MAX);
        let exceeded = i128::from(spent_micros) > limit_micros;
        Self {
            alert: percent >= i64::from(budget.limit.alert_threshold_percent),
            exceeded,
            percent,
            spent_micros,
            budget,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetOverview {
    pub month_start: DateTime<Utc>,
    pub month_end: DateTime<Utc>,
    /// Spent by the whole organization this month, budgeted or not.
    pub spent_micros: i64,
    /// The organization's budget first, then the agents' by name.
    pub items: Vec<BudgetStatus>,
}

/// The calendar month (UTC) `now` falls in, as `[start, end)`.
pub fn month_of(now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
    let first = |year: i32, month: u32| {
        Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0)
            .single()
            .unwrap_or(now)
    };
    let start = first(now.year(), now.month());
    let end = if now.month() == 12 {
        first(now.year() + 1, 1)
    } else {
        first(now.year(), now.month() + 1)
    };
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget(agent_id: Option<AgentId>, cents: i64, alert: i16) -> Budget {
        Budget {
            id: Uuid::now_v7(),
            organization_id: OrganizationId::new(),
            agent_id,
            agent_name: None,
            limit: Limit::new(cents, Some(alert)).unwrap(),
            updated_by: "user-1".into(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn limits_and_thresholds_are_bounded() {
        assert!(Limit::new(0, None).is_err());
        assert!(Limit::new(MAX_LIMIT_CENTS + 1, None).is_err());
        assert!(Limit::new(100, Some(0)).is_err());
        assert!(Limit::new(100, Some(101)).is_err());
        assert_eq!(
            Limit::new(100, None).unwrap().alert_threshold_percent,
            DEFAULT_ALERT_PERCENT
        );
    }

    #[test]
    fn an_agent_has_one_budget_at_most() {
        let agent = AgentId::new();
        let limit = Limit::new(100, None).unwrap();
        assert!(BudgetSettings::new(None, vec![(agent, limit), (agent, limit)]).is_err());
        assert!(BudgetSettings::new(None, vec![(agent, limit)]).is_ok());
    }

    #[test]
    fn the_status_compares_the_spend_to_the_limit() {
        let agent = AgentId::new();
        let spend = Spend {
            total_micros: 3_000_000,
            by_agent: HashMap::from([(agent, 4_500_000)]),
        };
        let organization = BudgetStatus::of(budget(None, 400, 80), &spend);
        assert_eq!(organization.spent_micros, 3_000_000);
        assert_eq!(organization.percent, 75);
        assert!(!organization.alert);
        assert!(!organization.exceeded);
        let agent_status = BudgetStatus::of(budget(Some(agent), 400, 80), &spend);
        assert_eq!(agent_status.percent, 112);
        assert!(agent_status.alert);
        assert!(agent_status.exceeded);
        let idle = BudgetStatus::of(budget(Some(AgentId::new()), 400, 80), &spend);
        assert_eq!(idle.spent_micros, 0);
        assert_eq!(idle.percent, 0);
    }

    #[test]
    fn the_month_runs_from_the_first_to_the_next_first() {
        let now = Utc.with_ymd_and_hms(2026, 12, 18, 10, 0, 0).unwrap();
        let (start, end) = month_of(now);
        assert_eq!(start, Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap());
        assert_eq!(end, Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap());
        let (start, end) = month_of(Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap());
        assert_eq!(start, Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap());
        assert_eq!(end, Utc.with_ymd_and_hms(2026, 3, 1, 0, 0, 0).unwrap());
    }
}
