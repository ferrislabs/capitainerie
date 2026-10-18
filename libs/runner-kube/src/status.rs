use capitainerie_core::domain::execution::TaskOutcome;
use k8s_openapi::api::{batch::v1::Job, core::v1::Pod};

const DEADLINE_REASON: &str = "DeadlineExceeded";

pub fn has_started(pod: &Pod) -> bool {
    matches!(
        pod.status
            .as_ref()
            .and_then(|status| status.phase.as_deref()),
        Some("Running" | "Succeeded" | "Failed")
    )
}
pub fn outcome(job: &Job, pod: Option<&Pod>) -> Option<TaskOutcome> {
    let conditions = job
        .status
        .as_ref()?
        .conditions
        .as_deref()
        .unwrap_or_default();
    let holds = |kind: &str| {
        conditions
            .iter()
            .find(|condition| condition.type_ == kind && condition.status == "True")
    };
    if holds("Complete").is_some() {
        return Some(TaskOutcome::Succeeded);
    }
    let failed = holds("Failed")?;
    if failed.reason.as_deref() == Some(DEADLINE_REASON) {
        return Some(TaskOutcome::DeadlineExceeded);
    }
    Some(TaskOutcome::Failed {
        reason: pod.and_then(container_failure).unwrap_or_else(|| {
            condition_reason(failed.reason.as_deref(), failed.message.as_deref())
        }),
    })
}

fn container_failure(pod: &Pod) -> Option<String> {
    let terminated = pod
        .status
        .as_ref()?
        .container_statuses
        .as_deref()?
        .iter()
        .find_map(|status| status.state.as_ref()?.terminated.as_ref())?;
    let mut reason = format!("container exited with code {}", terminated.exit_code);
    for detail in [terminated.reason.as_deref(), terminated.message.as_deref()]
        .into_iter()
        .flatten()
        .filter(|detail| !detail.is_empty())
    {
        reason.push_str(": ");
        reason.push_str(detail);
    }
    Some(reason)
}

fn condition_reason(reason: Option<&str>, message: Option<&str>) -> String {
    match (reason, message) {
        (Some(reason), Some(message)) => format!("{reason}: {message}"),
        (Some(detail), None) | (None, Some(detail)) => detail.to_owned(),
        (None, None) => "the job failed".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn job(conditions: serde_json::Value) -> Job {
        serde_json::from_value(json!({
            "metadata": {"name": "run-1"},
            "status": {"conditions": conditions}
        }))
        .unwrap()
    }

    fn pod(phase: &str, terminated: serde_json::Value) -> Pod {
        serde_json::from_value(json!({
            "metadata": {"name": "run-1-abc"},
            "status": {"phase": phase, "containerStatuses": [{
                "name": "executor", "image": "i", "imageID": "", "ready": false,
                "restartCount": 0,
                "state": {"terminated": terminated}
            }]}
        }))
        .unwrap()
    }

    #[test]
    fn a_running_job_has_no_outcome() {
        assert_eq!(outcome(&job(json!([])), None), None);
        let no_status: Job = serde_json::from_value(json!({"metadata": {"name": "x"}})).unwrap();
        assert_eq!(outcome(&no_status, None), None);
        let unset = job(json!([{"type": "Failed", "status": "False"}]));
        assert_eq!(outcome(&unset, None), None);
    }

    #[test]
    fn a_complete_job_succeeded() {
        let done = job(json!([{"type": "Complete", "status": "True"}]));
        assert_eq!(outcome(&done, None), Some(TaskOutcome::Succeeded));
    }

    #[test]
    fn the_deadline_is_told_apart() {
        let late = job(json!([{
            "type": "Failed", "status": "True", "reason": "DeadlineExceeded",
            "message": "Job was active longer than specified deadline"
        }]));
        assert_eq!(outcome(&late, None), Some(TaskOutcome::DeadlineExceeded));
    }

    #[test]
    fn a_failure_reports_the_container_exit() {
        let failed = job(json!([{
            "type": "Failed", "status": "True", "reason": "BackoffLimitExceeded"
        }]));
        let pod = pod(
            "Failed",
            json!({"exitCode": 2, "reason": "Error", "message": "boom"}),
        );
        assert_eq!(
            outcome(&failed, Some(&pod)),
            Some(TaskOutcome::Failed {
                reason: "container exited with code 2: Error: boom".into()
            })
        );
    }

    #[test]
    fn a_failure_without_pod_falls_back_to_the_condition() {
        let failed = job(json!([{
            "type": "Failed", "status": "True", "reason": "BackoffLimitExceeded",
            "message": "Job has reached the specified backoff limit"
        }]));
        assert_eq!(
            outcome(&failed, None),
            Some(TaskOutcome::Failed {
                reason: "BackoffLimitExceeded: Job has reached the specified backoff limit".into()
            })
        );
    }

    #[test]
    fn a_pod_has_started_once_it_runs_or_ended() {
        let running = pod("Running", json!({"exitCode": 0}));
        assert!(has_started(&running));
        let mut pending = pod("Pending", json!({"exitCode": 0}));
        assert!(!has_started(&pending));
        pending.status = None;
        assert!(!has_started(&pending));
    }
}
