#![allow(clippy::unwrap_used)]

use std::time::Duration;

use capitainerie_common::RunId;
use capitainerie_core::domain::{
    agent::SecretToken,
    execution::{
        ExecutorModel, RepositoryRef, TaskLine, TaskMarker, TaskOutcome, TaskSpec,
        ports::TaskRunner,
    },
};
use capitainerie_runner_kube::{KubeRunnerConfig, KubeTaskRunner, job_name};
use k8s_openapi::api::{
    batch::v1::Job,
    core::v1::{Namespace, Secret},
};
use kube::{
    Api, Client,
    api::{ObjectMeta, PostParams},
};
use tokio::sync::mpsc;

const SCRIPT: &str = r#"echo '{"type":"capitainerie","event":"cloned"}'; echo '{"type":"assistant","n":1}'; echo not json; echo '{"type":"capitainerie","event":"no_changes"}'"#;

fn enabled() -> bool {
    std::env::var("REQUIRE_KUBE").as_deref() == Ok("1")
}

fn spec(deadline: Duration) -> TaskSpec {
    TaskSpec {
        run_id: RunId::new(),
        repository: "acme/web".parse::<RepositoryRef>().unwrap(),
        base_branch: "main".into(),
        branch: "capitainerie/test".into(),
        prompt: "test".into(),
        model: ExecutorModel::default(),
        max_turns: 1,
        deadline,
        task_token: SecretToken::new("tt_integration"),
    }
}

async fn runner(script: &str) -> (KubeTaskRunner, Client, String) {
    let client = Client::try_default().await.unwrap();
    let mut config = KubeRunnerConfig::new("busybox:1.37", "http://10.0.0.1:4130");
    config.namespace = std::env::var("KUBE_TEST_NAMESPACE").unwrap_or(config.namespace);
    config.command = Some(vec!["sh".into(), "-c".into(), script.into()]);
    let namespaces: Api<Namespace> = Api::all(client.clone());
    let namespace = Namespace {
        metadata: ObjectMeta {
            name: Some(config.namespace.clone()),
            ..Default::default()
        },
        ..Default::default()
    };
    let _ = namespaces.create(&PostParams::default(), &namespace).await;
    let name = config.namespace.clone();
    (KubeTaskRunner::new(client.clone(), config), client, name)
}

async fn gone(jobs: &Api<Job>, name: &str) -> bool {
    for _ in 0..30 {
        if jobs.get_opt(name).await.unwrap().is_none() {
            return true;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    false
}

#[tokio::test]
async fn a_job_runs_streams_its_lines_and_is_deleted() {
    if !enabled() {
        return;
    }
    let (runner, client, namespace) = runner(SCRIPT).await;
    let spec = spec(Duration::from_secs(120));
    assert!(!runner.attach(spec.run_id).await.unwrap());
    runner.start(&spec).await.unwrap();
    runner.start(&spec).await.unwrap();
    assert!(runner.attach(spec.run_id).await.unwrap());

    let secrets: Api<Secret> = Api::namespaced(client.clone(), &namespace);
    let secret = secrets.get(&job_name(spec.run_id)).await.unwrap();
    assert_eq!(secret.metadata.owner_references.unwrap()[0].kind, "Job");

    let (sender, mut receiver) = mpsc::channel(16);
    let outcome = runner.follow(spec.run_id, sender).await.unwrap();
    assert_eq!(outcome, TaskOutcome::Succeeded);
    let mut lines = Vec::new();
    while let Some(line) = receiver.recv().await {
        lines.push(line);
    }
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert_eq!(lines[0], TaskLine::Capitainerie(TaskMarker::Cloned));
    assert!(matches!(lines[1], TaskLine::Agent(_)));
    assert_eq!(lines[2], TaskLine::Capitainerie(TaskMarker::NoChanges));

    runner.delete(spec.run_id).await.unwrap();
    runner.delete(spec.run_id).await.unwrap();
    let jobs: Api<Job> = Api::namespaced(client, &namespace);
    assert!(gone(&jobs, &job_name(spec.run_id)).await);
    assert!(!runner.attach(spec.run_id).await.unwrap());
}

#[tokio::test]
async fn a_dropped_receiver_does_not_stop_the_follow() {
    if !enabled() {
        return;
    }
    let (runner, _, _) = runner(SCRIPT).await;
    let spec = spec(Duration::from_secs(120));
    runner.start(&spec).await.unwrap();
    let (sender, receiver) = mpsc::channel(1);
    drop(receiver);
    let outcome = runner.follow(spec.run_id, sender).await.unwrap();
    assert_eq!(outcome, TaskOutcome::Succeeded);
    runner.delete(spec.run_id).await.unwrap();
}

#[tokio::test]
async fn a_task_past_its_deadline_is_reported() {
    if !enabled() {
        return;
    }
    let (runner, _, _) = runner("sleep 120").await;
    let spec = spec(Duration::from_secs(3));
    runner.start(&spec).await.unwrap();
    let (sender, _receiver) = mpsc::channel(1);
    let outcome = runner.follow(spec.run_id, sender).await.unwrap();
    assert_eq!(outcome, TaskOutcome::DeadlineExceeded);
    runner.delete(spec.run_id).await.unwrap();
}

#[tokio::test]
async fn a_failing_task_reports_its_exit_code() {
    if !enabled() {
        return;
    }
    let (runner, _, _) = runner("exit 3").await;
    let spec = spec(Duration::from_secs(120));
    runner.start(&spec).await.unwrap();
    let (sender, _receiver) = mpsc::channel(1);
    let outcome = runner.follow(spec.run_id, sender).await.unwrap();
    assert!(
        matches!(&outcome, TaskOutcome::Failed { reason } if reason.contains("code 3")),
        "{outcome:?}"
    );
    runner.delete(spec.run_id).await.unwrap();
}
