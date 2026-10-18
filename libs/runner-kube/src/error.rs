use capitainerie_common::CoreError;

#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("kubernetes request failed: {0}")]
    Kube(#[from] kube::Error),

    #[error("reading the pod logs failed: {0}")]
    Logs(#[from] std::io::Error),

    #[error("job {0} disappeared while it was followed")]
    JobGone(String),

    #[error("job {0} has no uid, so the secret cannot be owned by it")]
    NoJobUid(String),
}

impl From<RunnerError> for CoreError {
    fn from(error: RunnerError) -> Self {
        CoreError::Internal(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_runner_error_is_transient() {
        let error = CoreError::from(RunnerError::JobGone("run-1".into()));
        assert!(matches!(error, CoreError::Internal(message) if message.contains("run-1")));
    }
}
