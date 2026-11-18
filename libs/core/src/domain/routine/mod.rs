pub mod ports;
pub mod schedule;
pub mod service;

use aws_lc_rs::rand::{SecureRandom, SystemRandom};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use capitainerie_common::{
    AgentId, CardId, CoreError, OrganizationId, ProjectId, RoutineId, RoutineRunId,
    RoutineTriggerId, RunId,
};
use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;

use self::schedule::Schedule;
use crate::domain::{
    card::{CardTitle, Column, attributes::Priority, bounded_text},
    crew::Slug,
    run::RunStatus,
    source::TokenHash,
};

pub const NAME_MAX_CHARS: usize = 120;
pub const PROMPT_MAX_CHARS: usize = 20_000;
pub const REASON_MAX_CHARS: usize = 2_000;
pub const TITLE_TEMPLATE_MAX_CHARS: usize = 200;
pub const DEFAULT_TITLE_TEMPLATE: &str = "{nom} · {date}";
/// A planned passage older than this when the scheduler reaches it was missed.
pub const MISSED_AFTER: Duration = Duration::minutes(10);
/// A passage still `pending` after this was interrupted (worker stopped).
pub const PENDING_TIMEOUT: Duration = Duration::minutes(15);

/// Recurring work an agent does: each passage creates a card assigned to
/// the agent and starts its run on behalf of the routine's creator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Routine {
    pub id: RoutineId,
    pub organization_id: OrganizationId,
    pub slug: Slug,
    pub name: String,
    pub prompt: String,
    pub agent_id: AgentId,
    pub project_id: Option<ProjectId>,
    pub priority: Priority,
    pub title_template: TitleTemplate,
    pub timezone: Tz,
    pub enabled: bool,
    pub if_open: IfOpen,
    pub if_missed: IfMissed,
    pub allow_writes: bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub archived: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Routine {
    pub fn is_active(&self) -> bool {
        self.enabled && !self.archived
    }

    /// The title of the card of a passage planned for `at`.
    pub fn card_title(&self, at: DateTime<Utc>) -> String {
        let date = at
            .with_timezone(&self.timezone)
            .format("%d/%m/%Y")
            .to_string();
        let title = self
            .title_template
            .as_str()
            .replace("{nom}", &self.name)
            .replace("{name}", &self.name)
            .replace("{date}", &date);
        let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
        title.chars().take(CardTitle::MAX_CHARS).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleTemplate(String);

impl TitleTemplate {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TitleTemplate {
    fn default() -> Self {
        Self(DEFAULT_TITLE_TEMPLATE.to_owned())
    }
}

impl TryFrom<String> for TitleTemplate {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        bounded_text(
            value.trim().to_owned(),
            TITLE_TEMPLATE_MAX_CHARS,
            "title template",
            false,
        )
        .map(Self)
    }
}

macro_rules! text_enum {
    ($name:ident { $($variant:ident => $text:literal),+ $(,)? }, $what:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = CoreError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::ALL
                    .iter()
                    .copied()
                    .find(|known| known.as_str() == value)
                    .ok_or_else(|| CoreError::Invalid(format!("unknown {} {value}", $what)))
            }
        }
    };
}

text_enum!(IfOpen { Skip => "skip", Queue => "queue", Always => "always" }, "if_open policy");
text_enum!(IfMissed { Skip => "skip", RunOnce => "run_once", RunAll => "run_all" }, "if_missed policy");
text_enum!(RunOrigin { Planning => "planning", Manual => "manual", Webhook => "webhook" }, "run origin");
text_enum!(
    RoutineRunStatus {
        Pending => "pending",
        Queued => "queued",
        Created => "created",
        Skipped => "skipped",
        Failed => "failed",
    },
    "routine run status"
);

/// What starts a routine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trigger {
    pub id: RoutineTriggerId,
    pub organization_id: OrganizationId,
    pub routine_id: RoutineId,
    pub enabled: bool,
    pub kind: TriggerKind,
    /// The next planned passage; always `None` for a webhook.
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerKind {
    Planning(Schedule),
    Webhook {
        token_hash: TokenHash,
        /// The last characters of the token, to recognise it.
        token_hint: String,
    },
}

impl TriggerKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planning(_) => "planning",
            Self::Webhook { .. } => "webhook",
        }
    }

    pub fn schedule(&self) -> Option<&Schedule> {
        match self {
            Self::Planning(schedule) => Some(schedule),
            Self::Webhook { .. } => None,
        }
    }
}

impl Trigger {
    /// Plans the next passage from `now`, or clears it when nothing is planned.
    pub fn reschedule(&mut self, routine: &Routine, now: DateTime<Utc>) {
        self.next_run_at = match &self.kind {
            TriggerKind::Planning(schedule) if self.enabled && routine.is_active() => {
                schedule.next_after(routine.timezone, now, None)
            }
            _ => None,
        };
    }
}

/// The secret part of a routine's webhook URL, shown once.
#[derive(Clone, PartialEq, Eq)]
pub struct RoutineToken(String);

impl std::fmt::Debug for RoutineToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RoutineToken(***)")
    }
}

impl RoutineToken {
    const PREFIX: &str = "rtn_";
    const RANDOM_BYTES: usize = 32;
    const HINT_CHARS: usize = 4;
    pub const MAX_CHARS: usize = 64;

    pub fn generate() -> Result<Self, CoreError> {
        let mut bytes = [0u8; Self::RANDOM_BYTES];
        SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| CoreError::Internal("the system random source failed".into()))?;
        Ok(Self(format!(
            "{}{}",
            Self::PREFIX,
            URL_SAFE_NO_PAD.encode(bytes)
        )))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn hash(&self) -> TokenHash {
        TokenHash::of(&self.0)
    }

    pub fn hint(&self) -> String {
        let count = self.0.chars().count();
        self.0
            .chars()
            .skip(count.saturating_sub(Self::HINT_CHARS))
            .collect()
    }

    pub fn plausible(token: &str) -> bool {
        token.starts_with(Self::PREFIX)
            && token.len() <= Self::MAX_CHARS
            && token[Self::PREFIX.len()..]
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    }
}

/// One passage of a routine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutineRun {
    pub id: RoutineRunId,
    pub organization_id: OrganizationId,
    pub routine_id: RoutineId,
    pub trigger_id: Option<RoutineTriggerId>,
    pub origin: RunOrigin,
    pub scheduled_for: DateTime<Utc>,
    pub started_at: DateTime<Utc>,
    pub status: RoutineRunStatus,
    pub card_id: Option<CardId>,
    pub run_id: Option<RunId>,
    /// Why the passage was skipped or failed, or why the agent run did not start.
    pub reason: Option<String>,
}

impl RoutineRun {
    pub fn new(
        routine: &Routine,
        trigger_id: Option<RoutineTriggerId>,
        origin: RunOrigin,
        scheduled_for: DateTime<Utc>,
        now: DateTime<Utc>,
        status: RoutineRunStatus,
    ) -> Self {
        Self {
            id: RoutineRunId::new(),
            organization_id: routine.organization_id,
            routine_id: routine.id,
            trigger_id,
            origin,
            scheduled_for,
            started_at: now,
            status,
            card_id: None,
            run_id: None,
            reason: None,
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(
            reason
                .into()
                .chars()
                .take(REASON_MAX_CHARS)
                .collect::<String>(),
        );
        self
    }
}

/// A passage with what became of its card and its agent run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutineRunView {
    pub run: RoutineRun,
    pub card_column: Option<Column>,
    pub agent_run_status: Option<RunStatus>,
    pub agent_run_started_at: Option<DateTime<Utc>>,
    pub agent_run_finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RunCounts {
    pub created: i64,
    pub skipped: i64,
    pub failed: i64,
}

/// A routine with its triggers and recent activity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutineListing {
    pub routine: Routine,
    pub triggers: Vec<Trigger>,
    pub last_run: Option<RoutineRunView>,
    pub counts: RunCounts,
}

impl RoutineListing {
    /// The earliest planned passage, none while the routine is paused.
    pub fn next_run_at(&self) -> Option<DateTime<Utc>> {
        if !self.routine.is_active() {
            return None;
        }
        self.triggers
            .iter()
            .filter(|trigger| trigger.enabled)
            .filter_map(|trigger| trigger.next_run_at)
            .min()
    }
}

/// A routine with the webhook token just issued for one of its triggers.
#[derive(Debug, Clone)]
pub struct IssuedTrigger {
    pub listing: RoutineListing,
    pub trigger_id: RoutineTriggerId,
    pub token: Option<RoutineToken>,
}

/// What the scheduler knows about a trigger it claimed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Situation {
    /// The trigger's planned passage, when it is due.
    pub due: Option<DateTime<Utc>>,
    /// A passage waits for the previous card to close.
    pub queued: bool,
    /// The routine's last card is neither done nor rejected.
    pub previous_open: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    Missed,
    PreviousOpen,
}

impl SkipReason {
    pub fn text(self) -> &'static str {
        match self {
            Self::Missed => "passage manqué : la routine ignore les passages manqués",
            Self::PreviousOpen => "la tâche précédente est encore ouverte",
        }
    }
}

/// What to do with a claimed trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Nothing changes: the queued passage keeps waiting.
    Wait,
    /// Record a queued passage that waits for the previous card to close.
    Queue,
    Skip(SkipReason),
    Run,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    pub step: Step,
    /// The trigger's new next passage; `None` keeps it.
    pub next_run_at: Option<DateTime<Utc>>,
}

/// Applies the routine's policies to a claimed trigger.
///
/// `if_open` comes first: `skip` drops the passage, `queue` holds it until
/// the previous card closes, `always` ignores the open card. A released
/// queued passage runs whatever `if_missed` says. Otherwise a due passage
/// older than [`MISSED_AFTER`] follows `if_missed`: `skip` drops it,
/// `run_once` runs it and resumes from now, `run_all` runs it and moves to
/// the next passage after it, catching up one passage per tick.
pub fn plan(
    routine: &Routine,
    schedule: Option<&Schedule>,
    situation: Situation,
    now: DateTime<Utc>,
) -> Plan {
    let next_from = |after: DateTime<Utc>| {
        situation.due.and_then(|due| {
            schedule.and_then(|schedule| schedule.next_after(routine.timezone, after, Some(due)))
        })
    };
    if situation.previous_open {
        match routine.if_open {
            IfOpen::Skip => {
                return Plan {
                    step: Step::Skip(SkipReason::PreviousOpen),
                    next_run_at: next_from(now),
                };
            }
            IfOpen::Queue => {
                return Plan {
                    step: if situation.queued {
                        Step::Wait
                    } else {
                        Step::Queue
                    },
                    next_run_at: None,
                };
            }
            IfOpen::Always => {}
        }
    }
    if situation.queued {
        return Plan {
            step: Step::Run,
            next_run_at: next_from(now),
        };
    }
    let Some(due) = situation.due else {
        return Plan {
            step: Step::Wait,
            next_run_at: None,
        };
    };
    let missed = due < now - MISSED_AFTER;
    match (missed, routine.if_missed) {
        (true, IfMissed::Skip) => Plan {
            step: Step::Skip(SkipReason::Missed),
            next_run_at: next_from(now),
        },
        (true, IfMissed::RunAll) => Plan {
            step: Step::Run,
            next_run_at: next_from(due),
        },
        _ => Plan {
            step: Step::Run,
            next_run_at: next_from(now),
        },
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        schedule::{ScheduleSpec, parse_timezone},
        *,
    };

    pub(crate) fn routine(organization_id: OrganizationId) -> Routine {
        let now = Utc::now();
        Routine {
            id: RoutineId::new(),
            organization_id,
            slug: Slug::try_from("revue".to_owned()).unwrap(),
            name: "Revue hebdomadaire".into(),
            prompt: "Fais la revue.".into(),
            agent_id: AgentId::new(),
            project_id: None,
            priority: Priority::Normal,
            title_template: TitleTemplate::default(),
            timezone: parse_timezone("Europe/Paris").unwrap(),
            enabled: true,
            if_open: IfOpen::Always,
            if_missed: IfMissed::RunOnce,
            allow_writes: false,
            last_run_at: None,
            archived: false,
            created_by: "user-1".into(),
            created_at: now,
            updated_at: now,
        }
    }

    fn utc(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn daily() -> Schedule {
        Schedule::try_from(ScheduleSpec {
            frequency: "daily".into(),
            time: Some("08:00".into()),
            ..ScheduleSpec::default()
        })
        .unwrap()
    }

    fn situation(due: &str) -> Situation {
        Situation {
            due: Some(utc(due)),
            queued: false,
            previous_open: false,
        }
    }

    const NOW: &str = "2026-11-18T07:00:30Z";

    #[test]
    fn a_due_passage_runs_and_moves_to_the_next_day() {
        let plan = plan(
            &routine(OrganizationId::new()),
            Some(&daily()),
            situation("2026-11-18T07:00:00Z"),
            utc(NOW),
        );
        assert_eq!(plan.step, Step::Run);
        assert_eq!(plan.next_run_at, Some(utc("2026-11-19T07:00:00Z")));
    }

    #[test]
    fn a_missed_passage_follows_if_missed() {
        let schedule = daily();
        let mut routine = routine(OrganizationId::new());
        let late = situation("2026-11-15T07:00:00Z");

        routine.if_missed = IfMissed::Skip;
        let skipped = plan(&routine, Some(&schedule), late, utc(NOW));
        assert_eq!(skipped.step, Step::Skip(SkipReason::Missed));
        assert_eq!(skipped.next_run_at, Some(utc("2026-11-19T07:00:00Z")));

        routine.if_missed = IfMissed::RunOnce;
        let once = plan(&routine, Some(&schedule), late, utc(NOW));
        assert_eq!(once.step, Step::Run);
        assert_eq!(once.next_run_at, Some(utc("2026-11-19T07:00:00Z")));

        routine.if_missed = IfMissed::RunAll;
        let all = plan(&routine, Some(&schedule), late, utc(NOW));
        assert_eq!(all.step, Step::Run);
        assert_eq!(all.next_run_at, Some(utc("2026-11-16T07:00:00Z")));
    }

    #[test]
    fn a_passage_a_few_minutes_late_is_not_missed() {
        let mut routine = routine(OrganizationId::new());
        routine.if_missed = IfMissed::Skip;
        let plan = plan(
            &routine,
            Some(&daily()),
            situation("2026-11-18T06:55:00Z"),
            utc(NOW),
        );
        assert_eq!(plan.step, Step::Run);
    }

    #[test]
    fn an_open_previous_card_follows_if_open() {
        let schedule = daily();
        let mut routine = routine(OrganizationId::new());
        let open = Situation {
            previous_open: true,
            ..situation("2026-11-18T07:00:00Z")
        };

        routine.if_open = IfOpen::Skip;
        let skipped = plan(&routine, Some(&schedule), open, utc(NOW));
        assert_eq!(skipped.step, Step::Skip(SkipReason::PreviousOpen));
        assert_eq!(skipped.next_run_at, Some(utc("2026-11-19T07:00:00Z")));

        routine.if_open = IfOpen::Queue;
        let queued = plan(&routine, Some(&schedule), open, utc(NOW));
        assert_eq!(queued.step, Step::Queue);
        assert_eq!(queued.next_run_at, None);
        let waiting = plan(
            &routine,
            Some(&schedule),
            Situation {
                queued: true,
                ..open
            },
            utc(NOW),
        );
        assert_eq!(waiting.step, Step::Wait);

        routine.if_open = IfOpen::Always;
        assert_eq!(
            plan(&routine, Some(&schedule), open, utc(NOW)).step,
            Step::Run
        );
    }

    #[test]
    fn a_released_queued_passage_runs_even_when_missed() {
        let mut routine = routine(OrganizationId::new());
        routine.if_open = IfOpen::Queue;
        routine.if_missed = IfMissed::Skip;
        let released = plan(
            &routine,
            Some(&daily()),
            Situation {
                due: Some(utc("2026-11-10T07:00:00Z")),
                queued: true,
                previous_open: false,
            },
            utc(NOW),
        );
        assert_eq!(released.step, Step::Run);
        assert_eq!(released.next_run_at, Some(utc("2026-11-19T07:00:00Z")));

        let webhook = plan(
            &routine,
            None,
            Situation {
                due: None,
                queued: true,
                previous_open: false,
            },
            utc(NOW),
        );
        assert_eq!(webhook.step, Step::Run);
        assert_eq!(webhook.next_run_at, None);
    }

    #[test]
    fn the_card_title_fills_the_template_in_the_time_zone() {
        let routine = routine(OrganizationId::new());
        assert_eq!(
            routine.card_title(utc("2026-11-18T23:30:00Z")),
            "Revue hebdomadaire · 19/11/2026"
        );
    }

    #[test]
    fn policies_round_trip_and_tokens_hide_themselves() {
        for policy in IfOpen::ALL {
            assert_eq!(IfOpen::try_from(policy.as_str()).unwrap(), *policy);
        }
        for policy in IfMissed::ALL {
            assert_eq!(IfMissed::try_from(policy.as_str()).unwrap(), *policy);
        }
        assert!(IfOpen::try_from("stack").is_err());
        let token = RoutineToken::generate().unwrap();
        assert!(RoutineToken::plausible(token.expose()));
        assert!(!RoutineToken::plausible("src_abc"));
        assert_eq!(token.hint().len(), 4);
        assert!(token.expose().ends_with(&token.hint()));
        assert!(!format!("{token:?}").contains(token.expose()));
    }
}
