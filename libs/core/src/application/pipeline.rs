use std::time::{Duration, Instant};

use capitainerie_common::{CoreError, RunId, StepId};
use capitainerie_events::run::{RunFinished, RunStarted, StepFinished};
use capitainerie_macros::transactional;
use chrono::{DateTime, Utc};
use serde_json::json;
use tracing::Instrument;

use super::CapitainerieUseCase;
use crate::{
    domain::{
        agent::{LocatedAction, SecretToken, WriteRequest, ports::AgentRuntime},
        delegation::ports::{DelegationVault, TokenExchanger},
        event::ports::EventEmitter,
        organization::ports::OrganizationRepository,
        run::{
            Run, RunStatus, StepKind,
            locate::locate,
            ports::{RunRepository, StepRepository},
            step::{Pricing, Step, StepStatus, WriteOutput},
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
        let resumed = match run.status {
            RunStatus::Queued => {
                run.start(now)?;
                false
            }
            RunStatus::Running => {
                let heartbeat = run_repository.heartbeat_at(id).await?;
                if heartbeat.is_some_and(|at| at >= stale_before) {
                    return Ok(None);
                }
                true
            }
            RunStatus::Succeeded | RunStatus::Failed | RunStatus::Cancelled => return Ok(None),
        };
        if trace_id.is_some() {
            run.trace_id = trace_id;
        }
        run_repository.save(&run, Some(now)).await?;

        let transcript = transcript_repository
            .find(run.organization_id, run.transcript_id)
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

/// Extract → Deduplicate → Write, one step at a time, each committed before
/// the next starts. Model calls never run inside a database transaction.
pub struct MeetingPipeline<R, X> {
    usecase: CapitainerieUseCase,
    runtime: R,
    exchanger: X,
    config: PipelineConfig,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ExtractOutput {
    actions: Vec<LocatedAction>,
    /// Actions whose quote could not be found in the transcript: dropped.
    unlocated: Vec<String>,
}

impl<R: AgentRuntime, X: TokenExchanger> MeetingPipeline<R, X> {
    pub fn new(
        usecase: CapitainerieUseCase,
        runtime: R,
        exchanger: X,
        config: PipelineConfig,
    ) -> Self {
        Self {
            usecase,
            runtime,
            exchanger,
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
        let mut actions: Option<Vec<LocatedAction>> = None;
        for (index, kind) in context.run.pipeline.iter().enumerate() {
            let seq = i32::try_from(index).unwrap_or(i32::MAX);
            if let Some(done) = context.steps.iter().find(|step| {
                step.seq == seq
                    && matches!(step.status, StepStatus::Succeeded | StepStatus::Skipped)
            }) {
                if *kind == StepKind::Extract {
                    actions = Some(restore_actions(done)?);
                }
                continue;
            }
            match kind {
                StepKind::Extract => actions = Some(self.extract(context, seq).await?),
                StepKind::Deduplicate => self.skip_deduplicate(context, seq).await?,
                StepKind::Write => {
                    let located = actions.clone().unwrap_or_default();
                    self.write(context, seq, located).await?;
                }
            }
        }
        Ok(())
    }

    fn new_step(
        &self,
        context: &RunContext,
        seq: i32,
        kind: StepKind,
        agent: Option<String>,
        attempt: i32,
    ) -> Step {
        Step {
            id: StepId::new(),
            run_id: context.run.id,
            organization_id: context.run.organization_id,
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
    async fn with_retries<T, F, Fut>(&self, mut attempt_fn: F) -> Result<T, CoreError>
    where
        F: FnMut(u32) -> Fut,
        Fut: Future<Output = Result<T, CoreError>>,
    {
        let mut attempt = 1;
        loop {
            match attempt_fn(attempt).await {
                Ok(value) => return Ok(value),
                Err(CoreError::Internal(message)) if attempt < self.config.max_attempts => {
                    let delay = Duration::from_secs(2u64.pow(attempt));
                    tracing::warn!(attempt, %message, ?delay, "step attempt failed, retrying");
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                }
                Err(error) => return Err(error),
            }
        }
    }

    #[tracing::instrument(name = "step extract", skip_all, fields(seq))]
    async fn extract(
        &self,
        context: &RunContext,
        seq: i32,
    ) -> Result<Vec<LocatedAction>, CoreError> {
        let body = context.transcript.body.as_str();
        let step = self.new_step(context, seq, StepKind::Extract, Some("extractor".into()), 1);
        self.usecase
            .record_step(context.run.clone(), step.clone())
            .await?;
        let started = Instant::now();
        let result = self
            .with_retries(|_| self.runtime.extract(body))
            .await
            .map(|extraction| {
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
            _ => restore_actions(&step),
        }
    }

    async fn skip_deduplicate(&self, context: &RunContext, seq: i32) -> Result<(), CoreError> {
        let step = self.new_step(context, seq, StepKind::Deduplicate, None, 1);
        let output =
            json!({ "reason": "deduplication against existing cards (LanceDB) is not wired yet" });
        self.finish_step(
            context,
            step,
            Instant::now(),
            Ok((output, Default::default(), StepStatus::Skipped)),
        )
        .await
        .map(|_| ())
    }

    #[tracing::instrument(name = "step write", skip_all, fields(seq, actions = actions.len()))]
    async fn write(
        &self,
        context: &RunContext,
        seq: i32,
        actions: Vec<LocatedAction>,
    ) -> Result<(), CoreError> {
        let writer = self.config.writer_agent.clone();
        let step = self.new_step(context, seq, StepKind::Write, Some(writer.clone()), 1);
        if actions.is_empty() {
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
        let result = self.write_attempts(context, &writer, actions).await;
        let step = self.finish_step(context, step, started, result).await?;
        match step.status {
            StepStatus::Failed => Err(CoreError::Internal(step.error.unwrap_or_default())),
            _ => Ok(()),
        }
    }

    async fn write_attempts(
        &self,
        context: &RunContext,
        writer: &str,
        actions: Vec<LocatedAction>,
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
                    .write(WriteRequest {
                        run_id: context.run.id,
                        transcript_id: context.transcript.id,
                        actions: actions.clone(),
                        mcp_token: SecretToken::new(mcp_token.token.expose()),
                        traceparent: capitainerie_telemetry::current_traceparent(),
                    })
                    .await
            })
            .await?;
        let created = report
            .tool_calls
            .iter()
            .filter(|call| call.succeeded)
            .count();
        let output = serde_json::to_value(WriteOutput {
            tool_calls: report.tool_calls,
        })
        .map_err(|error| CoreError::Internal(format!("encoding the write report: {error}")))?;
        if created == 0 {
            return Err(CoreError::Internal(format!(
                "the writer created no card for {} action(s): {output}",
                actions.len()
            )));
        }
        Ok((output, report.usage, StepStatus::Succeeded))
    }
}

fn restore_actions(step: &Step) -> Result<Vec<LocatedAction>, CoreError> {
    let Some(output) = &step.output else {
        return Ok(Vec::new());
    };
    serde_json::from_value::<ExtractOutput>(output.clone())
        .map(|output| output.actions)
        .map_err(|error| CoreError::Internal(format!("decoding a stored extraction: {error}")))
}
