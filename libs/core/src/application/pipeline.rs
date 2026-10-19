use std::time::{Duration, Instant};

use capitainerie_common::{CardId, CoreError, OrganizationId, RunId, StepId};
use capitainerie_events::run::{RunFinished, RunStarted, StepFinished};
use capitainerie_macros::transactional;
use chrono::{DateTime, Utc};
use serde_json::json;
use tracing::Instrument;

use super::{CapitainerieUseCase, RunCardLinks};
use crate::{
    domain::{
        agent::{DuplicateAction, LocatedAction, SecretToken, WriteRequest, ports::AgentRuntime},
        card::{Card, attributes::CardFilter, ports::CardRepository},
        client::catalog::{Catalog, Shown},
        delegation::ports::{DelegationVault, TokenExchanger},
        event::ports::EventEmitter,
        organization::ports::OrganizationRepository,
        provider::{
            AgentRole, ModelAssignments, ModelRef, ModelTarget, ProviderAccess, ProviderKind,
        },
        run::{
            Run, RunStatus, StepKind,
            locate::locate,
            ports::{RunRepository, StepRepository},
            step::{Pricing, Step, StepStatus, WriteOutput},
        },
        similarity::{
            CardVector, DecidedBy, DedupOutput, DuplicateMatch, JudgedPair, Triage, action_text,
            card_text,
            ports::{Embedder, VectorStore},
            triage,
        },
        transcript::{Transcript, ports::TranscriptRepository},
    },
    infrastructure::delegation::{PgDelegationVault, is_expired, purge_delegations},
};

/// Everything a worker needs to execute a run it has claimed.
#[derive(Debug, Clone)]
pub struct RunContext {
    pub run: Run,
    pub transcript: Transcript,
    pub issuer_url: String,
    pub steps: Vec<Step>,
    pub resumed: bool,
}

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Client id of the writer agent: whose delegation is used and who `act`s.
    pub writer_agent: String,
    pub pricing: Pricing,
    pub max_attempts: u32,
    /// A running run whose heartbeat is older is considered abandoned.
    pub stale_after: Duration,
    /// Cosine similarity from which an action is the same task as a card.
    pub dedup_threshold: f32,
    pub dedup_gray_floor: f32,
    pub default_models: ModelAssignments,
}

impl CapitainerieUseCase {
    /// Takes a run for this worker: a queued one starts, an abandoned one
    /// resumes. `None` when the run is finished, missing, or alive elsewhere.
    #[transactional(run, step, transcript, organization, emitter)]
    pub async fn claim_run(
        &self,
        id: RunId,
        stale_before: DateTime<Utc>,
        trace_id: Option<String>,
    ) -> Result<Option<RunContext>, CoreError> {
        let mut run_repository = run_repository;
        let mut step_repository = step_repository;
        let mut transcript_repository = transcript_repository;
        let mut organization_repository = organization_repository;
        let Some(mut run) = run_repository.lock(id).await? else {
            return Ok(None);
        };
        let now = Utc::now();
        let Some(transcript_id) = run.subject.transcript_id() else {
            return Ok(None);
        };
        let heartbeat = match run.status {
            RunStatus::Running => run_repository.heartbeat_at(id).await?,
            _ => None,
        };
        let Some(resumed) = claim_decision(run.status, heartbeat, stale_before) else {
            return Ok(None);
        };
        if !resumed {
            run.start(now)?;
        }
        if trace_id.is_some() {
            run.trace_id = trace_id;
        }
        run_repository.save(&run, Some(now)).await?;

        let transcript = transcript_repository
            .find(run.organization_id, transcript_id)
            .await?
            .ok_or_else(|| CoreError::Internal(format!("run {id} has no transcript")))?;
        let organization = organization_repository
            .find(run.organization_id)
            .await?
            .ok_or_else(|| CoreError::Internal(format!("run {id} has no organization")))?;
        let steps = step_repository.list(id).await?;
        emitter.emit(&RunStarted {
            run_id: id.0,
            organization_id: run.organization_id.0,
            resumed,
        })?;
        Ok(Some(RunContext {
            run,
            transcript,
            issuer_url: organization.issuer_url,
            steps,
            resumed,
        }))
    }

    /// Records a step and keeps the run's heartbeat fresh.
    #[transactional(run, step, emitter)]
    pub async fn record_step(&self, run: Run, step: Step) -> Result<(), CoreError> {
        let mut run_repository = run_repository;
        let mut step_repository = step_repository;
        step_repository.save(&step).await?;
        run_repository.save(&run, Some(Utc::now())).await?;
        if step.status != StepStatus::Running {
            emitter.emit(&StepFinished {
                run_id: step.run_id.0,
                organization_id: step.organization_id.0,
                seq: step.seq,
                kind: step.kind.as_str().to_owned(),
                status: step.status.as_str().to_owned(),
                agent: step.agent.clone(),
                model: (!step.usage.model.is_empty()).then(|| step.usage.model.clone()),
                input_tokens: step.usage.input_tokens,
                output_tokens: step.usage.output_tokens,
                cost_micros: step.cost_micros,
                latency_ms: step.latency_ms,
            })?;
        }
        Ok(())
    }

    /// Ends the run and destroys its delegations: nothing can act for the user
    /// once the run is over.
    #[transactional(run, emitter)]
    pub async fn finish_run(
        &self,
        mut run: Run,
        outcome: Result<(), String>,
    ) -> Result<(), CoreError> {
        let mut run_repository = run_repository;
        let now = Utc::now();
        match &outcome {
            Ok(()) => run.succeed(now)?,
            Err(reason) => run.fail(reason.clone(), now)?,
        }
        run_repository.save(&run, Some(now)).await?;
        purge_delegations(&tx, run.id).await?;
        emitter.emit(&RunFinished {
            run_id: run.id.0,
            organization_id: run.organization_id.0,
            status: run.status.as_str().to_owned(),
            failure: run.failure.clone(),
        })?;
        Ok(())
    }

    #[transactional(run)]
    pub async fn pending_runs(
        &self,
        stale_before: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<RunId>, CoreError> {
        let mut run_repository = run_repository;
        run_repository.pending(stale_before, limit).await
    }

    /// Reads a card for the search index. This is the system indexing every
    /// card of every organization, not a user acting: there is no subject to
    /// authorize, so it deliberately skips the PDP. The organization still
    /// scopes the read.
    #[transactional(card)]
    pub async fn card_for_index(
        &self,
        organization_id: OrganizationId,
        id: CardId,
    ) -> Result<Option<Card>, CoreError> {
        let mut card_repository = card_repository;
        card_repository.find(organization_id, id).await
    }

    /// Every card of an organization, for the search index; see `card_for_index`.
    #[transactional(card)]
    pub async fn cards_for_index(
        &self,
        organization_id: OrganizationId,
    ) -> Result<Vec<Card>, CoreError> {
        let mut card_repository = card_repository;
        card_repository
            .list(organization_id, &CardFilter::default())
            .await
    }

    pub async fn target_for(
        &self,
        organization_id: OrganizationId,
        model: &ModelRef,
    ) -> Result<ModelTarget, CoreError> {
        let access = self
            .provider_access(organization_id, model.provider)
            .await?;
        target_of(model, access)
    }

    pub async fn embedding_target_for(
        &self,
        organization_id: OrganizationId,
        defaults: &ModelAssignments,
    ) -> Result<ModelTarget, CoreError> {
        let chosen = self.model_assignments_for(organization_id).await?;
        let assignments = chosen.as_ref().unwrap_or(defaults);
        self.target_for(
            organization_id,
            assignments.require_role(AgentRole::Embeddings)?,
        )
        .await
    }

    pub async fn ensure_indexed<E: Embedder, V: VectorStore>(
        &self,
        embedder: &E,
        vectors: &V,
        embedding: &ModelTarget,
        organization_id: OrganizationId,
    ) -> Result<(), CoreError> {
        let model = embedding.model.model.as_str();
        if vectors.holds_cards(model, organization_id).await? {
            return Ok(());
        }
        let cards = self.cards_for_index(organization_id).await?;
        if cards.is_empty() {
            return Ok(());
        }
        tracing::info!(%organization_id, model, cards = cards.len(), "indexing the cards for this embedding model");
        for chunk in cards.chunks(REINDEX_BATCH) {
            let texts: Vec<String> = chunk
                .iter()
                .map(|card| card_text(card.title.as_str(), &card.description))
                .collect();
            let (embeddings, _) = embedder.embed(embedding, &texts).await?;
            if embeddings.len() != chunk.len() {
                return Err(CoreError::Internal(format!(
                    "the embedder returned {} vectors for {} cards",
                    embeddings.len(),
                    chunk.len()
                )));
            }
            for ((card, text), embedding) in chunk.iter().zip(texts).zip(embeddings) {
                vectors
                    .upsert(
                        model,
                        CardVector {
                            card_id: card.id,
                            organization_id: card.organization_id,
                            text,
                            embedding,
                            column: card.column,
                        },
                    )
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn organizations_for_index(&self) -> Result<Vec<OrganizationId>, CoreError> {
        sqlx::query_scalar::<_, uuid::Uuid>("SELECT id FROM organizations")
            .fetch_all(&self.pool)
            .await
            .map(|ids| ids.into_iter().map(OrganizationId).collect())
            .map_err(|error| CoreError::Database(error.to_string()))
    }

    #[transactional()]
    pub async fn delegation_for(
        &self,
        run_id: RunId,
        agent: &str,
    ) -> Result<Option<crate::domain::delegation::DelegatedToken>, CoreError> {
        PgDelegationVault::new(&tx, self.cipher()?)
            .load(run_id, agent)
            .await
    }
}

pub(super) fn claim_decision(
    status: RunStatus,
    heartbeat: Option<DateTime<Utc>>,
    stale_before: DateTime<Utc>,
) -> Option<bool> {
    match status {
        RunStatus::Queued => Some(false),
        RunStatus::Running if heartbeat.is_some_and(|at| at >= stale_before) => None,
        RunStatus::Running => Some(true),
        RunStatus::Succeeded | RunStatus::Failed | RunStatus::Cancelled => None,
    }
}

pub(super) async fn retry_transient<T, F, Fut>(
    max_attempts: u32,
    base_delay: Duration,
    mut attempt_fn: F,
) -> Result<T, CoreError>
where
    F: FnMut(u32) -> Fut,
    Fut: Future<Output = Result<T, CoreError>>,
{
    let mut attempt = 1;
    loop {
        match attempt_fn(attempt).await {
            Ok(value) => return Ok(value),
            Err(CoreError::Internal(message)) if attempt < max_attempts => {
                let delay = base_delay.saturating_mul(2u32.pow(attempt));
                tracing::warn!(attempt, %message, ?delay, "step attempt failed, retrying");
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
            Err(error) => return Err(error),
        }
    }
}

pub(super) fn new_step(
    run: &Run,
    seq: i32,
    kind: StepKind,
    agent: Option<String>,
    attempt: i32,
) -> Step {
    Step {
        id: StepId::new(),
        run_id: run.id,
        organization_id: run.organization_id,
        seq,
        kind,
        agent,
        status: StepStatus::Running,
        attempt,
        output: None,
        usage: Default::default(),
        cost_micros: 0,
        latency_ms: None,
        error: None,
        started_at: Utc::now(),
        finished_at: None,
    }
}

pub fn target_of(
    model: &ModelRef,
    access: Option<ProviderAccess>,
) -> Result<ModelTarget, CoreError> {
    match (model.provider, access) {
        (_, Some(access)) => Ok(ModelTarget {
            model: model.clone(),
            access: Some(access),
        }),
        (ProviderKind::OpenAiCompatible, None) => Ok(ModelTarget {
            model: model.clone(),
            access: None,
        }),
        (kind, None) => Err(CoreError::Invalid(format!(
            "no {} key configured for this organization",
            kind.as_str()
        ))),
    }
}

/// Extract → Deduplicate → Write, one step at a time, each committed before
/// the next starts. Model calls never run inside a database transaction.
pub struct MeetingPipeline<R, X, E, V> {
    usecase: CapitainerieUseCase,
    runtime: R,
    exchanger: X,
    embedder: E,
    vectors: V,
    config: PipelineConfig,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ExtractOutput {
    actions: Vec<LocatedAction>,
    /// Actions whose quote could not be found in the transcript: dropped.
    unlocated: Vec<String>,
    #[serde(default)]
    shown: Option<Shown>,
}

impl<R: AgentRuntime, X: TokenExchanger, E: Embedder, V: VectorStore> MeetingPipeline<R, X, E, V> {
    pub fn new(
        usecase: CapitainerieUseCase,
        runtime: R,
        exchanger: X,
        embedder: E,
        vectors: V,
        config: PipelineConfig,
    ) -> Self {
        Self {
            usecase,
            runtime,
            exchanger,
            embedder,
            vectors,
            config,
        }
    }

    pub fn usecase(&self) -> &CapitainerieUseCase {
        &self.usecase
    }

    pub fn stale_before(&self) -> DateTime<Utc> {
        let stale = chrono::Duration::from_std(self.config.stale_after)
            .unwrap_or(chrono::Duration::minutes(15));
        Utc::now() - stale
    }

    /// Executes the run if this worker can claim it. `traceparent` links the
    /// run's spans to the request that queued it.
    pub async fn execute(&self, run_id: RunId, traceparent: Option<&str>) -> Result<(), CoreError> {
        let span = tracing::info_span!(
            "run",
            otel.name = "invoke_workflow meeting",
            run_id = %run_id,
            gen_ai.operation.name = "invoke_workflow",
        );
        if let Some(traceparent) = traceparent {
            capitainerie_telemetry::adopt_traceparent(&span, traceparent);
        }
        self.execute_in_span(run_id).instrument(span).await
    }

    async fn execute_in_span(&self, run_id: RunId) -> Result<(), CoreError> {
        let trace_id = capitainerie_telemetry::current_trace_id();
        let Some(context) = self
            .usecase
            .claim_run(run_id, self.stale_before(), trace_id)
            .await?
        else {
            tracing::debug!(%run_id, "run not claimable here");
            return Ok(());
        };
        tracing::info!(%run_id, resumed = context.resumed, "run claimed");
        let outcome = self.run_steps(&context).await;
        let verdict = match &outcome {
            Ok(()) => Ok(()),
            Err(CoreError::Internal(message)) => Err(message.clone()),
            Err(error) => Err(error.to_string()),
        };
        self.usecase
            .finish_run(context.run.clone(), verdict)
            .await?;
        match outcome {
            Ok(()) => tracing::info!(%run_id, "run succeeded"),
            Err(error) => tracing::warn!(%run_id, %error, "run failed"),
        }
        Ok(())
    }

    async fn run_steps(&self, context: &RunContext) -> Result<(), CoreError> {
        let mut actions: Vec<LocatedAction> = Vec::new();
        let mut dedup: Option<DedupOutput> = None;
        let mut shown: Option<Shown> = None;
        for (index, kind) in context.run.pipeline.iter().enumerate() {
            let seq = i32::try_from(index).unwrap_or(i32::MAX);
            if let Some(done) = context.steps.iter().find(|step| {
                step.seq == seq
                    && matches!(step.status, StepStatus::Succeeded | StepStatus::Skipped)
            }) {
                match kind {
                    StepKind::Extract => (actions, shown) = restore_extract(done)?,
                    StepKind::Deduplicate => dedup = restore_dedup(done),
                    StepKind::Write => {
                        let (new, _) = split_actions(&actions, dedup.as_ref());
                        self.finish_links(context, done, &new, shown.as_ref()).await;
                    }
                    StepKind::Execute => {}
                }
                continue;
            }
            match kind {
                StepKind::Extract => (actions, shown) = self.extract(context, seq).await?,
                StepKind::Deduplicate => {
                    dedup = Some(self.deduplicate(context, seq, &actions).await?)
                }
                StepKind::Write => {
                    let (new, duplicates) = split_actions(&actions, dedup.as_ref());
                    self.write(context, seq, new, duplicates, shown.as_ref())
                        .await?;
                }
                StepKind::Execute => {
                    return Err(CoreError::Invalid(
                        "the meeting pipeline cannot run an execute step".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    async fn target(
        &self,
        context: &RunContext,
        role: AgentRole,
    ) -> Result<ModelTarget, CoreError> {
        let assignments = context
            .run
            .models
            .as_ref()
            .unwrap_or(&self.config.default_models);
        self.usecase
            .target_for(context.run.organization_id, assignments.require_role(role)?)
            .await
    }

    async fn dedup_targets(
        &self,
        context: &RunContext,
    ) -> Result<(ModelTarget, ModelTarget), CoreError> {
        Ok((
            self.target(context, AgentRole::Embeddings).await?,
            self.target(context, AgentRole::Judge).await?,
        ))
    }

    fn new_step(
        &self,
        context: &RunContext,
        seq: i32,
        kind: StepKind,
        agent: Option<String>,
        attempt: i32,
    ) -> Step {
        new_step(&context.run, seq, kind, agent, attempt)
    }

    async fn finish_step(
        &self,
        context: &RunContext,
        mut step: Step,
        started: Instant,
        result: Result<(serde_json::Value, crate::domain::agent::Usage, StepStatus), CoreError>,
    ) -> Result<Step, CoreError> {
        step.latency_ms = Some(i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX));
        step.finished_at = Some(Utc::now());
        match result {
            Ok((output, usage, status)) => {
                step.cost_micros = self.config.pricing.cost_micros(&usage);
                step.usage = usage;
                step.output = Some(output);
                step.status = status;
            }
            Err(error) => {
                step.status = StepStatus::Failed;
                step.error = Some(error.to_string());
            }
        }
        self.usecase
            .record_step(context.run.clone(), step.clone())
            .await?;
        Ok(step)
    }

    /// Retries what may be transient (the model or the identity provider
    /// unreachable); a refusal or invalid output fails at once.
    async fn with_retries<T, F, Fut>(&self, attempt_fn: F) -> Result<T, CoreError>
    where
        F: FnMut(u32) -> Fut,
        Fut: Future<Output = Result<T, CoreError>>,
    {
        retry_transient(self.config.max_attempts, Duration::from_secs(1), attempt_fn).await
    }

    #[tracing::instrument(name = "step extract", skip_all, fields(seq))]
    async fn extract(
        &self,
        context: &RunContext,
        seq: i32,
    ) -> Result<(Vec<LocatedAction>, Option<Shown>), CoreError> {
        let body = context.transcript.body.as_str();
        let step = self.new_step(context, seq, StepKind::Extract, Some("extractor".into()), 1);
        self.usecase
            .record_step(context.run.clone(), step.clone())
            .await?;
        let started = Instant::now();
        let prepared = match self.target(context, AgentRole::Extractor).await {
            Ok(target) => self
                .detectable(context)
                .await
                .map(|catalog| (target, catalog)),
            Err(error) => Err(error),
        };
        let shown = prepared
            .as_ref()
            .ok()
            .and_then(|(_, catalog)| catalog.as_ref())
            .map(Catalog::shown);
        let extracted = match prepared {
            Ok((target, catalog)) => {
                let prompt = context
                    .transcript
                    .metadata
                    .prompt_with(body, catalog.as_ref());
                self.with_retries(|_| self.runtime.extract(&target, &prompt))
                    .await
            }
            Err(error) => Err(error),
        };
        let result = extracted.map(|extraction| {
            let mut located = Vec::new();
            let mut unlocated = Vec::new();
            for mut action in extraction.actions {
                match locate(body, &action.quote) {
                    Some((start, end, text)) => {
                        action.quote = text;
                        located.push(LocatedAction { action, start, end });
                    }
                    None => unlocated.push(action.title),
                }
            }
            if !unlocated.is_empty() {
                tracing::warn!(
                    count = unlocated.len(),
                    "actions dropped: quote not found in the transcript"
                );
            }
            (
                ExtractOutput {
                    actions: located,
                    unlocated,
                    shown,
                },
                extraction.usage,
            )
        });
        let result = result.and_then(|(output, usage)| {
            serde_json::to_value(&output)
                .map(|value| (value, usage, StepStatus::Succeeded))
                .map_err(|error| CoreError::Internal(format!("encoding the extraction: {error}")))
        });
        let step = self.finish_step(context, step, started, result).await?;
        match step.status {
            StepStatus::Failed => Err(CoreError::Internal(step.error.unwrap_or_default())),
            _ => restore_extract(&step),
        }
    }

    async fn detectable(&self, context: &RunContext) -> Result<Option<Catalog>, CoreError> {
        if !context.transcript.links.is_empty() {
            return Ok(None);
        }
        self.usecase
            .detection_catalog(context.run.organization_id)
            .await
            .map(Some)
    }

    async fn links_for(
        &self,
        context: &RunContext,
        actions: &[LocatedAction],
        shown: Option<&Shown>,
    ) -> Result<Vec<RunCardLinks>, CoreError> {
        if !context.transcript.links.is_empty() {
            return Ok(vec![RunCardLinks {
                span: None,
                links: context.transcript.links.clone(),
            }]);
        }
        let Some(shown) = shown else {
            return Ok(Vec::new());
        };
        let catalog = self
            .usecase
            .detection_catalog(context.run.organization_id)
            .await?
            .restricted_to(shown);
        Ok(actions
            .iter()
            .map(|located| RunCardLinks {
                span: Some((located.start, located.end)),
                links: catalog.resolve(
                    located.action.client_ref.as_deref(),
                    located.action.project_ref.as_deref(),
                ),
            })
            .collect())
    }

    async fn finish_links(
        &self,
        context: &RunContext,
        step: &Step,
        actions: &[LocatedAction],
        shown: Option<&Shown>,
    ) {
        let done = step
            .output
            .as_ref()
            .and_then(|output| output.get("links_done"))
            .and_then(serde_json::Value::as_bool)
            == Some(true);
        if step.status != StepStatus::Succeeded || done {
            return;
        }
        if self.link_cards(context, actions, shown).await
            && let Err(error) = self.usecase.mark_links_done(context.run.id, step.seq).await
        {
            tracing::warn!(%error, "linking could not be recorded as done");
        }
    }

    async fn link_cards(
        &self,
        context: &RunContext,
        actions: &[LocatedAction],
        shown: Option<&Shown>,
    ) -> bool {
        let linked = match self.links_for(context, actions, shown).await {
            Ok(batches) => {
                self.usecase
                    .link_run_cards(
                        context.run.organization_id,
                        context.run.id,
                        &context.run.initiated_by,
                        &self.config.writer_agent,
                        batches,
                    )
                    .await
            }
            Err(error) => Err(error),
        };
        match linked {
            Ok(count) => {
                tracing::info!(count, "cards linked to a client or project");
                true
            }
            Err(error) => {
                tracing::warn!(%error, "the cards could not be linked");
                false
            }
        }
    }

    #[tracing::instrument(name = "step deduplicate", skip_all, fields(seq, actions = actions.len()))]
    async fn deduplicate(
        &self,
        context: &RunContext,
        seq: i32,
        actions: &[LocatedAction],
    ) -> Result<DedupOutput, CoreError> {
        let step = self.new_step(
            context,
            seq,
            StepKind::Deduplicate,
            Some("deduplicator".into()),
            1,
        );
        if actions.is_empty() {
            let output = json!({ "reason": "no action item in the transcript" });
            self.finish_step(
                context,
                step,
                Instant::now(),
                Ok((output, Default::default(), StepStatus::Skipped)),
            )
            .await?;
            return Ok(all_new(0));
        }
        self.usecase
            .record_step(context.run.clone(), step.clone())
            .await?;
        let started = Instant::now();
        let found = match self.dedup_targets(context).await {
            Ok((embedding, judge)) => {
                self.with_retries(|_| async {
                    self.usecase
                        .ensure_indexed(
                            &self.embedder,
                            &self.vectors,
                            &embedding,
                            context.run.organization_id,
                        )
                        .await?;
                    find_duplicates(
                        &Dedup {
                            runtime: &self.runtime,
                            embedder: &self.embedder,
                            vectors: &self.vectors,
                            embedding: &embedding,
                            judge: &judge,
                            organization_id: context.run.organization_id,
                            threshold: self.config.dedup_threshold,
                            gray_floor: self.config.dedup_gray_floor,
                        },
                        actions,
                    )
                    .await
                })
                .await
            }
            Err(error) => Err(error),
        };
        let result = found.and_then(|(output, usage)| {
            serde_json::to_value(&output)
                .map(|value| (value, usage, StepStatus::Succeeded))
                .map_err(|error| CoreError::Internal(format!("encoding the dedup: {error}")))
        });
        let step = self.finish_step(context, step, started, result).await?;
        match step.status {
            StepStatus::Failed => Err(CoreError::Internal(step.error.unwrap_or_default())),
            _ => Ok(restore_dedup(&step).unwrap_or_else(|| all_new(actions.len()))),
        }
    }

    #[tracing::instrument(name = "step write", skip_all, fields(seq, actions = actions.len(), duplicates = duplicates.len()))]
    async fn write(
        &self,
        context: &RunContext,
        seq: i32,
        actions: Vec<LocatedAction>,
        duplicates: Vec<DuplicateAction>,
        shown: Option<&Shown>,
    ) -> Result<(), CoreError> {
        let writer = self.config.writer_agent.clone();
        let step = self.new_step(context, seq, StepKind::Write, Some(writer.clone()), 1);
        if actions.is_empty() && duplicates.is_empty() {
            let output = json!({ "reason": "no action item in the transcript" });
            self.finish_step(
                context,
                step,
                Instant::now(),
                Ok((output, Default::default(), StepStatus::Skipped)),
            )
            .await?;
            return Ok(());
        }
        self.usecase
            .record_step(context.run.clone(), step.clone())
            .await?;
        let started = Instant::now();
        let result = match self.target(context, AgentRole::Writer).await {
            Ok(target) => {
                self.write_attempts(context, &target, &writer, &actions, &duplicates)
                    .await
            }
            Err(error) => Err(error),
        };
        let step = self.finish_step(context, step, started, result).await?;
        match step.status {
            StepStatus::Failed => Err(CoreError::Internal(step.error.unwrap_or_default())),
            _ => {
                self.finish_links(context, &step, &actions, shown).await;
                Ok(())
            }
        }
    }

    async fn write_attempts(
        &self,
        context: &RunContext,
        target: &ModelTarget,
        writer: &str,
        actions: &[LocatedAction],
        duplicates: &[DuplicateAction],
    ) -> Result<(serde_json::Value, crate::domain::agent::Usage, StepStatus), CoreError> {
        let delegation = self
            .usecase
            .delegation_for(context.run.id, writer)
            .await?
            .ok_or_else(|| CoreError::Forbidden {
                reason: Some(format!(
                    "no delegation from the user to {writer} for this run"
                )),
            })?;
        if is_expired(&delegation, Utc::now()) {
            return Err(CoreError::Forbidden {
                reason: Some("the user's delegation expired before the writer ran".into()),
            });
        }
        let report = self
            .with_retries(|_| async {
                let mcp_token = self
                    .exchanger
                    .for_mcp(&context.issuer_url, &delegation.token, writer)
                    .await?;
                self.runtime
                    .write(
                        target,
                        WriteRequest {
                            run_id: context.run.id,
                            transcript_id: context.transcript.id,
                            meeting_date: context.transcript.metadata.date,
                            actions: actions.to_vec(),
                            duplicates: duplicates.to_vec(),
                            mcp_token: SecretToken::new(mcp_token.token.expose()),
                            traceparent: capitainerie_telemetry::current_traceparent(),
                        },
                    )
                    .await
            })
            .await?;
        let succeeded = report
            .tool_calls
            .iter()
            .filter(|call| call.succeeded)
            .count();
        let output = serde_json::to_value(WriteOutput {
            tool_calls: report.tool_calls,
        })
        .map_err(|error| CoreError::Internal(format!("encoding the write report: {error}")))?;
        if !write_succeeded(succeeded) {
            return Err(CoreError::Internal(format!(
                "the writer created no card and left no comment for {} new and {} repeated action(s): {output}",
                actions.len(),
                duplicates.len()
            )));
        }
        Ok((output, report.usage, StepStatus::Succeeded))
    }
}

fn restore_extract(step: &Step) -> Result<(Vec<LocatedAction>, Option<Shown>), CoreError> {
    let Some(output) = &step.output else {
        return Ok((Vec::new(), None));
    };
    serde_json::from_value::<ExtractOutput>(output.clone())
        .map(|output| (output.actions, output.shown))
        .map_err(|error| CoreError::Internal(format!("decoding a stored extraction: {error}")))
}

/// The write step has something to do whenever it is called (it is skipped
/// otherwise), so it succeeded only if at least one tool call went through: a
/// card created, or a comment left on the card an action repeated.
fn write_succeeded(successful_tool_calls: usize) -> bool {
    successful_tool_calls > 0
}

const REINDEX_BATCH: usize = 16;

struct Dedup<'a, R, E, V> {
    runtime: &'a R,
    embedder: &'a E,
    vectors: &'a V,
    embedding: &'a ModelTarget,
    judge: &'a ModelTarget,
    organization_id: OrganizationId,
    threshold: f32,
    gray_floor: f32,
}

async fn find_duplicates<R: AgentRuntime, E: Embedder, V: VectorStore>(
    dedup: &Dedup<'_, R, E, V>,
    actions: &[LocatedAction],
) -> Result<(DedupOutput, crate::domain::agent::Usage), CoreError> {
    let texts: Vec<String> = actions.iter().map(action_text).collect();
    let (embeddings, mut usage) = dedup.embedder.embed(dedup.embedding, &texts).await?;
    if embeddings.len() != actions.len() {
        return Err(CoreError::Internal(format!(
            "the embedder returned {} vectors for {} texts",
            embeddings.len(),
            actions.len()
        )));
    }
    let mut neighbours = Vec::with_capacity(embeddings.len());
    for embedding in &embeddings {
        neighbours.push(
            dedup
                .vectors
                .nearest(
                    &dedup.embedding.model.model,
                    dedup.organization_id,
                    embedding,
                    3,
                )
                .await?,
        );
    }
    let mut output = DedupOutput {
        new: Vec::new(),
        duplicates: Vec::new(),
        judged: Vec::new(),
        embedding_tokens: usage.input_tokens,
    };
    for (action, (located, candidates)) in actions.iter().zip(&neighbours).enumerate() {
        match triage(candidates, dedup.threshold, dedup.gray_floor) {
            Triage::Duplicate(card) => output.duplicates.push(DuplicateMatch {
                action,
                card_id: card.card_id,
                score: card.score,
                decided_by: DecidedBy::Similarity,
            }),
            Triage::New => output.new.push(action),
            Triage::Judge(card) => {
                let verdict = dedup
                    .runtime
                    .judge_duplicate(dedup.judge, located, card)
                    .await;
                let mut pair = JudgedPair {
                    action,
                    card_id: card.card_id,
                    score: card.score,
                    same_task: false,
                    reason: String::new(),
                    error: None,
                };
                match verdict {
                    Ok(judgement) => {
                        usage.absorb(&judgement.usage);
                        pair.same_task = judgement.same_task;
                        pair.reason = judgement.reason;
                    }
                    Err(error) => {
                        tracing::warn!(action, %error, "the judge failed, the action stays new");
                        pair.error = Some(error.to_string());
                    }
                }
                if pair.same_task {
                    output.duplicates.push(DuplicateMatch {
                        action,
                        card_id: card.card_id,
                        score: card.score,
                        decided_by: DecidedBy::Judge,
                    });
                } else {
                    output.new.push(action);
                }
                output.judged.push(pair);
            }
        }
    }
    Ok((output, usage))
}

fn all_new(count: usize) -> DedupOutput {
    DedupOutput {
        new: (0..count).collect(),
        duplicates: Vec::new(),
        judged: Vec::new(),
        embedding_tokens: 0,
    }
}

fn restore_dedup(step: &Step) -> Option<DedupOutput> {
    step.output
        .as_ref()
        .and_then(|output| serde_json::from_value::<DedupOutput>(output.clone()).ok())
}

fn split_actions(
    actions: &[LocatedAction],
    dedup: Option<&DedupOutput>,
) -> (Vec<LocatedAction>, Vec<DuplicateAction>) {
    let Some(dedup) = dedup else {
        return (actions.to_vec(), Vec::new());
    };
    let new = dedup
        .new
        .iter()
        .filter_map(|&index| actions.get(index).cloned())
        .collect();
    let duplicates = dedup
        .duplicates
        .iter()
        .filter_map(|found| {
            actions.get(found.action).map(|action| DuplicateAction {
                action: action.clone(),
                card_id: found.card_id,
                score: found.score,
            })
        })
        .collect();
    (new, duplicates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        agent::{ExtractedAction, Judgement, Usage, ports::MockAgentRuntime},
        similarity::{
            DuplicateMatch, Embedding, SimilarCard,
            ports::{MockEmbedder, MockVectorStore},
        },
    };

    fn located(title: &str) -> LocatedAction {
        LocatedAction {
            action: ExtractedAction {
                title: title.into(),
                description: String::new(),
                quote: "q".into(),
                owner: None,
                due: None,
                client_ref: None,
                project_ref: None,
            },
            start: 0,
            end: 1,
        }
    }

    #[test]
    fn the_write_succeeds_on_one_card_or_one_comment_and_fails_on_none() {
        assert!(write_succeeded(1));
        assert!(write_succeeded(3));
        assert!(!write_succeeded(0));
    }

    fn model(provider: ProviderKind) -> ModelRef {
        ModelRef {
            provider,
            model: "m".into(),
        }
    }

    fn access(kind: ProviderKind) -> ProviderAccess {
        ProviderAccess {
            kind,
            secret: SecretToken::new("k"),
            base_url: None,
        }
    }

    #[test]
    fn a_credential_of_the_organization_is_carried_into_the_target() {
        let target = target_of(
            &model(ProviderKind::Anthropic),
            Some(access(ProviderKind::Anthropic)),
        )
        .unwrap();
        assert_eq!(target.access, Some(access(ProviderKind::Anthropic)));
        assert_eq!(target.model, model(ProviderKind::Anthropic));
    }

    #[test]
    fn without_a_credential_only_the_openai_compatible_provider_uses_the_worker_endpoint() {
        let local = target_of(&model(ProviderKind::OpenAiCompatible), None).unwrap();
        assert_eq!(local.access, None);
        for kind in [ProviderKind::Anthropic, ProviderKind::Mistral] {
            let error = target_of(&model(kind), None).unwrap_err();
            assert!(matches!(&error, CoreError::Invalid(message)
                if message == &format!("no {} key configured for this organization", kind.as_str())));
        }
    }

    #[test]
    fn actions_split_into_new_and_duplicates() {
        let actions = [located("a"), located("b"), located("c")];
        let card = CardId::new();
        let dedup = DedupOutput {
            new: vec![0, 2],
            duplicates: vec![DuplicateMatch {
                action: 1,
                card_id: card,
                score: 0.9,
                decided_by: DecidedBy::Similarity,
            }],
            judged: Vec::new(),
            embedding_tokens: 0,
        };
        let (new, duplicates) = split_actions(&actions, Some(&dedup));
        assert_eq!(new, [located("a"), located("c")]);
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].action, located("b"));
        assert_eq!(duplicates[0].card_id, card);
    }

    #[test]
    fn without_a_dedup_every_action_is_new() {
        let (new, duplicates) = split_actions(&[located("a")], None);
        assert_eq!(new.len(), 1);
        assert!(duplicates.is_empty());
    }

    #[test]
    fn a_skipped_dedup_output_restores_to_nothing() {
        let skipped = serde_json::json!({ "reason": "no action item in the transcript" });
        assert!(serde_json::from_value::<DedupOutput>(skipped).is_err());
    }

    fn embedder(tokens: u64) -> MockEmbedder {
        let mut embedder = MockEmbedder::new();
        embedder.expect_embed().returning(move |_, texts| {
            let embeddings = texts.iter().map(|_| Embedding(vec![1.0])).collect();
            Box::pin(std::future::ready(Ok((
                embeddings,
                Usage {
                    model: "emb".into(),
                    input_tokens: tokens,
                    output_tokens: 0,
                },
            ))))
        });
        embedder
    }

    fn vectors_scoring(scores: Vec<f32>) -> (MockVectorStore, Vec<CardId>) {
        let cards: Vec<SimilarCard> = scores
            .into_iter()
            .map(|score| SimilarCard {
                card_id: CardId::new(),
                score,
                text: "card".into(),
            })
            .collect();
        let ids = cards.iter().map(|card| card.card_id).collect();
        let mut vectors = MockVectorStore::new();
        let calls = std::sync::atomic::AtomicUsize::new(0);
        vectors.expect_nearest().returning(move |_, _, _, limit| {
            assert_eq!(limit, 3);
            let at = calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Box::pin(std::future::ready(Ok(vec![cards[at].clone()])))
        });
        (vectors, ids)
    }

    fn target(model: &str) -> ModelTarget {
        ModelTarget {
            model: ModelRef {
                provider: ProviderKind::OpenAiCompatible,
                model: model.into(),
            },
            access: None,
        }
    }

    fn targets() -> (ModelTarget, ModelTarget) {
        (target("emb"), target("chat"))
    }

    fn dedup_with<'a>(
        targets: &'a (ModelTarget, ModelTarget),
        runtime: &'a MockAgentRuntime,
        embedder: &'a MockEmbedder,
        vectors: &'a MockVectorStore,
    ) -> Dedup<'a, MockAgentRuntime, MockEmbedder, MockVectorStore> {
        Dedup {
            runtime,
            embedder,
            vectors,
            embedding: &targets.0,
            judge: &targets.1,
            organization_id: OrganizationId::new(),
            threshold: 0.75,
            gray_floor: 0.65,
        }
    }

    fn judgement(same_task: bool) -> Judgement {
        Judgement {
            same_task,
            reason: "because".into(),
            usage: Usage {
                model: "chat".into(),
                input_tokens: 100,
                output_tokens: 5,
            },
        }
    }

    async fn dedup(
        runtime: &MockAgentRuntime,
        scores: Vec<f32>,
    ) -> Result<(DedupOutput, Usage), CoreError> {
        let actions: Vec<LocatedAction> =
            (0..scores.len()).map(|i| located(&i.to_string())).collect();
        let (vectors, _) = vectors_scoring(scores);
        let targets = targets();
        find_duplicates(
            &dedup_with(&targets, runtime, &embedder(12), &vectors),
            &actions,
        )
        .await
    }

    #[tokio::test]
    async fn above_the_threshold_and_below_the_floor_the_judge_is_not_asked() {
        let mut runtime = MockAgentRuntime::new();
        runtime.expect_judge_duplicate().times(0);

        let (output, usage) = dedup(&runtime, vec![0.75, 0.9, 0.64, 0.1]).await.unwrap();

        assert_eq!(output.new, [2, 3]);
        let decided: Vec<_> = output
            .duplicates
            .iter()
            .map(|found| (found.action, found.decided_by))
            .collect();
        assert_eq!(
            decided,
            [(0, DecidedBy::Similarity), (1, DecidedBy::Similarity)]
        );
        assert!(output.judged.is_empty());
        assert_eq!(output.embedding_tokens, 12);
        assert_eq!(usage.model, "emb");
    }

    #[tokio::test]
    async fn in_the_gray_zone_the_judge_decides_and_its_usage_is_added() {
        let mut runtime = MockAgentRuntime::new();
        let answers = std::sync::Mutex::new(vec![true, false]);
        runtime
            .expect_judge_duplicate()
            .times(2)
            .returning(move |_, _, _| {
                let same = answers.lock().unwrap().remove(0);
                Box::pin(std::future::ready(Ok(judgement(same))))
            });

        let (output, usage) = dedup(&runtime, vec![0.7, 0.65, 0.2]).await.unwrap();

        assert_eq!(output.new, [1, 2]);
        assert_eq!(output.duplicates.len(), 1);
        assert_eq!(output.duplicates[0].action, 0);
        assert_eq!(output.duplicates[0].decided_by, DecidedBy::Judge);
        let verdicts: Vec<_> = output
            .judged
            .iter()
            .map(|pair| (pair.action, pair.same_task, pair.reason.as_str()))
            .collect();
        assert_eq!(verdicts, [(0, true, "because"), (1, false, "because")]);
        assert_eq!(output.embedding_tokens, 12);
        assert_eq!(usage.input_tokens, 12 + 200);
        assert_eq!(usage.output_tokens, 10);
        assert_eq!(usage.model, "chat");
    }

    #[tokio::test]
    async fn the_judge_is_given_the_best_candidate_only() {
        let best = SimilarCard {
            card_id: CardId::new(),
            score: 0.7,
            text: "best".into(),
        };
        let second = SimilarCard {
            card_id: CardId::new(),
            score: 0.68,
            text: "second".into(),
        };
        let mut vectors = MockVectorStore::new();
        let candidates = vec![second, best.clone()];
        vectors
            .expect_nearest()
            .returning(move |_, _, _, _| Box::pin(std::future::ready(Ok(candidates.clone()))));
        let mut runtime = MockAgentRuntime::new();
        let expected = best.clone();
        runtime
            .expect_judge_duplicate()
            .times(1)
            .returning(move |_, _, candidate| {
                assert_eq!(*candidate, expected);
                Box::pin(std::future::ready(Ok(judgement(true))))
            });

        let targets = targets();
        let (output, _) = find_duplicates(
            &dedup_with(&targets, &runtime, &embedder(1), &vectors),
            &[located("a")],
        )
        .await
        .unwrap();

        assert_eq!(output.duplicates[0].card_id, best.card_id);
    }

    #[tokio::test]
    async fn a_failing_judge_leaves_the_action_new_and_says_so() {
        let mut runtime = MockAgentRuntime::new();
        runtime
            .expect_judge_duplicate()
            .times(1)
            .returning(|_, _, _| {
                Box::pin(std::future::ready(Err(CoreError::Internal(
                    "model down".into(),
                ))))
            });

        let (output, usage) = dedup(&runtime, vec![0.7]).await.unwrap();

        assert_eq!(output.new, [0]);
        assert!(output.duplicates.is_empty());
        assert!(!output.judged[0].same_task);
        assert!(
            output.judged[0]
                .error
                .as_deref()
                .unwrap()
                .contains("model down")
        );
        assert_eq!(usage.model, "emb");
        assert_eq!(usage.input_tokens, 12);
    }

    #[tokio::test]
    async fn a_wrong_number_of_vectors_is_an_error() {
        let mut embedder = MockEmbedder::new();
        embedder
            .expect_embed()
            .returning(|_, _| Box::pin(std::future::ready(Ok((Vec::new(), Usage::default())))));
        let vectors = MockVectorStore::new();
        let targets = targets();
        let error = find_duplicates(
            &dedup_with(&targets, &MockAgentRuntime::new(), &embedder, &vectors),
            &[located("a")],
        )
        .await
        .unwrap_err();
        assert!(matches!(error, CoreError::Internal(_)));
    }
}
