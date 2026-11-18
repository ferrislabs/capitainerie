use std::str::FromStr;

use capitainerie_common::CoreError;
use chrono::{
    DateTime, Datelike, Days, Duration, LocalResult, Months, NaiveDate, NaiveDateTime, NaiveTime,
    TimeZone, Timelike, Utc, Weekday,
};
use chrono_tz::Tz;
use croner::{
    Cron,
    parser::{CronParser, Seconds, Year},
};

pub const MAX_EVERY_HOURS: u16 = 744;
const CRON_MAX_CHARS: usize = 120;
const DAILY_SEARCH_DAYS: u64 = 8;
const MONTHLY_SEARCH_MONTHS: u32 = 3;

/// When a planning trigger fires, in the routine's time zone.
#[derive(Debug, Clone)]
pub enum Schedule {
    /// Every N hours: on the hour first, then every N hours from the last passage.
    EveryHours(u16),
    Daily(NaiveTime),
    Weekly {
        at: NaiveTime,
        days: WeekdaySet,
    },
    /// The 1st of each month.
    Monthly(NaiveTime),
    Cron(Box<CronExpression>),
}

impl PartialEq for Schedule {
    fn eq(&self, other: &Self) -> bool {
        self.spec() == other.spec()
    }
}

impl Eq for Schedule {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WeekdaySet(u8);

impl WeekdaySet {
    const NAMES: [(&str, Weekday); 7] = [
        ("mon", Weekday::Mon),
        ("tue", Weekday::Tue),
        ("wed", Weekday::Wed),
        ("thu", Weekday::Thu),
        ("fri", Weekday::Fri),
        ("sat", Weekday::Sat),
        ("sun", Weekday::Sun),
    ];

    pub fn contains(self, day: Weekday) -> bool {
        self.0 & (1 << day.num_days_from_monday()) != 0
    }

    pub fn names(self) -> Vec<String> {
        Self::NAMES
            .iter()
            .filter(|(_, day)| self.contains(*day))
            .map(|(name, _)| (*name).to_owned())
            .collect()
    }

    pub fn parse(names: &[String]) -> Result<Self, CoreError> {
        let mut bits = 0u8;
        for name in names {
            let (_, day) = Self::NAMES
                .iter()
                .find(|(known, _)| *known == name.as_str())
                .ok_or_else(|| {
                    CoreError::Invalid(format!("unknown day {name}: use mon, tue … sun"))
                })?;
            bits |= 1 << day.num_days_from_monday();
        }
        if bits == 0 {
            return Err(CoreError::Invalid(
                "a weekly schedule needs at least one day".into(),
            ));
        }
        Ok(Self(bits))
    }
}

/// A five-field cron expression (minute hour day month weekday).
#[derive(Debug, Clone)]
pub struct CronExpression {
    text: String,
    cron: Cron,
}

impl CronExpression {
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl TryFrom<&str> for CronExpression {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let text = value.split_whitespace().collect::<Vec<_>>().join(" ");
        if text.is_empty() || text.chars().count() > CRON_MAX_CHARS {
            return Err(CoreError::Invalid(format!(
                "the cron expression must have 1 to {CRON_MAX_CHARS} characters"
            )));
        }
        let cron = CronParser::builder()
            .seconds(Seconds::Disallowed)
            .year(Year::Disallowed)
            .build()
            .parse(&text)
            .map_err(|error| CoreError::Invalid(format!("invalid cron expression: {error}")))?;
        Ok(Self { text, cron })
    }
}

/// The flat form of a schedule, as the API and the database carry it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScheduleSpec {
    /// `every_hours`, `daily`, `weekly`, `monthly` or `cron`.
    pub frequency: String,
    pub hours: Option<u16>,
    /// `HH:MM`.
    pub time: Option<String>,
    pub days: Vec<String>,
    pub cron: Option<String>,
}

impl Schedule {
    pub fn spec(&self) -> ScheduleSpec {
        let time = |at: &NaiveTime| Some(at.format("%H:%M").to_string());
        match self {
            Self::EveryHours(hours) => ScheduleSpec {
                frequency: "every_hours".into(),
                hours: Some(*hours),
                ..ScheduleSpec::default()
            },
            Self::Daily(at) => ScheduleSpec {
                frequency: "daily".into(),
                time: time(at),
                ..ScheduleSpec::default()
            },
            Self::Weekly { at, days } => ScheduleSpec {
                frequency: "weekly".into(),
                time: time(at),
                days: days.names(),
                ..ScheduleSpec::default()
            },
            Self::Monthly(at) => ScheduleSpec {
                frequency: "monthly".into(),
                time: time(at),
                ..ScheduleSpec::default()
            },
            Self::Cron(expression) => ScheduleSpec {
                frequency: "cron".into(),
                cron: Some(expression.as_str().to_owned()),
                ..ScheduleSpec::default()
            },
        }
    }

    /// The first passage strictly after `after`. An `every_hours` schedule
    /// counts from `anchor`, its previous passage, when there is one.
    pub fn next_after(
        &self,
        timezone: Tz,
        after: DateTime<Utc>,
        anchor: Option<DateTime<Utc>>,
    ) -> Option<DateTime<Utc>> {
        match self {
            Self::EveryHours(hours) => every_hours(*hours, timezone, after, anchor),
            Self::Daily(at) => next_matching_day(timezone, after, *at, |_| true),
            Self::Weekly { at, days } => {
                next_matching_day(timezone, after, *at, |date| days.contains(date.weekday()))
            }
            Self::Monthly(at) => next_first_of_month(timezone, after, *at),
            Self::Cron(expression) => expression
                .cron
                .find_next_occurrence(&after.with_timezone(&timezone), false)
                .ok()
                .map(|next| next.with_timezone(&Utc)),
        }
    }
}

impl TryFrom<ScheduleSpec> for Schedule {
    type Error = CoreError;

    fn try_from(spec: ScheduleSpec) -> Result<Self, Self::Error> {
        let at = || -> Result<NaiveTime, CoreError> {
            let text = spec.time.as_deref().unwrap_or("08:00");
            NaiveTime::parse_from_str(text, "%H:%M")
                .ok()
                .filter(|_| text.len() == 5)
                .ok_or_else(|| CoreError::Invalid(format!("invalid time {text}: use HH:MM")))
        };
        match spec.frequency.as_str() {
            "every_hours" => match spec.hours {
                Some(hours) if (1..=MAX_EVERY_HOURS).contains(&hours) => {
                    Ok(Self::EveryHours(hours))
                }
                _ => Err(CoreError::Invalid(format!(
                    "every_hours needs hours between 1 and {MAX_EVERY_HOURS}"
                ))),
            },
            "daily" => Ok(Self::Daily(at()?)),
            "weekly" => Ok(Self::Weekly {
                at: at()?,
                days: WeekdaySet::parse(&spec.days)?,
            }),
            "monthly" => Ok(Self::Monthly(at()?)),
            "cron" => Ok(Self::Cron(Box::new(CronExpression::try_from(
                spec.cron.as_deref().unwrap_or_default(),
            )?))),
            other => Err(CoreError::Invalid(format!(
                "unknown frequency {other}: use every_hours, daily, weekly, monthly or cron"
            ))),
        }
    }
}

/// An IANA time zone, such as `Europe/Paris`.
pub fn parse_timezone(value: &str) -> Result<Tz, CoreError> {
    Tz::from_str(value.trim()).map_err(|_| CoreError::Invalid(format!("unknown time zone {value}")))
}

fn every_hours(
    hours: u16,
    timezone: Tz,
    after: DateTime<Utc>,
    anchor: Option<DateTime<Utc>>,
) -> Option<DateTime<Utc>> {
    let step = Duration::hours(i64::from(hours));
    match anchor {
        Some(anchor) if anchor > after => Some(anchor),
        Some(anchor) => {
            let elapsed = (after - anchor).num_seconds();
            let periods = elapsed / step.num_seconds() + 1;
            Some(anchor + step * i32::try_from(periods).ok()?)
        }
        None => {
            let local = after.with_timezone(&timezone);
            let hour = local
                .with_minute(0)?
                .with_second(0)?
                .with_nanosecond(0)?
                .with_timezone(&Utc);
            Some(hour + Duration::hours(1))
        }
    }
}

fn next_matching_day(
    timezone: Tz,
    after: DateTime<Utc>,
    at: NaiveTime,
    matches: impl Fn(NaiveDate) -> bool,
) -> Option<DateTime<Utc>> {
    let start = after.with_timezone(&timezone).date_naive();
    (0..DAILY_SEARCH_DAYS)
        .filter_map(|offset| start.checked_add_days(Days::new(offset)))
        .filter(|date| matches(*date))
        .filter_map(|date| resolve(timezone, date.and_time(at)))
        .find(|candidate| *candidate > after)
}

fn next_first_of_month(timezone: Tz, after: DateTime<Utc>, at: NaiveTime) -> Option<DateTime<Utc>> {
    let local = after.with_timezone(&timezone).date_naive();
    let first = local.with_day(1)?;
    (0..MONTHLY_SEARCH_MONTHS)
        .filter_map(|offset| first.checked_add_months(Months::new(offset)))
        .filter_map(|date| resolve(timezone, date.and_time(at)))
        .find(|candidate| *candidate > after)
}

/// A local time made absolute: the earlier instant when the clock goes back,
/// the hour after when it falls in the gap of a clock going forward.
fn resolve(timezone: Tz, local: NaiveDateTime) -> Option<DateTime<Utc>> {
    match timezone.from_local_datetime(&local) {
        LocalResult::Single(at) => Some(at.with_timezone(&Utc)),
        LocalResult::Ambiguous(earlier, _) => Some(earlier.with_timezone(&Utc)),
        LocalResult::None => timezone
            .from_local_datetime(&(local + Duration::hours(1)))
            .earliest()
            .map(|at| at.with_timezone(&Utc)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARIS: Tz = chrono_tz::Europe::Paris;

    fn utc(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn spec(frequency: &str) -> ScheduleSpec {
        ScheduleSpec {
            frequency: frequency.into(),
            time: Some("08:00".into()),
            ..ScheduleSpec::default()
        }
    }

    fn schedule(spec: ScheduleSpec) -> Schedule {
        Schedule::try_from(spec).unwrap()
    }

    #[test]
    fn a_daily_passage_keeps_its_local_hour_across_the_dst_changes() {
        let daily = schedule(spec("daily"));
        assert_eq!(
            daily.next_after(PARIS, utc("2026-03-28T08:00:00Z"), None),
            Some(utc("2026-03-29T06:00:00Z"))
        );
        assert_eq!(
            daily.next_after(PARIS, utc("2026-10-24T07:00:00Z"), None),
            Some(utc("2026-10-25T07:00:00Z"))
        );
        assert_eq!(
            daily.next_after(PARIS, utc("2026-11-18T06:59:00Z"), None),
            Some(utc("2026-11-18T07:00:00Z"))
        );
        assert_eq!(
            daily.next_after(PARIS, utc("2026-11-18T07:00:00Z"), None),
            Some(utc("2026-11-19T07:00:00Z"))
        );
    }

    #[test]
    fn a_time_in_the_spring_gap_moves_an_hour_later_and_a_repeated_one_runs_once() {
        let mut gap = spec("daily");
        gap.time = Some("02:30".into());
        assert_eq!(
            schedule(gap).next_after(PARIS, utc("2026-03-28T12:00:00Z"), None),
            Some(utc("2026-03-29T01:30:00Z"))
        );
        let mut repeated = spec("daily");
        repeated.time = Some("02:30".into());
        let repeated = schedule(repeated);
        let first = repeated
            .next_after(PARIS, utc("2026-10-24T12:00:00Z"), None)
            .unwrap();
        assert_eq!(first, utc("2026-10-25T00:30:00Z"));
        assert_eq!(
            repeated.next_after(PARIS, first, None),
            Some(utc("2026-10-26T01:30:00Z"))
        );
    }

    #[test]
    fn a_weekly_schedule_fires_on_its_days_only() {
        let mut weekly = spec("weekly");
        weekly.days = vec!["mon".into(), "thu".into()];
        let weekly = schedule(weekly);
        assert_eq!(
            weekly.next_after(PARIS, utc("2026-11-18T10:00:00Z"), None),
            Some(utc("2026-11-19T07:00:00Z"))
        );
        assert_eq!(
            weekly.next_after(PARIS, utc("2026-11-19T07:00:00Z"), None),
            Some(utc("2026-11-23T07:00:00Z"))
        );
        assert_eq!(weekly.spec().days, ["mon", "thu"]);
    }

    #[test]
    fn a_monthly_schedule_fires_on_the_first() {
        let monthly = schedule(spec("monthly"));
        assert_eq!(
            monthly.next_after(PARIS, utc("2026-11-18T10:00:00Z"), None),
            Some(utc("2026-12-01T07:00:00Z"))
        );
        assert_eq!(
            monthly.next_after(PARIS, utc("2026-03-15T10:00:00Z"), None),
            Some(utc("2026-04-01T06:00:00Z"))
        );
    }

    #[test]
    fn every_n_hours_starts_on_the_hour_then_counts_from_the_last_passage() {
        let every = schedule(ScheduleSpec {
            frequency: "every_hours".into(),
            hours: Some(48),
            ..ScheduleSpec::default()
        });
        let now = utc("2026-11-18T10:17:00Z");
        assert_eq!(
            every.next_after(PARIS, now, None),
            Some(utc("2026-11-18T11:00:00Z"))
        );
        let anchor = utc("2026-11-10T09:00:00Z");
        assert_eq!(
            every.next_after(PARIS, now, Some(anchor)),
            Some(utc("2026-11-20T09:00:00Z"))
        );
        assert_eq!(
            every.next_after(PARIS, anchor, Some(anchor)),
            Some(utc("2026-11-12T09:00:00Z"))
        );
    }

    #[test]
    fn a_cron_expression_is_read_in_the_time_zone() {
        let cron = schedule(ScheduleSpec {
            frequency: "cron".into(),
            cron: Some(" 30  6 * * 1-5 ".into()),
            ..ScheduleSpec::default()
        });
        assert_eq!(cron.spec().cron.as_deref(), Some("30 6 * * 1-5"));
        assert_eq!(
            cron.next_after(PARIS, utc("2026-11-20T06:00:00Z"), None),
            Some(utc("2026-11-23T05:30:00Z"))
        );
        assert_eq!(
            cron.next_after(PARIS, utc("2026-03-27T06:00:00Z"), None),
            Some(utc("2026-03-30T04:30:00Z"))
        );
    }

    #[test]
    fn invalid_schedules_are_refused() {
        let invalid = [
            ScheduleSpec {
                frequency: "hourly".into(),
                ..ScheduleSpec::default()
            },
            ScheduleSpec {
                frequency: "every_hours".into(),
                hours: Some(0),
                ..ScheduleSpec::default()
            },
            ScheduleSpec {
                frequency: "daily".into(),
                time: Some("25:00".into()),
                ..ScheduleSpec::default()
            },
            ScheduleSpec {
                frequency: "daily".into(),
                time: Some("8:00".into()),
                ..ScheduleSpec::default()
            },
            spec("weekly"),
            ScheduleSpec {
                frequency: "weekly".into(),
                days: vec!["lun".into()],
                ..ScheduleSpec::default()
            },
            ScheduleSpec {
                frequency: "cron".into(),
                cron: Some("61 * * * *".into()),
                ..ScheduleSpec::default()
            },
            ScheduleSpec {
                frequency: "cron".into(),
                cron: Some("0 0 6 * * *".into()),
                ..ScheduleSpec::default()
            },
            ScheduleSpec {
                frequency: "cron".into(),
                ..ScheduleSpec::default()
            },
        ];
        for spec in invalid {
            assert!(
                matches!(Schedule::try_from(spec.clone()), Err(CoreError::Invalid(_))),
                "{spec:?}"
            );
        }
        assert!(parse_timezone("Europe/Paris").is_ok());
        assert!(parse_timezone("Mars/Olympus").is_err());
    }

    #[test]
    fn a_spec_round_trips() {
        let mut weekly = spec("weekly");
        weekly.days = vec!["fri".into(), "mon".into()];
        let parsed = schedule(weekly);
        assert_eq!(schedule(parsed.spec()), parsed);
        assert_eq!(parsed.spec().days, ["mon", "fri"]);
        assert_eq!(parsed.spec().time.as_deref(), Some("08:00"));
    }
}
