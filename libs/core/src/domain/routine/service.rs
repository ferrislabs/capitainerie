use capitainerie_common::{
    AgentId, CardId, CoreError, OrganizationId, ProjectId, RoutineId, RoutineTriggerId, RunId,
};
use capitainerie_events::routine::{RoutineRunCreated, RoutineRunSkipped};
use chrono::{DateTime, Utc};

use super::{
    IfMissed, IfOpen, IssuedTrigger, NAME_MAX_CHARS, PENDING_TIMEOUT, PROMPT_MAX_CHARS, Plan,
    Routine, RoutineListing, RoutineRun, RoutineRunStatus, RoutineRunView, RoutineToken, RunOrigin,
    Situation, Step, TitleTemplate, Trigger, TriggerKind, plan,
    ports::{RoutineRepository, RoutineRunRepository},
    schedule::{Schedule, ScheduleSpec, parse_timezone},
};
use crate::domain::{
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    card::{attributes::Priority, bounded_text, relations::check_agent},
    client::links::Change,
    crew::{Slug, ports::CrewRepository},
    event::ports::EventEmitter,
    label::Name,
    project::ports::ProjectRepository,
    source::TokenHash,
};

#[derive(Debug, Clone)]
pub struct CreateRoutine {
    /// Derived from the name when absent.
    pub slug: Option<String>,
    pub name: String,
    pub prompt: String,
    pub agent_id: AgentId,
    pub project_id: Option<ProjectId>,
    pub priority: Priority,
    pub title_template: Option<String>,
    pub timezone: Option<String>,
    pub enabled: bool,
    pub if_open: IfOpen,
    pub if_missed: IfMissed,
    pub allow_writes: bool,
    /// The planning trigger created with the routine, if any.
    pub schedule: Option<ScheduleSpec>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateRoutine {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub prompt: Option<String>,
    pub agent_id: Option<AgentId>,
    pub project_id: Change<ProjectId>,
    pub priority: Option<Priority>,
    pub title_template: Option<String>,
    pub timezone: Option<String>,
    pub enabled: Option<bool>,
    pub if_open: Option<IfOpen>,
    pub if_missed: Option<IfMissed>,
    pub allow_writes: Option<bool>,
    pub archived: Option<bool>,
}

impl UpdateRoutine {
    fn changes_nothing(&self) -> bool {
        self.slug.is_none()
            && self.name.is_none()
            && self.prompt.is_none()
            && self.agent_id.is_none()
            && matches!(self.project_id, Change::Keep)
            && self.priority.is_none()
            && self.title_template.is_none()
            && self.timezone.is_none()
            && self.enabled.is_none()
            && self.if_open.is_none()
            && self.if_missed.is_none()
            && self.allow_writes.is_none()
            && self.archived.is_none()
    }
}

#[derive(Debug, Clone)]
pub struct AddTrigger {
    /// `planning` or `webhook`.
    pub kind: String,
    pub enabled: bool,
    pub schedule: Option<ScheduleSpec>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateTrigger {
    pub enabled: Option<bool>,
    pub schedule: Option<ScheduleSpec>,
}

/// A passage whose card is to be created: the caller creates it and starts
/// the agent run, then records the outcome.
#[derive(Debug, Clone)]
pub struct Job {
    pub routine: Routine,
    pub run: RoutineRun,
}

/// What became of a job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobOutcome {
    Created {
        card_id: CardId,
        run_id: Option<RunId>,
        note: Option<String>,
    },
    Failed(String),
}

pub struct RoutineService<'a, R, A> {
    routines: R,
    authorizer: &'a A,
}

impl<'a, R, A> RoutineService<'a, R, A>
where
    R: RoutineRepository,
    A: Authorizer,
{
    pub fn new(routines: R, authorizer: &'a A) -> Self {
        Self {
            routines,
            authorizer,
        }
    }

    pub async fn list(
        mut self,
        subject: Subject,
        archived: Option<bool>,
    ) -> Result<Vec<RoutineListing>, CoreError> {
        let organization_id = subject.organization_id;
        require_routine(self.authorizer, subject, Action::RoutineRead, None).await?;
        self.routines.list(organization_id, archived).await
    }

    pub async fn get(
        mut self,
        subject: Subject,
        id: RoutineId,
    ) -> Result<RoutineListing, CoreError> {
        let organization_id = subject.organization_id;
        require_routine(self.authorizer, subject, Action::RoutineRead, Some(id)).await?;
        self.found(organization_id, id).await
    }

    pub async fn create<C, P>(
        mut self,
        subject: Subject,
        command: CreateRoutine,
        crew: &mut C,
        projects: &mut P,
        now: DateTime<Utc>,
    ) -> Result<RoutineListing, CoreError>
    where
        C: CrewRepository,
        P: ProjectRepository,
    {
        let organization_id = subject.organization_id;
        require_routine(
            self.authorizer,
            subject.clone(),
            Action::RoutineManage,
            None,
        )
        .await?;
        check_agent(crew, organization_id, command.agent_id).await?;
        if let Some(project) = command.project_id {
            check_project(projects, organization_id, project).await?;
        }
        let name = Name::try_from(command.name)?.as_str().to_owned();
        let slug = match command.slug {
            Some(slug) => Slug::try_from(slug)?,
            None => Slug::try_from(slugify(&name))?,
        };
        let schedule = command.schedule.map(Schedule::try_from).transpose()?;
        let routine = Routine {
            id: RoutineId::new(),
            organization_id,
            slug,
            name: bounded_name(name)?,
            prompt: prompt(command.prompt)?,
            agent_id: command.agent_id,
            project_id: command.project_id,
            priority: command.priority,
            title_template: command
                .title_template
                .map_or_else(|| Ok(TitleTemplate::default()), TitleTemplate::try_from)?,
            timezone: parse_timezone(command.timezone.as_deref().unwrap_or("Europe/Paris"))?,
            enabled: command.enabled,
            if_open: command.if_open,
            if_missed: command.if_missed,
            allow_writes: command.allow_writes,
            last_run_at: None,
            archived: false,
            created_by: subject.id,
            created_at: now,
            updated_at: now,
        };
        self.routines.insert(&routine).await?;
        if let Some(schedule) = schedule {
            let mut trigger = Trigger {
                id: RoutineTriggerId::new(),
                organization_id,
                routine_id: routine.id,
                enabled: true,
                kind: TriggerKind::Planning(schedule),
                next_run_at: None,
                created_at: now,
            };
            trigger.reschedule(&routine, now);
            self.routines.insert_trigger(&trigger).await?;
        }
        self.found(organization_id, routine.id).await
    }

    pub async fn update<C, P>(
        mut self,
        subject: Subject,
        id: RoutineId,
        command: UpdateRoutine,
        crew: &mut C,
        projects: &mut P,
        now: DateTime<Utc>,
    ) -> Result<RoutineListing, CoreError>
    where
        C: CrewRepository,
        P: ProjectRepository,
    {
        let organization_id = subject.organization_id;
        require_routine(self.authorizer, subject, Action::RoutineManage, Some(id)).await?;
        let RoutineListing {
            mut routine,
            triggers,
            ..
        } = self.found(organization_id, id).await?;
        if command.changes_nothing() {
            return Err(CoreError::Invalid(
                "nothing to update: give at least one field".into(),
            ));
        }
        let was_active = routine.is_active();
        let previous_timezone = routine.timezone;
        if let Some(slug) = command.slug {
            routine.slug = Slug::try_from(slug)?;
        }
        if let Some(name) = command.name {
            routine.name = bounded_name(Name::try_from(name)?.as_str().to_owned())?;
        }
        if let Some(value) = command.prompt {
            routine.prompt = prompt(value)?;
        }
        if let Some(agent) = command.agent_id {
            check_agent(crew, organization_id, agent).await?;
            routine.agent_id = agent;
        }
        match command.project_id {
            Change::Keep => {}
            Change::Clear => routine.project_id = None,
            Change::Set(project) => {
                check_project(projects, organization_id, project).await?;
                routine.project_id = Some(project);
            }
        }
        if let Some(priority) = command.priority {
            routine.priority = priority;
        }
        if let Some(template) = command.title_template {
            routine.title_template = TitleTemplate::try_from(template)?;
        }
        if let Some(timezone) = command.timezone {
            routine.timezone = parse_timezone(&timezone)?;
        }
        if let Some(enabled) = command.enabled {
            routine.enabled = enabled;
        }
        if let Some(policy) = command.if_open {
            routine.if_open = policy;
        }
        if let Some(policy) = command.if_missed {
            routine.if_missed = policy;
        }
        if let Some(allow_writes) = command.allow_writes {
            routine.allow_writes = allow_writes;
        }
        if let Some(archived) = command.archived {
            routine.archived = archived;
        }
        routine.updated_at = now;
        self.routines.update(&routine).await?;
        if routine.is_active() != was_active || routine.timezone != previous_timezone {
            for mut trigger in triggers {
                trigger.reschedule(&routine, now);
                self.routines.update_trigger(&trigger).await?;
            }
        }
        self.found(organization_id, id).await
    }

    pub async fn archive(
        mut self,
        subject: Subject,
        id: RoutineId,
        now: DateTime<Utc>,
    ) -> Result<RoutineListing, CoreError> {
        let organization_id = subject.organization_id;
        require_routine(self.authorizer, subject, Action::RoutineManage, Some(id)).await?;
        let RoutineListing {
            mut routine,
            triggers,
            ..
        } = self.found(organization_id, id).await?;
        if !routine.archived {
            routine.archived = true;
            routine.updated_at = now;
            self.routines.update(&routine).await?;
            for mut trigger in triggers {
                trigger.reschedule(&routine, now);
                self.routines.update_trigger(&trigger).await?;
            }
        }
        self.found(organization_id, id).await
    }

    pub async fn add_trigger(
        mut self,
        subject: Subject,
        id: RoutineId,
        command: AddTrigger,
        now: DateTime<Utc>,
    ) -> Result<IssuedTrigger, CoreError> {
        let organization_id = subject.organization_id;
        require_routine(self.authorizer, subject, Action::RoutineManage, Some(id)).await?;
        let routine = self.found(organization_id, id).await?.routine;
        let (kind, token) = match (command.kind.as_str(), command.schedule) {
            ("planning", Some(spec)) => (TriggerKind::Planning(Schedule::try_from(spec)?), None),
            ("planning", None) => {
                return Err(CoreError::Invalid(
                    "a planning trigger needs a schedule".into(),
                ));
            }
            ("webhook", None) => {
                let token = RoutineToken::generate()?;
                (
                    TriggerKind::Webhook {
                        token_hash: token.hash(),
                        token_hint: token.hint(),
                    },
                    Some(token),
                )
            }
            ("webhook", Some(_)) => {
                return Err(CoreError::Invalid(
                    "a webhook trigger takes no schedule".into(),
                ));
            }
            (other, _) => {
                return Err(CoreError::Invalid(format!(
                    "unknown trigger kind {other}: use planning or webhook"
                )));
            }
        };
        let mut trigger = Trigger {
            id: RoutineTriggerId::new(),
            organization_id,
            routine_id: id,
            enabled: command.enabled,
            kind,
            next_run_at: None,
            created_at: now,
        };
        trigger.reschedule(&routine, now);
        self.routines.insert_trigger(&trigger).await?;
        Ok(IssuedTrigger {
            listing: self.found(organization_id, id).await?,
            trigger_id: trigger.id,
            token,
        })
    }

    pub async fn update_trigger(
        mut self,
        subject: Subject,
        id: RoutineId,
        trigger_id: RoutineTriggerId,
        command: UpdateTrigger,
        now: DateTime<Utc>,
    ) -> Result<RoutineListing, CoreError> {
        let organization_id = subject.organization_id;
        require_routine(self.authorizer, subject, Action::RoutineManage, Some(id)).await?;
        let listing = self.found(organization_id, id).await?;
        let mut trigger = trigger_of(&listing, trigger_id)?;
        if command.enabled.is_none() && command.schedule.is_none() {
            return Err(CoreError::Invalid(
                "nothing to update: give enabled or schedule".into(),
            ));
        }
        if let Some(enabled) = command.enabled {
            trigger.enabled = enabled;
        }
        if let Some(spec) = command.schedule {
            match trigger.kind {
                TriggerKind::Planning(_) => {
                    trigger.kind = TriggerKind::Planning(Schedule::try_from(spec)?);
                }
                TriggerKind::Webhook { .. } => {
                    return Err(CoreError::Invalid(
                        "a webhook trigger takes no schedule".into(),
                    ));
                }
            }
        }
        trigger.reschedule(&listing.routine, now);
        self.routines.update_trigger(&trigger).await?;
        self.found(organization_id, id).await
    }

    pub async fn remove_trigger(
        mut self,
        subject: Subject,
        id: RoutineId,
        trigger_id: RoutineTriggerId,
    ) -> Result<RoutineListing, CoreError> {
        let organization_id = subject.organization_id;
        require_routine(self.authorizer, subject, Action::RoutineManage, Some(id)).await?;
        self.found(organization_id, id).await?;
        if !self
            .routines
            .delete_trigger(organization_id, id, trigger_id)
            .await?
        {
            return Err(CoreError::NotFound);
        }
        self.found(organization_id, id).await
    }

    /// Issues a new webhook token; the previous one stops working at once.
    pub async fn rotate_trigger_token(
        mut self,
        subject: Subject,
        id: RoutineId,
        trigger_id: RoutineTriggerId,
    ) -> Result<IssuedTrigger, CoreError> {
        let organization_id = subject.organization_id;
        require_routine(self.authorizer, subject, Action::RoutineManage, Some(id)).await?;
        let listing = self.found(organization_id, id).await?;
        let mut trigger = trigger_of(&listing, trigger_id)?;
        if !matches!(trigger.kind, TriggerKind::Webhook { .. }) {
            return Err(CoreError::Invalid(
                "only a webhook trigger has a token".into(),
            ));
        }
        let token = RoutineToken::generate()?;
        trigger.kind = TriggerKind::Webhook {
            token_hash: token.hash(),
            token_hint: token.hint(),
        };
        self.routines.update_trigger(&trigger).await?;
        Ok(IssuedTrigger {
            listing: self.found(organization_id, id).await?,
            trigger_id,
            token: Some(token),
        })
    }

    /// The routine a manual launch goes to: managing it is required.
    pub async fn for_launch(
        mut self,
        subject: Subject,
        id: RoutineId,
    ) -> Result<Routine, CoreError> {
        let organization_id = subject.organization_id;
        require_routine(self.authorizer, subject, Action::RoutineManage, Some(id)).await?;
        let routine = self.found(organization_id, id).await?.routine;
        if routine.archived {
            return Err(CoreError::Invalid("the routine is archived".into()));
        }
        Ok(routine)
    }

    /// The routine and webhook trigger a presented token designates, if both
    /// are enabled. Every other case is a `NotFound`.
    pub async fn by_token(mut self, token: &str) -> Result<(Routine, Trigger), CoreError> {
        if !RoutineToken::plausible(token) {
            return Err(CoreError::NotFound);
        }
        self.routines
            .find_by_token(&TokenHash::of(token))
            .await?
            .filter(|(routine, trigger)| routine.is_active() && trigger.enabled)
            .ok_or(CoreError::NotFound)
    }

    async fn found(
        &mut self,
        organization_id: OrganizationId,
        id: RoutineId,
    ) -> Result<RoutineListing, CoreError> {
        self.routines
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)
    }
}

/// Turns triggers into passages: claims what is due, applies the policies,
/// and records what became of each passage.
pub struct RoutineScheduler<'a, R, Q, E> {
    routines: R,
    runs: Q,
    emitter: &'a E,
}

impl<'a, R, Q, E> RoutineScheduler<'a, R, Q, E>
where
    R: RoutineRepository,
    Q: RoutineRunRepository,
    E: EventEmitter,
{
    pub fn new(routines: R, runs: Q, emitter: &'a E) -> Self {
        Self {
            routines,
            runs,
            emitter,
        }
    }

    /// Claims up to `limit` due triggers and returns the passages to run.
    /// Each claimed trigger is rescheduled in the same transaction, so a
    /// concurrent tick neither sees nor runs it.
    pub async fn claim(mut self, now: DateTime<Utc>, limit: i64) -> Result<Vec<Job>, CoreError> {
        self.runs
            .fail_stale(
                now - PENDING_TIMEOUT,
                "passage interrompu avant la création de la tâche",
            )
            .await?;
        let claimed = self.routines.claim_due(now, limit).await?;
        let mut jobs = Vec::with_capacity(claimed.len());
        for (routine, trigger) in claimed {
            let due = trigger.next_run_at.filter(|at| *at <= now);
            if let Triggered::Run(job) = self
                .advance(&routine, trigger, RunOrigin::Planning, due, now)
                .await?
            {
                jobs.push(*job);
            }
        }
        Ok(jobs)
    }

    /// A webhook call: a passage due now, under the routine's `if_open` policy.
    pub async fn trigger(
        mut self,
        routine: Routine,
        trigger: Trigger,
        now: DateTime<Utc>,
    ) -> Result<Triggered, CoreError> {
        self.advance(&routine, trigger, RunOrigin::Webhook, Some(now), now)
            .await
    }

    /// A manual launch: a passage now, whatever the policies say.
    pub async fn launch(mut self, routine: Routine, now: DateTime<Utc>) -> Result<Job, CoreError> {
        let run = RoutineRun::new(
            &routine,
            None,
            RunOrigin::Manual,
            now,
            now,
            RoutineRunStatus::Pending,
        );
        self.runs.insert(&run).await?;
        Ok(Job { routine, run })
    }

    pub async fn record(
        mut self,
        job: Job,
        outcome: JobOutcome,
        now: DateTime<Utc>,
    ) -> Result<RoutineRun, CoreError> {
        let Job { routine, mut run } = job;
        match outcome {
            JobOutcome::Created {
                card_id,
                run_id,
                note,
            } => {
                run.status = RoutineRunStatus::Created;
                run.card_id = Some(card_id);
                run.run_id = run_id;
                run = match note {
                    Some(note) => run.with_reason(note),
                    None => run,
                };
                self.runs.save(&run).await?;
                self.routines
                    .mark_ran(routine.organization_id, routine.id, now)
                    .await?;
                self.emitter.emit(&RoutineRunCreated {
                    routine_run_id: run.id.0,
                    routine_id: routine.id.0,
                    organization_id: routine.organization_id.0,
                    origin: run.origin.as_str().to_owned(),
                    scheduled_for: run.scheduled_for,
                    card_id: card_id.0,
                    run_id: run_id.map(|id| id.0),
                })?;
            }
            JobOutcome::Failed(reason) => {
                run.status = RoutineRunStatus::Failed;
                run = run.with_reason(reason);
                self.runs.save(&run).await?;
                self.routines
                    .mark_ran(routine.organization_id, routine.id, now)
                    .await?;
                self.announce_skipped(&run)?;
            }
        }
        Ok(run)
    }

    async fn advance(
        &mut self,
        routine: &Routine,
        mut trigger: Trigger,
        origin: RunOrigin,
        due: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> Result<Triggered, CoreError> {
        let queued = self.runs.queued(trigger.id).await?;
        let situation = Situation {
            due,
            queued: queued.is_some(),
            previous_open: self.runs.previous_card_open(routine.id).await?,
        };
        let Plan { step, next_run_at } = plan(routine, trigger.kind.schedule(), situation, now);
        let moves_on = due.is_some() && matches!(step, Step::Run | Step::Skip(_));
        if moves_on && trigger.kind.schedule().is_some() {
            trigger.next_run_at = next_run_at;
            self.routines.update_trigger(&trigger).await?;
        }
        let scheduled_for = due.unwrap_or(now);
        let fresh = |status| {
            RoutineRun::new(
                routine,
                Some(trigger.id),
                origin,
                scheduled_for,
                now,
                status,
            )
        };
        match step {
            Step::Wait => Ok(Triggered::Held(queued)),
            Step::Queue => {
                let run = fresh(RoutineRunStatus::Queued);
                self.runs.insert(&run).await?;
                Ok(Triggered::Held(Some(run)))
            }
            Step::Skip(reason) => {
                let run = match queued {
                    Some(mut run) => {
                        run.status = RoutineRunStatus::Skipped;
                        let run = run.with_reason(reason.text());
                        self.runs.save(&run).await?;
                        run
                    }
                    None => {
                        let run = fresh(RoutineRunStatus::Skipped).with_reason(reason.text());
                        self.runs.insert(&run).await?;
                        run
                    }
                };
                self.announce_skipped(&run)?;
                Ok(Triggered::Held(Some(run)))
            }
            Step::Run => {
                let run = match queued {
                    Some(mut run) => {
                        run.status = RoutineRunStatus::Pending;
                        run.started_at = now;
                        self.runs.save(&run).await?;
                        run
                    }
                    None => {
                        let run = fresh(RoutineRunStatus::Pending);
                        self.runs.insert(&run).await?;
                        run
                    }
                };
                Ok(Triggered::Run(Box::new(Job {
                    routine: routine.clone(),
                    run,
                })))
            }
        }
    }

    fn announce_skipped(&self, run: &RoutineRun) -> Result<(), CoreError> {
        self.emitter.emit(&RoutineRunSkipped {
            routine_run_id: run.id.0,
            routine_id: run.routine_id.0,
            organization_id: run.organization_id.0,
            origin: run.origin.as_str().to_owned(),
            scheduled_for: run.scheduled_for,
            status: run.status.as_str().to_owned(),
            reason: run.reason.clone().unwrap_or_default(),
        })
    }
}

/// What a claimed trigger or a webhook call produced.
#[derive(Debug, Clone)]
pub enum Triggered {
    Run(Box<Job>),
    /// Queued, still waiting, or skipped by a policy.
    Held(Option<RoutineRun>),
}

pub struct RoutineRunService<'a, Q, A> {
    runs: Q,
    authorizer: &'a A,
}

impl<'a, Q, A> RoutineRunService<'a, Q, A>
where
    Q: RoutineRunRepository,
    A: Authorizer,
{
    pub fn new(runs: Q, authorizer: &'a A) -> Self {
        Self { runs, authorizer }
    }

    /// The routine must be readable; the caller checks it exists.
    pub async fn list(
        mut self,
        subject: Subject,
        routine_id: RoutineId,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<RoutineRunView>, i64), CoreError> {
        let organization_id = subject.organization_id;
        require_routine(
            self.authorizer,
            subject,
            Action::RoutineRead,
            Some(routine_id),
        )
        .await?;
        self.runs
            .list(
                organization_id,
                routine_id,
                limit.clamp(1, 100),
                offset.max(0),
            )
            .await
    }
}

fn trigger_of(listing: &RoutineListing, id: RoutineTriggerId) -> Result<Trigger, CoreError> {
    listing
        .triggers
        .iter()
        .find(|trigger| trigger.id == id)
        .cloned()
        .ok_or(CoreError::NotFound)
}

async fn require_routine<A: Authorizer>(
    authorizer: &A,
    subject: Subject,
    action: Action,
    id: Option<RoutineId>,
) -> Result<(), CoreError> {
    require(
        authorizer,
        AccessRequest::new(subject, action, ResourceKind::Routine, id.map(|id| id.0)),
    )
    .await
}

async fn check_project<P: ProjectRepository>(
    projects: &mut P,
    organization_id: OrganizationId,
    id: ProjectId,
) -> Result<(), CoreError> {
    match projects.find(organization_id, id).await? {
        None => Err(CoreError::Invalid(
            "the project is not in the organization".into(),
        )),
        Some(project) if project.archived_at.is_some() => {
            Err(CoreError::Invalid("the project is archived".into()))
        }
        Some(_) => Ok(()),
    }
}

fn bounded_name(name: String) -> Result<String, CoreError> {
    bounded_text(name, NAME_MAX_CHARS, "name", false)
}

fn prompt(value: String) -> Result<String, CoreError> {
    bounded_text(value.trim().to_owned(), PROMPT_MAX_CHARS, "prompt", false)
}

/// A kebab-case slug from a name: accents dropped, other characters as dashes.
pub fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    for c in name.chars().flat_map(char::to_lowercase) {
        let folded = match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => "a",
            'é' | 'è' | 'ê' | 'ë' => "e",
            'î' | 'ï' | 'í' | 'ì' => "i",
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => "o",
            'ù' | 'û' | 'ü' | 'ú' => "u",
            'ç' => "c",
            'ñ' => "n",
            'ÿ' => "y",
            'œ' => "oe",
            'æ' => "ae",
            c if c.is_ascii_alphanumeric() => {
                slug.push(c);
                continue;
            }
            _ => "-",
        };
        slug.push_str(folded);
    }
    let joined = slug
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let cut: String = joined.chars().take(Slug::MAX_CHARS).collect();
    match cut.trim_end_matches('-') {
        "" => "routine".to_owned(),
        trimmed => trimmed.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slug_drops_accents_and_punctuation() {
        assert_eq!(
            slugify(" Revue des accès externes (Drive) "),
            "revue-des-acces-externes-drive"
        );
        assert_eq!(slugify("Notoriété : bilan & plan"), "notoriete-bilan-plan");
        assert_eq!(slugify("Œuvre"), "oeuvre");
        assert_eq!(slugify("¡¿!"), "routine");
        let long = slugify(&"mot ".repeat(40));
        assert!(long.len() <= Slug::MAX_CHARS && !long.ends_with('-'));
        assert!(Slug::try_from(long).is_ok());
    }
}
