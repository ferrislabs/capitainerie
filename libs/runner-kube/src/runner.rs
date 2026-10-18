use std::time::Duration;

use capitainerie_common::{CoreError, RunId};
use capitainerie_core::domain::execution::{TaskLine, TaskOutcome, TaskSpec, ports::TaskRunner};
use futures_util::{AsyncBufReadExt, StreamExt};
use k8s_openapi::api::{
    batch::v1::Job,
    core::v1::{Pod, Secret},
};
use kube::{
    Api, Client, Resource,
    api::{DeleteParams, ListParams, LogParams, PostParams},
};
use tokio::sync::mpsc;

use crate::{
    config::KubeRunnerConfig,
    error::RunnerError,
    logs::parse_line,
    manifest::{build_job, build_secret, job_name, run_selector},
    status::{has_started, outcome},
};

const POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct KubeTaskRunner {
    client: Client,
    config: KubeRunnerConfig,
}

fn is_status(error: &kube::Error, code: u16) -> bool {
    matches!(error, kube::Error::Api(status) if status.code == code)
}

impl KubeTaskRunner {
    pub fn new(client: Client, config: KubeRunnerConfig) -> Self {
        Self { client, config }
    }
    pub async fn try_default(config: KubeRunnerConfig) -> Result<Self, RunnerError> {
        Ok(Self::new(Client::try_default().await?, config))
    }

    fn jobs(&self) -> Api<Job> {
        Api::namespaced(self.client.clone(), &self.config.namespace)
    }

    fn pods(&self) -> Api<Pod> {
        Api::namespaced(self.client.clone(), &self.config.namespace)
    }

    fn secrets(&self) -> Api<Secret> {
        Api::namespaced(self.client.clone(), &self.config.namespace)
    }

    async fn create_job(&self, spec: &TaskSpec) -> Result<Job, RunnerError> {
        let name = job_name(spec.run_id);
        match self
            .jobs()
            .create(&PostParams::default(), &build_job(&self.config, spec))
            .await
        {
            Ok(job) => Ok(job),
            Err(error) if is_status(&error, 409) => Ok(self.jobs().get(&name).await?),
            Err(error) => Err(error.into()),
        }
    }

    async fn start_task(&self, spec: &TaskSpec) -> Result<(), RunnerError> {
        let job = self.create_job(spec).await?;
        let owner = job
            .controller_owner_ref(&())
            .ok_or_else(|| RunnerError::NoJobUid(job_name(spec.run_id)))?;
        let secret = build_secret(&self.config, spec, owner);
        match self.secrets().create(&PostParams::default(), &secret).await {
            Ok(_) => Ok(()),
            Err(error) if is_status(&error, 409) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    async fn pod_of(&self, run_id: RunId) -> Result<Option<Pod>, RunnerError> {
        let list = self
            .pods()
            .list(&ListParams::default().labels(&run_selector(run_id)))
            .await?;
        Ok(list.items.into_iter().next())
    }

    async fn job(&self, name: &str) -> Result<Job, RunnerError> {
        match self.jobs().get(name).await {
            Err(error) if is_status(&error, 404) => Err(RunnerError::JobGone(name.to_owned())),
            other => Ok(other?),
        }
    }

    async fn wait_for_pod(&self, run_id: RunId) -> Result<Result<Pod, TaskOutcome>, RunnerError> {
        let name = job_name(run_id);
        loop {
            let pod = self.pod_of(run_id).await?;
            if let Some(pod) = pod.filter(has_started) {
                return Ok(Ok(pod));
            }
            let job = self.job(&name).await?;
            if let Some(ended) = outcome(&job, None) {
                return Ok(Err(ended));
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    async fn wait_for_outcome(&self, run_id: RunId) -> Result<TaskOutcome, RunnerError> {
        let name = job_name(run_id);
        loop {
            let job = self.job(&name).await?;
            if let Some(ended) = outcome(&job, self.pod_of(run_id).await?.as_ref()) {
                return Ok(ended);
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    async fn follow_task(
        &self,
        run_id: RunId,
        lines: mpsc::Sender<TaskLine>,
    ) -> Result<TaskOutcome, RunnerError> {
        let pod = match self.wait_for_pod(run_id).await? {
            Ok(pod) => pod,
            Err(ended) => return Ok(ended),
        };
        let Some(pod_name) = pod.metadata.name else {
            return self.wait_for_outcome(run_id).await;
        };
        let params = LogParams {
            follow: true,
            ..Default::default()
        };
        let mut stream = self.pods().log_stream(&pod_name, &params).await?.lines();
        let mut listening = true;
        while let Some(line) = stream.next().await {
            let Some(parsed) = parse_line(&line?) else {
                continue;
            };
            if listening && lines.send(parsed).await.is_err() {
                listening = false;
            }
        }
        self.wait_for_outcome(run_id).await
    }
}

impl TaskRunner for KubeTaskRunner {
    async fn start(&self, spec: &TaskSpec) -> Result<(), CoreError> {
        Ok(self.start_task(spec).await?)
    }

    async fn follow(
        &self,
        run_id: RunId,
        lines: mpsc::Sender<TaskLine>,
    ) -> Result<TaskOutcome, CoreError> {
        Ok(self.follow_task(run_id, lines).await?)
    }

    async fn delete(&self, run_id: RunId) -> Result<(), CoreError> {
        match self
            .jobs()
            .delete(&job_name(run_id), &DeleteParams::background())
            .await
        {
            Ok(_) => Ok(()),
            Err(error) if is_status(&error, 404) => Ok(()),
            Err(error) => Err(RunnerError::from(error).into()),
        }
    }
}
