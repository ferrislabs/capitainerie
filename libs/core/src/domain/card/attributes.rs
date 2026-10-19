use std::collections::HashSet;

use capitainerie_common::{ClientId, CoreError, ProjectId};
use chrono::{Months, NaiveDate, Utc};

use crate::domain::label::clean_label;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Criterion {
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AcceptanceCriteria(Vec<Criterion>);

impl AcceptanceCriteria {
    pub const MAX_COUNT: usize = 20;
    pub const MAX_CHARS: usize = 300;

    pub fn as_slice(&self) -> &[Criterion] {
        &self.0
    }

    pub fn done_count(&self) -> usize {
        self.0.iter().filter(|criterion| criterion.done).count()
    }
}

impl TryFrom<Vec<Criterion>> for AcceptanceCriteria {
    type Error = CoreError;

    fn try_from(values: Vec<Criterion>) -> Result<Self, Self::Error> {
        if values.len() > Self::MAX_COUNT {
            return Err(CoreError::Invalid(format!(
                "there are more than {} acceptance criteria",
                Self::MAX_COUNT
            )));
        }
        values
            .into_iter()
            .map(|criterion| {
                let text = criterion.text.trim();
                if text.is_empty() {
                    return Err(CoreError::Invalid(
                        "an acceptance criterion is empty".into(),
                    ));
                }
                if text.chars().count() > Self::MAX_CHARS {
                    return Err(CoreError::Invalid(format!(
                        "an acceptance criterion exceeds {} characters",
                        Self::MAX_CHARS
                    )));
                }
                if text
                    .chars()
                    .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
                {
                    return Err(CoreError::Invalid(
                        "an acceptance criterion contains a control character".into(),
                    ));
                }
                Ok(Criterion {
                    text: text.to_owned(),
                    done: criterion.done,
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Self)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

impl Priority {
    pub const ALL: [Priority; 4] = [
        Priority::Low,
        Priority::Normal,
        Priority::High,
        Priority::Urgent,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }
}

impl TryFrom<&str> for Priority {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|priority| priority.as_str() == value)
            .ok_or_else(|| CoreError::Invalid(format!("unknown priority {value}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Estimate {
    Xs,
    S,
    M,
    L,
    Xl,
}

impl Estimate {
    pub const ALL: [Estimate; 5] = [
        Estimate::Xs,
        Estimate::S,
        Estimate::M,
        Estimate::L,
        Estimate::Xl,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Xs => "xs",
            Self::S => "s",
            Self::M => "m",
            Self::L => "l",
            Self::Xl => "xl",
        }
    }
}

impl TryFrom<&str> for Estimate {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|estimate| estimate.as_str() == value)
            .ok_or_else(|| CoreError::Invalid(format!("unknown estimate {value}")))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Labels(Vec<String>);

impl Labels {
    pub const MAX_COUNT: usize = 10;
    pub const MAX_CHARS: usize = 30;

    pub fn as_slice(&self) -> &[String] {
        &self.0
    }
}

impl TryFrom<Vec<String>> for Labels {
    type Error = CoreError;

    fn try_from(values: Vec<String>) -> Result<Self, Self::Error> {
        let mut labels = values
            .iter()
            .map(|label| clean_label(label, Self::MAX_CHARS, "a label"))
            .collect::<Result<Vec<_>, _>>()?;
        let mut seen = HashSet::new();
        labels.retain(|label| seen.insert(label.to_lowercase()));
        if labels.len() > Self::MAX_COUNT {
            return Err(CoreError::Invalid(format!(
                "there are more than {} labels",
                Self::MAX_COUNT
            )));
        }
        Ok(Self(labels))
    }
}

/// A calendar date as `YYYY-MM-DD`.
pub fn parse_due_date(value: &str) -> Result<NaiveDate, CoreError> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| CoreError::Invalid(format!("a due date is YYYY-MM-DD, not {value}")))
}

pub const DUE_DATE_MAX_YEARS_AHEAD: u32 = 2;

/// Accepts `date` only from `floor` to two years after it.
pub fn check_due_date_window(date: NaiveDate, floor: NaiveDate) -> Result<NaiveDate, CoreError> {
    let ceiling = floor
        .checked_add_months(Months::new(12 * DUE_DATE_MAX_YEARS_AHEAD))
        .unwrap_or(NaiveDate::MAX);
    if date < floor {
        return Err(CoreError::Invalid(format!(
            "due_date {date} is in the past: give a date from {floor} to {ceiling}, or omit it"
        )));
    }
    if date > ceiling {
        return Err(CoreError::Invalid(format!(
            "due_date {date} is too far ahead: give a date from {floor} to {ceiling}, or omit it"
        )));
    }
    Ok(date)
}

/// The window an agent's deadline must fall in: not before today, nor before
/// the meeting it comes from, and at most two years after that.
pub fn agent_due_date(date: NaiveDate, meeting: Option<NaiveDate>) -> Result<NaiveDate, CoreError> {
    let today = Utc::now().date_naive();
    check_due_date_window(date, meeting.map_or(today, |meeting| meeting.max(today)))
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attributes {
    pub acceptance_criteria: AcceptanceCriteria,
    pub priority: Priority,
    pub due_date: Option<NaiveDate>,
    pub assignee_subject: Option<String>,
    pub labels: Labels,
    pub estimate: Option<Estimate>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardFilter {
    pub client_id: Option<ClientId>,
    pub project_id: Option<ProjectId>,
    pub priority: Option<Priority>,
    pub assignee: Option<String>,
    pub label: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn criterion(text: &str, done: bool) -> Criterion {
        Criterion {
            text: text.into(),
            done,
        }
    }

    #[test]
    fn criteria_are_trimmed_and_keep_their_state() {
        let criteria = AcceptanceCriteria::try_from(vec![
            criterion("  La CI passe sur ARM ", true),
            criterion("Le déploiement est documenté", false),
        ])
        .unwrap();
        assert_eq!(criteria.as_slice()[0].text, "La CI passe sur ARM");
        assert_eq!(criteria.done_count(), 1);
    }

    #[test]
    fn criteria_are_bounded_in_count_and_characters() {
        let many = |count: usize| {
            (0..count)
                .map(|n| criterion(&format!("c{n}"), false))
                .collect::<Vec<_>>()
        };
        assert!(AcceptanceCriteria::try_from(many(AcceptanceCriteria::MAX_COUNT)).is_ok());
        assert!(AcceptanceCriteria::try_from(many(AcceptanceCriteria::MAX_COUNT + 1)).is_err());
        let long = |chars: usize| vec![criterion(&"é".repeat(chars), false)];
        assert!(AcceptanceCriteria::try_from(long(AcceptanceCriteria::MAX_CHARS)).is_ok());
        assert!(AcceptanceCriteria::try_from(long(AcceptanceCriteria::MAX_CHARS + 1)).is_err());
    }

    #[test]
    fn a_criterion_with_a_control_character_or_line_separator_is_refused() {
        for bad in [
            "a\nb",
            "a\tb",
            "a\rb",
            "a\u{0}b",
            "a\u{2028}b",
            "a\u{2029}b",
        ] {
            assert!(
                AcceptanceCriteria::try_from(vec![criterion(bad, false)]).is_err(),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn a_blank_criterion_is_refused() {
        assert!(AcceptanceCriteria::try_from(vec![criterion("   ", false)]).is_err());
        assert!(AcceptanceCriteria::try_from(vec![criterion("", true)]).is_err());
        assert!(AcceptanceCriteria::try_from(vec![]).is_ok());
    }

    #[test]
    fn priorities_and_estimates_round_trip_and_reject_unknown_names() {
        for priority in Priority::ALL {
            assert_eq!(Priority::try_from(priority.as_str()).unwrap(), priority);
        }
        for estimate in Estimate::ALL {
            assert_eq!(Estimate::try_from(estimate.as_str()).unwrap(), estimate);
        }
        assert!(Priority::try_from("critical").is_err());
        assert!(Priority::try_from("Urgent").is_err());
        assert!(Estimate::try_from("xxl").is_err());
        assert_eq!(Priority::default(), Priority::Normal);
    }

    #[test]
    fn labels_are_trimmed_deduplicated_without_case_and_capped_at_ten() {
        let labels =
            Labels::try_from(vec!["Urgent".into(), " urgent ".into(), "Infra".into()]).unwrap();
        assert_eq!(labels.as_slice(), ["Urgent", "Infra"]);
        let many = |count: usize| (0..count).map(|n| format!("l{n}")).collect::<Vec<_>>();
        assert!(Labels::try_from(many(Labels::MAX_COUNT)).is_ok());
        assert!(Labels::try_from(many(Labels::MAX_COUNT + 1)).is_err());
    }

    #[test]
    fn due_dates_are_iso_calendar_dates() {
        assert_eq!(
            parse_due_date(" 2026-10-31 ").unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 31).unwrap()
        );
        for bad in ["vendredi", "2026-13-40", "31/10/2026", "2026-02-30", ""] {
            assert!(parse_due_date(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_due_date_window_runs_from_the_floor_to_two_years_after() {
        let day = |y, m, d| NaiveDate::from_ymd_opt(y, m, d).unwrap();
        let floor = day(2026, 10, 19);
        assert!(check_due_date_window(floor, floor).is_ok());
        assert!(check_due_date_window(day(2028, 10, 19), floor).is_ok());
        assert!(check_due_date_window(day(2026, 10, 18), floor).is_err());
        assert!(check_due_date_window(day(2028, 10, 20), floor).is_err());
        let past = check_due_date_window(day(2020, 1, 1), floor).unwrap_err();
        assert!(
            matches!(past, CoreError::Invalid(m) if m.contains("in the past") && m.contains("2028-10-19"))
        );
    }

    #[test]
    fn an_agent_deadline_is_bounded_by_today_and_the_meeting() {
        let day = |y, m, d| NaiveDate::from_ymd_opt(y, m, d).unwrap();
        assert!(agent_due_date(day(2001, 1, 1), None).is_err());
        assert!(agent_due_date(day(2001, 1, 1), Some(day(2099, 1, 1))).is_err());
        assert!(agent_due_date(day(2099, 1, 5), Some(day(2099, 1, 1))).is_ok());
        assert!(agent_due_date(day(2098, 12, 31), Some(day(2099, 1, 1))).is_err());
        assert!(agent_due_date(day(2999, 1, 1), None).is_err());
    }

    #[test]
    fn labels_follow_the_label_rules() {
        assert!(Labels::try_from(vec!["é".repeat(Labels::MAX_CHARS)]).is_ok());
        assert!(Labels::try_from(vec!["é".repeat(Labels::MAX_CHARS + 1)]).is_err());
        for bad in ["a,b", "a\nb", "a\tb", "a\u{2028}b", "a\u{0}b", "  "] {
            assert!(Labels::try_from(vec![bad.to_owned()]).is_err(), "{bad:?}");
        }
    }
}
