use std::{collections::BTreeMap, net::IpAddr};

use capitainerie_common::RunId;
use capitainerie_core::domain::execution::{
    PLAN_PATH, REPORT_PATH, TaskSpec, contribution::CONTRIBUTION_PATH, harness::Harness,
};
use k8s_openapi::{
    api::{
        batch::v1::{Job, JobSpec},
        core::v1::{
            Capabilities, Container, EmptyDirVolumeSource, EnvVar, EnvVarSource, HostAlias,
            PodDNSConfig, PodSecurityContext, PodSpec, PodTemplateSpec, SeccompProfile, Secret,
            SecretKeySelector, SecurityContext, Volume, VolumeMount,
        },
    },
    apimachinery::pkg::{
        api::resource::Quantity,
        apis::meta::v1::{ObjectMeta, OwnerReference},
    },
};

use crate::{config::KubeRunnerConfig, error::RunnerError};

pub const PART_OF_LABEL: &str = "app.kubernetes.io/part-of";
pub const RUN_ID_LABEL: &str = "capitainerie.dev/run-id";
pub const TOKEN_KEY: &str = "task-token";
const EXECUTOR_UID: i64 = 1000;
const CODEX_HOME: &str = "/home/executor/.codex";
const VIBE_HOME: &str = "/home/executor/.vibe";
const WRITABLE_DIRS: [(&str, &str, &str); 3] = [
    ("workspace", "/workspace", "4Gi"),
    ("tmp", "/tmp", "1Gi"),
    ("home", "/home/executor", "1Gi"),
];

/// The proxy's address, resolved by the worker: the pod resolves nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyHost {
    pub name: String,
    pub ip: IpAddr,
}

pub fn job_name(run_id: RunId) -> String {
    format!("run-{}", run_id.0)
}

pub fn run_selector(run_id: RunId) -> String {
    format!("{RUN_ID_LABEL}={}", run_id.0)
}

fn labels(run_id: RunId) -> BTreeMap<String, String> {
    BTreeMap::from([
        (PART_OF_LABEL.to_owned(), "capitainerie".to_owned()),
        (RUN_ID_LABEL.to_owned(), run_id.0.to_string()),
    ])
}

fn plain(name: &str, value: impl Into<String>) -> EnvVar {
    EnvVar {
        name: name.to_owned(),
        value: Some(value.into()),
        value_from: None,
    }
}

fn from_secret(name: &str, secret: &str) -> EnvVar {
    EnvVar {
        name: name.to_owned(),
        value: None,
        value_from: Some(EnvVarSource {
            secret_key_ref: Some(SecretKeySelector {
                name: secret.to_owned(),
                key: TOKEN_KEY.to_owned(),
                optional: Some(false),
            }),
            ..Default::default()
        }),
    }
}

fn environment(config: &KubeRunnerConfig, spec: &TaskSpec, secret: &str) -> Vec<EnvVar> {
    let proxy = &config.proxy_base_url;
    let mut variables = vec![
        from_secret("CAPITAINERIE_TASK_TOKEN", secret),
        plain(
            "GIT_REMOTE",
            format!(
                "{proxy}/git/{}/{}.git",
                spec.repository.owner(),
                spec.repository.name()
            ),
        ),
        plain("REPOSITORY", spec.repository.to_string()),
        plain("BASE_BRANCH", &spec.base_branch),
        plain("BRANCH", &spec.branch),
        plain("MODEL", cli_model(spec)),
        plain("CAPITAINERIE_MODE", spec.mode.as_str()),
        plain("REPORT_PATH", REPORT_PATH),
        plain("PLAN_PATH", PLAN_PATH),
        plain("CONTRIBUTION_PATH", CONTRIBUTION_PATH),
        plain("COMMIT_AUTHOR_NAME", &spec.commit.author.name),
        plain("COMMIT_AUTHOR_EMAIL", &spec.commit.author.email),
        plain("COMMIT_FALLBACK_MESSAGE", &spec.commit.fallback_message),
        plain("LEAK_TERMS", &spec.commit.leak_terms),
        plain("MAX_TURNS", spec.max_turns.to_string()),
        plain("PROMPT", &spec.prompt),
        plain("HOME", "/home/executor"),
    ];
    variables.extend(harness_environment(config, spec, secret));
    variables
}

fn cli_model(spec: &TaskSpec) -> &str {
    let model = spec.model.as_str();
    match spec.model.harness() {
        Harness::MistralVibe => Harness::MistralVibe.wire_model(model),
        Harness::ClaudeCode | Harness::Codex | Harness::Antigravity => model,
    }
}

fn harness_environment(config: &KubeRunnerConfig, spec: &TaskSpec, secret: &str) -> Vec<EnvVar> {
    let proxy = &config.proxy_base_url;
    let model = spec.model.as_str();
    let harness = spec.model.harness();
    let mut variables = vec![plain("CAPITAINERIE_HARNESS", harness.as_str())];
    match harness {
        Harness::ClaudeCode => variables.extend([
            plain("ANTHROPIC_BASE_URL", format!("{proxy}/anthropic")),
            from_secret("ANTHROPIC_API_KEY", secret),
            // Claude Code asks a small model for background work; the proxy
            // serves only the run's model, so every role points at it.
            plain("ANTHROPIC_SMALL_FAST_MODEL", model),
            plain("ANTHROPIC_DEFAULT_HAIKU_MODEL", model),
            plain("CLAUDE_CODE_SUBAGENT_MODEL", model),
        ]),
        Harness::Codex => variables.extend([
            plain("CAPITAINERIE_OPENAI_BASE_URL", format!("{proxy}/openai/v1")),
            from_secret("CODEX_API_KEY", secret),
            plain("CODEX_HOME", CODEX_HOME),
        ]),
        Harness::MistralVibe => variables.extend([
            plain("CAPITAINERIE_MISTRAL_BASE_URL", format!("{proxy}/mistral")),
            from_secret("MISTRAL_API_KEY", secret),
            plain("VIBE_HOME", VIBE_HOME),
        ]),
        Harness::Antigravity => variables.extend([
            plain("GOOGLE_GEMINI_BASE_URL", format!("{proxy}/google")),
            from_secret("GEMINI_API_KEY", secret),
            plain("AGY_CLI_DISABLE_AUTO_UPDATE", "true"),
        ]),
    }
    variables
}

fn image(config: &KubeRunnerConfig, harness: Harness) -> Result<&str, RunnerError> {
    match harness {
        Harness::ClaudeCode => Ok(&config.image),
        Harness::Codex => config
            .codex_image
            .as_deref()
            .ok_or(RunnerError::NoHarnessImage(harness.as_str())),
        Harness::Antigravity => config
            .antigravity_image
            .as_deref()
            .ok_or(RunnerError::NoHarnessImage(harness.as_str())),
        Harness::MistralVibe => config
            .mistral_vibe_image
            .as_deref()
            .ok_or(RunnerError::NoHarnessImage(harness.as_str())),
    }
}

pub fn build_job(
    config: &KubeRunnerConfig,
    spec: &TaskSpec,
    proxy: Option<&ProxyHost>,
) -> Result<Job, RunnerError> {
    let name = job_name(spec.run_id);
    let labels = labels(spec.run_id);
    let container = Container {
        name: "executor".to_owned(),
        image: Some(image(config, spec.model.harness())?.to_owned()),
        command: config.command.clone(),
        env: Some(environment(config, spec, &name)),
        resources: Some(config.resources.clone()),
        security_context: Some(SecurityContext {
            allow_privilege_escalation: Some(false),
            read_only_root_filesystem: Some(true),
            run_as_non_root: Some(true),
            run_as_user: Some(EXECUTOR_UID),
            capabilities: Some(Capabilities {
                drop: Some(vec!["ALL".to_owned()]),
                add: None,
            }),
            ..Default::default()
        }),
        volume_mounts: Some(
            WRITABLE_DIRS
                .iter()
                .map(|(volume, path, _)| VolumeMount {
                    name: (*volume).to_owned(),
                    mount_path: (*path).to_owned(),
                    ..Default::default()
                })
                .collect(),
        ),
        ..Default::default()
    };
    Ok(Job {
        metadata: ObjectMeta {
            name: Some(name),
            namespace: Some(config.namespace.clone()),
            labels: Some(labels.clone()),
            ..Default::default()
        },
        spec: Some(JobSpec {
            backoff_limit: Some(0),
            active_deadline_seconds: Some(
                i64::try_from(spec.deadline.as_secs()).unwrap_or(i64::MAX),
            ),
            ttl_seconds_after_finished: Some(config.ttl_seconds_after_finished),
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(labels),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    restart_policy: Some("Never".to_owned()),
                    automount_service_account_token: Some(false),
                    security_context: Some(PodSecurityContext {
                        run_as_non_root: Some(true),
                        run_as_user: Some(EXECUTOR_UID),
                        run_as_group: Some(EXECUTOR_UID),
                        fs_group: Some(EXECUTOR_UID),
                        seccomp_profile: Some(SeccompProfile {
                            type_: "RuntimeDefault".to_owned(),
                            localhost_profile: None,
                        }),
                        ..Default::default()
                    }),
                    dns_policy: Some("None".to_owned()),
                    dns_config: Some(PodDNSConfig {
                        nameservers: Some(vec!["127.0.0.1".to_owned()]),
                        ..Default::default()
                    }),
                    host_aliases: proxy.map(|proxy| {
                        vec![HostAlias {
                            ip: proxy.ip.to_string(),
                            hostnames: Some(vec![proxy.name.clone()]),
                        }]
                    }),
                    containers: vec![container],
                    volumes: Some(
                        WRITABLE_DIRS
                            .iter()
                            .map(|(volume, _, limit)| Volume {
                                name: (*volume).to_owned(),
                                empty_dir: Some(EmptyDirVolumeSource {
                                    size_limit: Some(Quantity((*limit).to_owned())),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            })
                            .collect(),
                    ),
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
        status: None,
    })
}

pub fn build_secret(config: &KubeRunnerConfig, spec: &TaskSpec, owner: OwnerReference) -> Secret {
    Secret {
        metadata: ObjectMeta {
            name: Some(job_name(spec.run_id)),
            namespace: Some(config.namespace.clone()),
            labels: Some(labels(spec.run_id)),
            owner_references: Some(vec![owner]),
            ..Default::default()
        },
        string_data: Some(BTreeMap::from([(
            TOKEN_KEY.to_owned(),
            spec.task_token.expose().to_owned(),
        )])),
        type_: Some("Opaque".to_owned()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use capitainerie_core::domain::{
        agent::SecretToken,
        execution::{CommitAuthor, CommitSettings, ExecutionMode, ExecutorModel, RepositoryRef},
    };

    use super::*;

    const TOKEN: &str = "tt_super_secret_value";

    fn spec() -> TaskSpec {
        TaskSpec {
            run_id: RunId(uuid::Uuid::parse_str("0a1b2c3d-0000-7000-8000-000000000000").unwrap()),
            repository: "acme/web".parse::<RepositoryRef>().unwrap(),
            base_branch: "main".into(),
            branch: "change/fix-the-bug-4e5f00".into(),
            prompt: "Fix the bug".into(),
            model: ExecutorModel::default(),
            max_turns: 50,
            deadline: Duration::from_secs(1800),
            mode: ExecutionMode::Change,
            task_token: SecretToken::new(TOKEN),
            commit: CommitSettings {
                author: CommitAuthor {
                    name: "Ada Lovelace".into(),
                    email: "ada@example.test".into(),
                },
                leak_terms: "claude-opus-5-5\ncapitainerie".into(),
                fallback_message: "Fix the bug".into(),
            },
        }
    }

    fn config() -> KubeRunnerConfig {
        KubeRunnerConfig::new(
            "capitainerie-executor:1",
            "http://capitainerie-egress-proxy.capitainerie.svc:4130",
        )
    }

    fn codex_config() -> KubeRunnerConfig {
        config().with_codex_image(Some("capitainerie-executor-codex:1".into()))
    }

    fn codex_spec() -> TaskSpec {
        TaskSpec {
            model: ExecutorModel::new(Harness::Codex, Harness::Codex.default_model()).unwrap(),
            ..spec()
        }
    }

    fn antigravity_config() -> KubeRunnerConfig {
        config().with_antigravity_image(Some("capitainerie-executor-antigravity:1".into()))
    }

    fn antigravity_spec() -> TaskSpec {
        TaskSpec {
            model: ExecutorModel::new(Harness::Antigravity, Harness::Antigravity.default_model())
                .unwrap(),
            ..spec()
        }
    }

    fn vibe_config() -> KubeRunnerConfig {
        config().with_mistral_vibe_image(Some("capitainerie-executor-mistral-vibe:1".into()))
    }

    fn vibe_spec() -> TaskSpec {
        TaskSpec {
            model: ExecutorModel::new(Harness::MistralVibe, "mistral-medium-3.5").unwrap(),
            ..spec()
        }
    }

    fn proxy() -> ProxyHost {
        ProxyHost {
            name: "capitainerie-egress-proxy.capitainerie.svc".into(),
            ip: "10.96.0.12".parse().unwrap(),
        }
    }

    fn pod(job: &Job) -> &PodSpec {
        job.spec
            .as_ref()
            .and_then(|spec| spec.template.spec.as_ref())
            .unwrap()
    }

    fn env<'a>(job: &'a Job, name: &str) -> &'a EnvVar {
        pod(job).containers[0]
            .env
            .as_ref()
            .unwrap()
            .iter()
            .find(|var| var.name == name)
            .unwrap()
    }

    #[test]
    fn the_job_is_named_after_the_run_and_bounded() {
        let job = build_job(&config(), &spec(), Some(&proxy())).unwrap();
        assert_eq!(
            job.metadata.name.as_deref(),
            Some("run-0a1b2c3d-0000-7000-8000-000000000000")
        );
        assert_eq!(
            job.metadata.namespace.as_deref(),
            Some("capitainerie-tasks")
        );
        let job_spec = job.spec.as_ref().unwrap();
        assert_eq!(job_spec.backoff_limit, Some(0));
        assert_eq!(job_spec.active_deadline_seconds, Some(1800));
        assert_eq!(job_spec.ttl_seconds_after_finished, Some(1800));
        assert_eq!(env(&job, "MAX_TURNS").value.as_deref(), Some("50"));
    }

    #[test]
    fn job_and_pod_carry_the_labels() {
        let job = build_job(&config(), &spec(), Some(&proxy())).unwrap();
        let template_labels = job
            .spec
            .as_ref()
            .and_then(|spec| spec.template.metadata.as_ref())
            .and_then(|meta| meta.labels.as_ref())
            .unwrap();
        for labels in [job.metadata.labels.as_ref().unwrap(), template_labels] {
            assert_eq!(labels[PART_OF_LABEL], "capitainerie");
            assert_eq!(labels[RUN_ID_LABEL], "0a1b2c3d-0000-7000-8000-000000000000");
        }
    }

    #[test]
    fn the_pod_is_locked_down() {
        let job = build_job(&config(), &spec(), Some(&proxy())).unwrap();
        let pod = pod(&job);
        assert_eq!(pod.restart_policy.as_deref(), Some("Never"));
        assert_eq!(pod.automount_service_account_token, Some(false));
        let pod_context = pod.security_context.as_ref().unwrap();
        assert_eq!(pod_context.run_as_non_root, Some(true));
        assert_eq!(pod_context.run_as_user, Some(1000));
        assert_eq!(
            pod_context.seccomp_profile.as_ref().unwrap().type_,
            "RuntimeDefault"
        );
        let context = pod.containers[0].security_context.as_ref().unwrap();
        assert_eq!(context.run_as_non_root, Some(true));
        assert_eq!(context.run_as_user, Some(1000));
        assert_eq!(context.read_only_root_filesystem, Some(true));
        assert_eq!(context.allow_privilege_escalation, Some(false));
        assert_eq!(
            context.capabilities.as_ref().unwrap().drop,
            Some(vec!["ALL".to_owned()])
        );
        assert_eq!(context.privileged, None);
    }

    #[test]
    fn the_pod_resolves_the_proxy_from_its_hosts_file_and_nothing_else() {
        let job = build_job(&config(), &spec(), Some(&proxy())).unwrap();
        let spec_of_pod = pod(&job);
        assert_eq!(spec_of_pod.dns_policy.as_deref(), Some("None"));
        assert_eq!(
            spec_of_pod.dns_config.as_ref().unwrap().nameservers,
            Some(vec!["127.0.0.1".to_owned()])
        );
        let aliases = spec_of_pod.host_aliases.as_ref().unwrap();
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].ip, "10.96.0.12");
        assert_eq!(
            aliases[0].hostnames,
            Some(vec![
                "capitainerie-egress-proxy.capitainerie.svc".to_owned()
            ])
        );
        let literal = build_job(&config(), &spec(), None).unwrap();
        assert_eq!(pod(&literal).host_aliases, None);
        assert_eq!(pod(&literal).dns_policy.as_deref(), Some("None"));
    }

    #[test]
    fn the_writable_paths_and_the_container_are_size_bounded() {
        let job = build_job(&config(), &spec(), Some(&proxy())).unwrap();
        let pod = pod(&job);
        let limits: BTreeMap<_, _> = pod
            .volumes
            .as_ref()
            .unwrap()
            .iter()
            .map(|volume| {
                (
                    volume.name.as_str(),
                    volume
                        .empty_dir
                        .as_ref()
                        .unwrap()
                        .size_limit
                        .as_ref()
                        .unwrap()
                        .0
                        .as_str(),
                )
            })
            .collect();
        assert_eq!(limits["workspace"], "4Gi");
        assert_eq!(limits["tmp"], "1Gi");
        assert_eq!(limits["home"], "1Gi");
        let container = pod.containers[0].resources.as_ref().unwrap();
        assert_eq!(
            container.limits.as_ref().unwrap()["ephemeral-storage"].0,
            "6Gi"
        );
    }

    #[test]
    fn the_only_writable_paths_are_empty_dirs() {
        let job = build_job(&config(), &spec(), Some(&proxy())).unwrap();
        let pod = pod(&job);
        let volumes = pod.volumes.as_ref().unwrap();
        assert!(volumes.iter().all(|volume| volume.empty_dir.is_some()));
        let mut paths: Vec<_> = pod.containers[0]
            .volume_mounts
            .as_ref()
            .unwrap()
            .iter()
            .map(|mount| mount.mount_path.as_str())
            .collect();
        paths.sort_unstable();
        assert_eq!(paths, ["/home/executor", "/tmp", "/workspace"]);
        assert!(
            pod.containers[0]
                .volume_mounts
                .as_ref()
                .unwrap()
                .iter()
                .all(|mount| volumes.iter().any(|volume| volume.name == mount.name))
        );
    }

    #[test]
    fn the_task_token_is_the_only_secret_and_never_inline() {
        let job = build_job(&config(), &spec(), Some(&proxy())).unwrap();
        let secret_name = "run-0a1b2c3d-0000-7000-8000-000000000000";
        let mut referenced = Vec::new();
        for var in pod(&job).containers[0].env.as_ref().unwrap() {
            match (&var.value, &var.value_from) {
                (Some(value), None) => assert!(!value.contains(TOKEN), "{}", var.name),
                (None, Some(source)) => {
                    let key = source.secret_key_ref.as_ref().unwrap();
                    assert_eq!(key.name, secret_name);
                    assert_eq!(key.key, TOKEN_KEY);
                    referenced.push(var.name.as_str());
                }
                _ => panic!("env {} is neither a value nor a reference", var.name),
            }
        }
        referenced.sort_unstable();
        assert_eq!(referenced, ["ANTHROPIC_API_KEY", "CAPITAINERIE_TASK_TOKEN"]);
        let rendered = serde_json::to_string(&job).unwrap();
        assert!(!rendered.contains(TOKEN));
    }

    #[test]
    fn the_environment_points_at_the_proxy() {
        let job = build_job(&config(), &spec(), Some(&proxy())).unwrap();
        let proxy = "http://capitainerie-egress-proxy.capitainerie.svc:4130";
        assert_eq!(
            env(&job, "ANTHROPIC_BASE_URL").value.as_deref(),
            Some(format!("{proxy}/anthropic").as_str())
        );
        assert_eq!(
            env(&job, "GIT_REMOTE").value.as_deref(),
            Some(format!("{proxy}/git/acme/web.git").as_str())
        );
        assert_eq!(env(&job, "REPOSITORY").value.as_deref(), Some("acme/web"));
        assert_eq!(env(&job, "BASE_BRANCH").value.as_deref(), Some("main"));
        assert_eq!(
            env(&job, "BRANCH").value.as_deref(),
            Some("change/fix-the-bug-4e5f00")
        );
        assert_eq!(
            env(&job, "MODEL").value.as_deref(),
            Some(Harness::ClaudeCode.default_model())
        );
        assert_eq!(env(&job, "PROMPT").value.as_deref(), Some("Fix the bug"));
    }

    #[test]
    fn the_mode_and_the_report_path_reach_the_container() {
        let change = build_job(&config(), &spec(), Some(&proxy())).unwrap();
        assert_eq!(
            env(&change, "CAPITAINERIE_MODE").value.as_deref(),
            Some("change")
        );
        let audit = TaskSpec {
            mode: ExecutionMode::Audit,
            ..spec()
        };
        let job = build_job(&config(), &audit, Some(&proxy())).unwrap();
        assert_eq!(
            env(&job, "CAPITAINERIE_MODE").value.as_deref(),
            Some("audit")
        );
        let report = env(&job, "REPORT_PATH").value.clone().unwrap();
        assert_eq!(report, "/workspace/.capitainerie/report.md");
        assert!(report.starts_with("/workspace/"));
        assert!(!report.starts_with("/workspace/repo/"));
        assert_eq!(
            env(&job, "PLAN_PATH").value.as_deref(),
            Some("/workspace/.capitainerie/plan.md")
        );
    }

    #[test]
    fn the_commit_author_the_leak_terms_and_the_fallback_reach_the_container() {
        let job = build_job(&config(), &spec(), Some(&proxy())).unwrap();
        let value = |name| env(&job, name).value.clone().unwrap();
        assert_eq!(value("COMMIT_AUTHOR_NAME"), "Ada Lovelace");
        assert_eq!(value("COMMIT_AUTHOR_EMAIL"), "ada@example.test");
        assert_eq!(value("COMMIT_FALLBACK_MESSAGE"), "Fix the bug");
        assert_eq!(value("LEAK_TERMS"), "claude-opus-5-5\ncapitainerie");
        assert_eq!(
            value("CONTRIBUTION_PATH"),
            "/workspace/.capitainerie/contribution.md"
        );
    }

    #[test]
    fn every_model_role_of_claude_code_is_the_run_model() {
        let spec = TaskSpec {
            model: ExecutorModel::new(Harness::ClaudeCode, "claude-opus-5").unwrap(),
            ..spec()
        };
        let job = build_job(&config(), &spec, Some(&proxy())).unwrap();
        for name in [
            "MODEL",
            "ANTHROPIC_SMALL_FAST_MODEL",
            "ANTHROPIC_DEFAULT_HAIKU_MODEL",
            "CLAUDE_CODE_SUBAGENT_MODEL",
        ] {
            assert_eq!(
                env(&job, name).value.as_deref(),
                Some("claude-opus-5"),
                "{name}"
            );
        }
    }

    #[test]
    fn the_secret_holds_the_token_and_is_owned_by_the_job() {
        let owner = OwnerReference {
            api_version: "batch/v1".into(),
            kind: "Job".into(),
            name: "run-x".into(),
            uid: "uid-1".into(),
            ..Default::default()
        };
        let secret = build_secret(&config(), &spec(), owner.clone());
        assert_eq!(
            secret.metadata.name.as_deref(),
            Some("run-0a1b2c3d-0000-7000-8000-000000000000")
        );
        assert_eq!(secret.metadata.owner_references, Some(vec![owner]));
        assert_eq!(secret.string_data.unwrap()[TOKEN_KEY], TOKEN);
    }

    fn assert_no_inline_secret(job: &Job) {
        let secret_name = "run-0a1b2c3d-0000-7000-8000-000000000000";
        for var in pod(job).containers[0].env.as_ref().unwrap() {
            match (&var.value, &var.value_from) {
                (Some(value), None) => assert!(!value.contains(TOKEN), "{}", var.name),
                (None, Some(source)) => {
                    let key = source.secret_key_ref.as_ref().unwrap();
                    assert_eq!(key.name, secret_name, "{}", var.name);
                    assert_eq!(key.key, TOKEN_KEY, "{}", var.name);
                }
                _ => panic!("env {} is neither a value nor a reference", var.name),
            }
        }
        assert!(!serde_json::to_string(job).unwrap().contains(TOKEN));
    }

    #[test]
    fn every_harness_pod_holds_only_literals_and_references_to_the_task_secret() {
        for (config, spec) in [
            (config(), spec()),
            (codex_config(), codex_spec()),
            (antigravity_config(), antigravity_spec()),
            (vibe_config(), vibe_spec()),
        ] {
            let job = build_job(&config, &spec, Some(&proxy())).unwrap();
            assert_no_inline_secret(&job);
            assert_eq!(
                env(&job, "CAPITAINERIE_HARNESS").value.as_deref(),
                Some(spec.model.harness().as_str())
            );
        }
    }

    #[test]
    fn a_codex_pod_talks_to_the_openai_route_of_the_proxy() {
        let job = build_job(&codex_config(), &codex_spec(), Some(&proxy())).unwrap();
        assert_eq!(
            env(&job, "CAPITAINERIE_OPENAI_BASE_URL").value.as_deref(),
            Some("http://capitainerie-egress-proxy.capitainerie.svc:4130/openai/v1")
        );
        assert_eq!(
            env(&job, "CODEX_HOME").value.as_deref(),
            Some("/home/executor/.codex")
        );
        let key = env(&job, "CODEX_API_KEY");
        assert_eq!(key.value, None);
        assert_eq!(
            key.value_from
                .as_ref()
                .and_then(|source| source.secret_key_ref.as_ref())
                .map(|key| key.key.as_str()),
            Some(TOKEN_KEY)
        );
        let names: Vec<_> = pod(&job).containers[0]
            .env
            .as_ref()
            .unwrap()
            .iter()
            .map(|var| var.name.as_str())
            .collect();
        assert!(names.iter().all(|name| !name.starts_with("ANTHROPIC_")));
        assert!(!names.contains(&"CLAUDE_CODE_SUBAGENT_MODEL"));
    }

    #[test]
    fn an_antigravity_pod_talks_to_the_google_route_of_the_proxy() {
        let job = build_job(&antigravity_config(), &antigravity_spec(), Some(&proxy())).unwrap();
        assert_eq!(
            env(&job, "GOOGLE_GEMINI_BASE_URL").value.as_deref(),
            Some("http://capitainerie-egress-proxy.capitainerie.svc:4130/google")
        );
        assert_eq!(
            env(&job, "AGY_CLI_DISABLE_AUTO_UPDATE").value.as_deref(),
            Some("true")
        );
        let key = env(&job, "GEMINI_API_KEY");
        assert_eq!(key.value, None);
        assert_eq!(
            key.value_from
                .as_ref()
                .and_then(|source| source.secret_key_ref.as_ref())
                .map(|key| key.key.as_str()),
            Some(TOKEN_KEY)
        );
        let names: Vec<_> = pod(&job).containers[0]
            .env
            .as_ref()
            .unwrap()
            .iter()
            .map(|var| var.name.as_str())
            .collect();
        assert!(names.iter().all(|name| !name.starts_with("ANTHROPIC_")));
        assert!(names.iter().all(|name| !name.starts_with("CODEX_")));
    }

    #[test]
    fn a_vibe_pod_talks_to_the_mistral_route_of_the_proxy_with_the_wire_model() {
        let job = build_job(&vibe_config(), &vibe_spec(), Some(&proxy())).unwrap();
        assert_eq!(
            env(&job, "CAPITAINERIE_MISTRAL_BASE_URL").value.as_deref(),
            Some("http://capitainerie-egress-proxy.capitainerie.svc:4130/mistral")
        );
        assert_eq!(
            env(&job, "VIBE_HOME").value.as_deref(),
            Some("/home/executor/.vibe")
        );
        assert_eq!(
            env(&job, "MODEL").value.as_deref(),
            Some("mistral-vibe-cli-latest")
        );
        let key = env(&job, "MISTRAL_API_KEY");
        assert_eq!(key.value, None);
        assert_eq!(
            key.value_from
                .as_ref()
                .and_then(|source| source.secret_key_ref.as_ref())
                .map(|key| key.key.as_str()),
            Some(TOKEN_KEY)
        );
        let names: Vec<_> = pod(&job).containers[0]
            .env
            .as_ref()
            .unwrap()
            .iter()
            .map(|var| var.name.as_str())
            .collect();
        assert!(names.iter().all(|name| !name.starts_with("ANTHROPIC_")));
        assert!(names.iter().all(|name| !name.starts_with("CODEX_")));
        assert!(!names.contains(&"GEMINI_API_KEY"));
    }

    #[test]
    fn the_image_follows_the_harness() {
        let codex = build_job(&codex_config(), &codex_spec(), Some(&proxy())).unwrap();
        assert_eq!(
            pod(&codex).containers[0].image.as_deref(),
            Some("capitainerie-executor-codex:1")
        );
        let antigravity =
            build_job(&antigravity_config(), &antigravity_spec(), Some(&proxy())).unwrap();
        assert_eq!(
            pod(&antigravity).containers[0].image.as_deref(),
            Some("capitainerie-executor-antigravity:1")
        );
        let vibe = build_job(&vibe_config(), &vibe_spec(), Some(&proxy())).unwrap();
        assert_eq!(
            pod(&vibe).containers[0].image.as_deref(),
            Some("capitainerie-executor-mistral-vibe:1")
        );
        let claude = build_job(&codex_config(), &spec(), Some(&proxy())).unwrap();
        assert_eq!(
            pod(&claude).containers[0].image.as_deref(),
            Some("capitainerie-executor:1")
        );
        assert_eq!(
            env(&claude, "CAPITAINERIE_HARNESS").value.as_deref(),
            Some("claude-code")
        );
    }

    #[test]
    fn a_codex_execution_without_a_codex_image_fails_before_any_job() {
        let error = build_job(&config(), &codex_spec(), Some(&proxy())).unwrap_err();
        assert!(matches!(error, RunnerError::NoHarnessImage("codex")));
        assert!(error.to_string().contains("codex"));
    }

    #[test]
    fn an_antigravity_execution_without_an_antigravity_image_fails_before_any_job() {
        let error = build_job(&config(), &antigravity_spec(), Some(&proxy())).unwrap_err();
        assert!(matches!(error, RunnerError::NoHarnessImage("antigravity")));
        assert!(error.to_string().contains("antigravity"));
    }

    #[test]
    fn a_vibe_execution_without_a_vibe_image_fails_before_any_job() {
        let error = build_job(&config(), &vibe_spec(), Some(&proxy())).unwrap_err();
        assert!(matches!(error, RunnerError::NoHarnessImage("mistral-vibe")));
        assert!(error.to_string().contains("mistral-vibe"));
    }
}
