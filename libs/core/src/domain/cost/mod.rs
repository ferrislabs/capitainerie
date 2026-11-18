use capitainerie_common::{AgentId, CardId, CoreError, ProjectId};
use chrono::{DateTime, Duration, NaiveDate, Utc};

pub mod ports;
pub mod service;

pub const DEFAULT_WINDOW_DAYS: i64 = 30;

/// A window `[from, to[` over which costs are reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Period {
    from: DateTime<Utc>,
    to: DateTime<Utc>,
}

impl Period {
    pub fn new(
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> Result<Self, CoreError> {
        let to = to.unwrap_or(now);
        let from = from.unwrap_or(to - Duration::days(DEFAULT_WINDOW_DAYS));
        if from >= to {
            return Err(CoreError::Invalid("`from` must be before `to`".into()));
        }
        Ok(Self { from, to })
    }

    pub fn from(&self) -> DateTime<Utc> {
        self.from
    }

    pub fn to(&self) -> DateTime<Utc> {
        self.to
    }
}

/// Narrows a report to the steps of one agent and/or one task; both apply together.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CostFilter {
    pub agent: Option<AgentId>,
    pub card: Option<CardId>,
}

/// Tokens and cost in micro-dollars; a step without cost counts 0.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Spent {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_micros: i64,
}

impl Spent {
    pub fn add(&mut self, other: &Spent) {
        self.input_tokens = self.input_tokens.saturating_add(other.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(other.output_tokens);
        self.cost_micros = self.cost_micros.saturating_add(other.cost_micros);
    }
}

/// `id` is `None` for work no longer tied to an agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRef {
    pub id: Option<AgentId>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectAgentRow {
    pub project_id: Option<ProjectId>,
    pub project_name: Option<String>,
    pub agent: AgentRef,
    pub runs: i64,
    pub spent: Spent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAgentRow {
    pub card_id: CardId,
    pub title: String,
    pub project_id: Option<ProjectId>,
    pub agent: AgentRef,
    pub runs: i64,
    pub spent: Spent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserModelRow {
    pub user: String,
    pub model: String,
    pub spent: Spent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRow {
    pub agent: AgentRef,
    pub runs: i64,
    pub spent: Spent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSpend {
    pub agent: AgentRef,
    pub runs: i64,
    pub spent: Spent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSpend {
    pub project_id: Option<ProjectId>,
    pub name: Option<String>,
    pub spent: Spent,
    pub agents: Vec<AgentSpend>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSpend {
    pub card_id: CardId,
    pub title: String,
    pub project_id: Option<ProjectId>,
    pub spent: Spent,
    pub agents: Vec<AgentSpend>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSpend {
    pub model: String,
    pub spent: Spent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSpend {
    pub user: String,
    pub spent: Spent,
    pub models: Vec<ModelSpend>,
}

/// Spend of one UTC day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaySpend {
    pub day: NaiveDate,
    pub spent: Spent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostReport<T> {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub total: Spent,
    pub rows: Vec<T>,
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    fn spent_add_saturates() {
        let mut spent = Spent {
            input_tokens: i64::MAX,
            output_tokens: 1,
            cost_micros: i64::MAX - 1,
        };
        spent.add(&Spent {
            input_tokens: 1,
            output_tokens: 2,
            cost_micros: 5,
        });
        assert_eq!(spent.input_tokens, i64::MAX);
        assert_eq!(spent.output_tokens, 3);
        assert_eq!(spent.cost_micros, i64::MAX);
    }

    #[test]
    fn defaults_to_the_last_thirty_days() {
        let now = Utc::now();
        let period = Period::new(None, None, now).unwrap();
        assert_eq!(period.to(), now);
        assert_eq!(period.from(), now - Duration::days(30));
    }

    #[test]
    fn rejects_an_empty_or_reversed_window() {
        let now = Utc::now();
        let earlier = now - Duration::days(1);
        assert!(matches!(
            Period::new(Some(now), Some(now), now),
            Err(CoreError::Invalid(_))
        ));
        assert!(matches!(
            Period::new(Some(now), Some(earlier), now),
            Err(CoreError::Invalid(_))
        ));
    }

    #[test]
    fn keeps_explicit_bounds() {
        let now = Utc::now();
        let from = now - Duration::days(3);
        let to = now - Duration::days(1);
        let period = Period::new(Some(from), Some(to), now).unwrap();
        assert_eq!((period.from(), period.to()), (from, to));
    }
}
