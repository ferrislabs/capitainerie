use super::{RunOverview, step::Step, step::StepStatus};

/// A queued or running run with what the live page shows of it.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveRun {
    pub overview: RunOverview,
    /// The latest step still running; `None` while the run waits in the queue.
    pub current_step: Option<Step>,
    pub steps_done: u32,
    pub steps_total: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_micros: u64,
}

impl ActiveRun {
    pub fn assemble(overview: RunOverview, steps: &[Step]) -> Self {
        let current_step = steps
            .iter()
            .filter(|step| step.status == StepStatus::Running)
            .max_by_key(|step| step.seq)
            .cloned();
        let steps_done = steps
            .iter()
            .filter(|step| matches!(step.status, StepStatus::Succeeded | StepStatus::Skipped))
            .count();
        Self {
            steps_done: u32::try_from(steps_done).unwrap_or(u32::MAX),
            steps_total: u32::try_from(overview.run.pipeline.len()).unwrap_or(u32::MAX),
            input_tokens: steps.iter().map(|step| step.usage.input_tokens).sum(),
            output_tokens: steps.iter().map(|step| step.usage.output_tokens).sum(),
            cost_micros: steps
                .iter()
                .map(|step| u64::try_from(step.cost_micros).unwrap_or_default())
                .sum(),
            current_step,
            overview,
        }
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_common::{OrganizationId, StepId, TranscriptId};
    use chrono::Utc;

    use super::*;
    use crate::domain::{
        agent::Usage,
        run::{Run, StepKind},
    };

    fn overview(pipeline: Vec<StepKind>) -> RunOverview {
        let (run, _) = Run::queue(
            OrganizationId::new(),
            TranscriptId::new(),
            "user-1".into(),
            pipeline,
            Utc::now(),
        );
        RunOverview {
            run,
            title: None,
            repository_hint: None,
            links: Default::default(),
        }
    }

    fn step(run: &Run, seq: i32, status: StepStatus, tokens: (u64, u64), cost_micros: i64) -> Step {
        Step {
            id: StepId::new(),
            run_id: run.id,
            organization_id: run.organization_id,
            seq,
            kind: StepKind::Extract,
            agent: Some("extractor".into()),
            status,
            attempt: 1,
            output: None,
            usage: Usage {
                model: "m".into(),
                input_tokens: tokens.0,
                output_tokens: tokens.1,
            },
            cost_micros,
            latency_ms: None,
            error: None,
            started_at: Utc::now(),
            finished_at: None,
        }
    }

    #[test]
    fn a_queued_run_has_no_current_step_and_no_usage() {
        let active = ActiveRun::assemble(overview(Run::MEETING_PIPELINE.to_vec()), &[]);

        assert_eq!(active.current_step, None);
        assert_eq!(active.steps_done, 0);
        assert_eq!(active.steps_total, 3);
        assert_eq!(
            (
                active.input_tokens,
                active.output_tokens,
                active.cost_micros
            ),
            (0, 0, 0)
        );
    }

    #[test]
    fn the_current_step_is_the_latest_running_one_and_usage_sums_every_step() {
        let overview = overview(Run::MEETING_PIPELINE.to_vec());
        let steps = [
            step(&overview.run, 0, StepStatus::Succeeded, (100, 10), 5),
            step(&overview.run, 1, StepStatus::Skipped, (0, 0), 0),
            step(&overview.run, 2, StepStatus::Running, (7, 3), 2),
        ];

        let active = ActiveRun::assemble(overview, &steps);

        assert_eq!(active.current_step.map(|step| step.seq), Some(2));
        assert_eq!(active.steps_done, 2);
        assert_eq!(active.input_tokens, 107);
        assert_eq!(active.output_tokens, 13);
        assert_eq!(active.cost_micros, 7);
    }

    #[test]
    fn a_failed_step_is_neither_done_nor_current() {
        let overview = overview(Run::MEETING_PIPELINE.to_vec());
        let steps = [step(&overview.run, 0, StepStatus::Failed, (1, 1), 1)];

        let active = ActiveRun::assemble(overview, &steps);

        assert_eq!(active.current_step, None);
        assert_eq!(active.steps_done, 0);
    }
}
