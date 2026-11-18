use capitainerie_common::{
    AgentId, CardId, ProjectId, RoutineId, RoutineRunId, RoutineTriggerId, RunId,
};
use capitainerie_core::domain::{
    card::{Column, attributes::Priority},
    routine::{
        IfMissed, IfOpen, RoutineListing, RoutineRun, RoutineRunStatus, RoutineRunView, RunOrigin,
        Trigger, TriggerKind,
        schedule::{Schedule, ScheduleSpec},
    },
    run::RunStatus,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

macro_rules! dto {
    ($dto:ident from $domain:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
        #[serde(rename_all = "snake_case")]
        pub enum $dto {
            $($variant),+
        }

        impl From<$domain> for $dto {
            fn from(value: $domain) -> Self {
                match value {
                    $($domain::$variant => Self::$variant),+
                }
            }
        }

        impl $dto {
            pub fn as_domain(self) -> $domain {
                match self {
                    $(Self::$variant => $domain::$variant),+
                }
            }
        }
    };
}

dto!(PriorityDto from Priority { Low, Normal, High, Urgent });
dto!(IfOpenDto from IfOpen { Skip, Queue, Always });
dto!(IfMissedDto from IfMissed { Skip, RunOnce, RunAll });
dto!(RunOriginDto from RunOrigin { Planning, Manual, Webhook });
dto!(RoutineRunStatusDto from RoutineRunStatus { Pending, Queued, Created, Skipped, Failed });
dto!(CardColumnDto from Column { Proposed, Todo, InProgress, Done, Rejected });
dto!(AgentRunStatusDto from RunStatus { Queued, Running, Succeeded, Failed, Cancelled });

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FrequencyDto {
    EveryHours,
    Daily,
    Weekly,
    Monthly,
    Cron,
}

impl FrequencyDto {
    fn as_str(self) -> &'static str {
        match self {
            Self::EveryHours => "every_hours",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Cron => "cron",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "every_hours" => Self::EveryHours,
            "daily" => Self::Daily,
            "weekly" => Self::Weekly,
            "monthly" => Self::Monthly,
            _ => Self::Cron,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TriggerKindDto {
    Planning,
    Webhook,
}

/// When a planning trigger fires, in the routine's time zone.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScheduleDto {
    pub frequency: FrequencyDto,
    /// `every_hours` only: 1 to 744.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hours: Option<u16>,
    /// `daily`, `weekly` and `monthly`: `HH:MM`, 08:00 by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    /// `weekly` only: `mon` … `sun`, at least one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub days: Vec<String>,
    /// `cron` only: five fields (minute hour day month weekday).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cron: Option<String>,
}

impl From<ScheduleDto> for ScheduleSpec {
    fn from(dto: ScheduleDto) -> Self {
        Self {
            frequency: dto.frequency.as_str().to_owned(),
            hours: dto.hours,
            time: dto.time,
            days: dto.days,
            cron: dto.cron,
        }
    }
}

impl From<&Schedule> for ScheduleDto {
    fn from(schedule: &Schedule) -> Self {
        let spec = schedule.spec();
        Self {
            frequency: FrequencyDto::parse(&spec.frequency),
            hours: spec.hours,
            time: spec.time,
            days: spec.days,
            cron: spec.cron,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TriggerResponse {
    pub id: RoutineTriggerId,
    pub kind: TriggerKindDto,
    pub enabled: bool,
    /// Planning triggers only.
    pub schedule: Option<ScheduleDto>,
    /// The next planned passage; null for a webhook, a disabled trigger or a paused routine.
    pub next_run_at: Option<DateTime<Utc>>,
    /// Webhook triggers only: the last characters of the token.
    pub token_hint: Option<String>,
}

impl From<Trigger> for TriggerResponse {
    fn from(trigger: Trigger) -> Self {
        let (kind, schedule, token_hint) = match &trigger.kind {
            TriggerKind::Planning(schedule) => {
                (TriggerKindDto::Planning, Some(schedule.into()), None)
            }
            TriggerKind::Webhook { token_hint, .. } => {
                (TriggerKindDto::Webhook, None, Some(token_hint.clone()))
            }
        };
        Self {
            id: trigger.id,
            kind,
            enabled: trigger.enabled,
            schedule,
            next_run_at: trigger.next_run_at,
            token_hint,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RunCountsResponse {
    /// Passages that created their card.
    pub created: i64,
    pub skipped: i64,
    pub failed: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoutineRunResponse {
    pub id: RoutineRunId,
    pub routine_id: RoutineId,
    pub trigger_id: Option<RoutineTriggerId>,
    pub origin: RunOriginDto,
    /// The planned passage, or the moment of a manual or webhook launch.
    pub scheduled_for: DateTime<Utc>,
    pub started_at: DateTime<Utc>,
    pub status: RoutineRunStatusDto,
    /// Why the passage was skipped or failed, or why the agent run did not start.
    pub reason: Option<String>,
    pub card_id: Option<CardId>,
    /// Where the card stands now.
    pub card_column: Option<CardColumnDto>,
    pub run_id: Option<RunId>,
    pub run_status: Option<AgentRunStatusDto>,
    pub run_started_at: Option<DateTime<Utc>>,
    pub run_finished_at: Option<DateTime<Utc>>,
}

impl From<RoutineRunView> for RoutineRunResponse {
    fn from(view: RoutineRunView) -> Self {
        Self {
            card_column: view.card_column.map(Into::into),
            run_status: view.agent_run_status.map(Into::into),
            run_started_at: view.agent_run_started_at,
            run_finished_at: view.agent_run_finished_at,
            ..RoutineRunResponse::from(view.run)
        }
    }
}

impl From<RoutineRun> for RoutineRunResponse {
    fn from(run: RoutineRun) -> Self {
        Self {
            id: run.id,
            routine_id: run.routine_id,
            trigger_id: run.trigger_id,
            origin: run.origin.into(),
            scheduled_for: run.scheduled_for,
            started_at: run.started_at,
            status: run.status.into(),
            reason: run.reason,
            card_id: run.card_id,
            card_column: None,
            run_id: run.run_id,
            run_status: None,
            run_started_at: None,
            run_finished_at: None,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoutineResponse {
    pub id: RoutineId,
    pub slug: String,
    pub name: String,
    /// The description of every card the routine creates.
    pub prompt: String,
    pub agent_id: AgentId,
    pub project_id: Option<ProjectId>,
    pub priority: PriorityDto,
    /// `{nom}` and `{date}` are replaced in each card's title.
    pub title_template: String,
    /// IANA time zone the schedules are read in.
    pub timezone: String,
    pub enabled: bool,
    pub if_open: IfOpenDto,
    pub if_missed: IfMissedDto,
    /// Whether the agent runs may use the tools that write.
    pub allow_writes: bool,
    pub archived: bool,
    /// The earliest planned passage; null while paused or without planning.
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_run: Option<RoutineRunResponse>,
    pub counts: RunCountsResponse,
    pub triggers: Vec<TriggerResponse>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<RoutineListing> for RoutineResponse {
    fn from(listing: RoutineListing) -> Self {
        let next_run_at = listing.next_run_at();
        let RoutineListing {
            routine,
            triggers,
            last_run,
            counts,
        } = listing;
        Self {
            id: routine.id,
            slug: routine.slug.as_str().to_owned(),
            name: routine.name,
            prompt: routine.prompt,
            agent_id: routine.agent_id,
            project_id: routine.project_id,
            priority: routine.priority.into(),
            title_template: routine.title_template.as_str().to_owned(),
            timezone: routine.timezone.name().to_owned(),
            enabled: routine.enabled,
            if_open: routine.if_open.into(),
            if_missed: routine.if_missed.into(),
            allow_writes: routine.allow_writes,
            archived: routine.archived,
            next_run_at,
            last_run_at: routine.last_run_at,
            last_run: last_run.map(Into::into),
            counts: RunCountsResponse {
                created: counts.created,
                skipped: counts.skipped,
                failed: counts.failed,
            },
            triggers: triggers.into_iter().map(Into::into).collect(),
            created_by: routine.created_by,
            created_at: routine.created_at,
            updated_at: routine.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoutineListResponse {
    pub items: Vec<RoutineResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoutineRunListResponse {
    pub items: Vec<RoutineRunResponse>,
    pub total: i64,
}

/// Returned when a trigger is added or its token rotated. A webhook's token
/// and URL are shown this once: only a hash of the token is kept.
#[derive(Debug, Serialize, ToSchema)]
pub struct IssuedTriggerResponse {
    pub routine: RoutineResponse,
    pub trigger_id: RoutineTriggerId,
    pub token: Option<String>,
    pub webhook_url: Option<String>,
}
