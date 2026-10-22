use std::fmt;

use clap::Args;

fn masked(value: &Option<String>) -> &'static str {
    if value.is_some() { "***" } else { "unset" }
}

#[derive(Debug, Clone, Args)]
pub struct ServerArgs {
    #[arg(long = "server-host", env = "SERVER_HOST", default_value = "0.0.0.0")]
    pub host: String,

    #[arg(long = "server-port", env = "SERVER_PORT", default_value_t = 4100)]
    pub port: u16,

    #[arg(
        long = "allowed-origins",
        env = "ALLOWED_ORIGINS",
        value_delimiter = ',',
        default_value = "http://localhost:4080"
    )]
    pub allowed_origins: Vec<String>,
}

#[derive(Clone, Args)]
pub struct DatabaseArgs {
    #[arg(long = "database-url", env = "DATABASE_URL")]
    pub url: String,

    #[arg(
        long = "database-max-connections",
        env = "DATABASE_MAX_CONNECTIONS",
        default_value_t = 10
    )]
    pub max_connections: u32,
}

impl fmt::Debug for DatabaseArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseArgs")
            .field("url", &"***")
            .field("max_connections", &self.max_connections)
            .finish()
    }
}

#[derive(Debug, Clone, Args)]
pub struct AuthArgs {
    /// Client ids whose tokens the API accepts (`azp`). FerrisKey tokens carry
    /// no API audience, so the client is what scopes a token to this API.
    #[arg(
        long = "auth-accepted-clients",
        env = "AUTH_ACCEPTED_CLIENTS",
        value_delimiter = ',',
        default_value = "capitainerie-console"
    )]
    pub accepted_clients: Vec<String>,

    /// Required `aud`: the API's resource indicator (RFC 8707), which the
    /// console requests at login. Unset, the audience is not checked.
    #[arg(long = "auth-audience", env = "AUTH_AUDIENCE")]
    pub audience: Option<String>,
}

#[derive(Clone, Args)]
pub struct DelegationArgs {
    /// Base64 of 32 random bytes; seals the tokens agents act with between
    /// submission and execution. `openssl rand -base64 32`.
    #[arg(id = "delegation_key", long = "delegation-key", env = "DELEGATION_KEY")]
    pub key: Option<String>,

    /// Client ids of the agents a run acts through.
    #[arg(
        long = "pipeline-agents",
        env = "PIPELINE_AGENTS",
        value_delimiter = ',',
        default_value = "capitainerie-agent-writer"
    )]
    pub agents: Vec<String>,
}

impl fmt::Debug for DelegationArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DelegationArgs")
            .field("key", &masked(&self.key))
            .field("agents", &self.agents)
            .finish()
    }
}

#[derive(Clone, Args)]
pub struct SecretsArgs {
    /// Base64 of 32 random bytes, distinct from `DELEGATION_KEY`; seals the
    /// providers' API keys at rest. `openssl rand -base64 32`.
    #[arg(id = "secrets_key", long = "secrets-key", env = "SECRETS_KEY")]
    pub key: Option<String>,
}

impl fmt::Debug for SecretsArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretsArgs")
            .field("key", &masked(&self.key))
            .finish()
    }
}

#[derive(Debug, Clone, Args)]
pub struct LogArgs {
    #[arg(long = "log-filter", env = "LOG_FILTER", default_value = "info")]
    pub filter: String,

    #[arg(long = "log-json", env = "LOG_JSON", default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ObservabilityArgs {
    #[arg(long = "otlp-endpoint", env = "OTLP_ENDPOINT")]
    pub otlp_endpoint: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct GithubArgs {
    #[arg(
        id = "github_api_base",
        long = "github-api-base",
        env = "GITHUB_API_BASE",
        default_value = "https://api.github.com"
    )]
    pub api_base: String,

    #[arg(
        id = "github_web_base",
        long = "github-web-base",
        env = "GITHUB_WEB_BASE",
        default_value = "https://github.com"
    )]
    pub web_base: String,
}

#[derive(Debug, Clone, Args)]
pub struct IntegrationsArgs {
    #[arg(
        id = "integrations_allow_insecure_http",
        long = "integrations-allow-insecure-http",
        env = "INTEGRATIONS_ALLOW_INSECURE_HTTP",
        default_value_t = false
    )]
    pub allow_insecure_http: bool,

    #[arg(
        id = "integrations_allow_private_networks",
        long = "integrations-allow-private-networks",
        env = "INTEGRATIONS_ALLOW_PRIVATE_NETWORKS",
        default_value_t = false
    )]
    pub allow_private_networks: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ExecutorArgs {
    /// Kubernetes namespace the task jobs run in. Unset, executions are off.
    #[arg(
        id = "executor_namespace",
        long = "executor-namespace",
        env = "EXECUTOR_NAMESPACE"
    )]
    pub namespace: Option<String>,

    #[arg(id = "executor_image", long = "executor-image", env = "EXECUTOR_IMAGE")]
    pub image: Option<String>,

    /// Base URL of the credentials proxy as the task pods reach it.
    #[arg(
        id = "executor_proxy_url",
        long = "executor-proxy-url",
        env = "EXECUTOR_PROXY_URL"
    )]
    pub proxy_url: Option<String>,

    #[arg(
        id = "executor_max_turns",
        long = "executor-max-turns",
        env = "EXECUTOR_MAX_TURNS",
        default_value_t = 50
    )]
    pub max_turns: u32,

    #[arg(
        id = "executor_deadline_secs",
        long = "executor-deadline-secs",
        env = "EXECUTOR_DEADLINE_SECS",
        default_value_t = 1800
    )]
    pub deadline_secs: u64,

    /// How many card executions the worker follows at once, apart from the
    /// other runs: each one holds a Kubernetes Job and a log stream.
    #[arg(
        id = "executor_concurrency",
        long = "executor-concurrency",
        env = "EXECUTOR_CONCURRENCY",
        default_value_t = 2
    )]
    pub concurrency: usize,
}

#[derive(Debug, Clone, Args)]
pub struct EgressProxyArgs {
    #[arg(
        id = "egress_proxy_host",
        long = "egress-proxy-host",
        env = "EGRESS_PROXY_HOST",
        default_value = "0.0.0.0"
    )]
    pub host: String,

    #[arg(
        id = "egress_proxy_port",
        long = "egress-proxy-port",
        env = "EGRESS_PROXY_PORT",
        default_value_t = 4130
    )]
    pub port: u16,

    #[arg(
        id = "egress_anthropic_base",
        long = "egress-anthropic-base",
        env = "EGRESS_ANTHROPIC_BASE",
        default_value = "https://api.anthropic.com"
    )]
    pub anthropic_base: String,

    #[arg(
        id = "egress_github_base",
        long = "egress-github-base",
        env = "EGRESS_GITHUB_BASE",
        default_value = "https://github.com"
    )]
    pub github_base: String,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        server: ServerArgs,
        #[command(flatten)]
        log: LogArgs,
        #[command(flatten)]
        observability: ObservabilityArgs,
    }

    #[test]
    fn defaults_apply_without_flags() {
        let cli = Cli::parse_from(["bin"]);
        assert_eq!(cli.server.port, 4100);
        assert_eq!(cli.server.allowed_origins, vec!["http://localhost:4080"]);
        assert_eq!(cli.log.filter, "info");
        assert!(cli.observability.otlp_endpoint.is_none());
    }

    #[test]
    fn debug_output_never_shows_a_secret() {
        let shown = format!(
            "{:?} {:?} {:?}",
            DelegationArgs {
                key: Some("delegation-leak".into()),
                agents: vec![]
            },
            SecretsArgs {
                key: Some("secrets-leak".into())
            },
            DatabaseArgs {
                url: "postgres://u:db-leak@h/d".into(),
                max_connections: 1
            },
        );
        for leak in ["delegation-leak", "secrets-leak", "db-leak"] {
            assert!(!shown.contains(leak), "{leak} in {shown}");
        }
    }

    #[derive(Parser)]
    struct GithubCli {
        #[command(flatten)]
        github: GithubArgs,
    }

    #[test]
    fn github_hosts_default_to_the_public_service() {
        let github = GithubCli::parse_from(["bin"]).github;
        assert_eq!(github.api_base, "https://api.github.com");
        assert_eq!(github.web_base, "https://github.com");
    }

    #[derive(Parser)]
    struct IntegrationsCli {
        #[command(flatten)]
        integrations: IntegrationsArgs,
    }

    #[test]
    fn integrations_refuse_plain_http_and_private_networks_by_default() {
        let integrations = IntegrationsCli::parse_from(["bin"]).integrations;
        assert!(!integrations.allow_insecure_http);
        assert!(!integrations.allow_private_networks);
    }

    #[test]
    fn each_integrations_guard_is_lifted_on_its_own_flag() {
        let insecure =
            IntegrationsCli::parse_from(["bin", "--integrations-allow-insecure-http"]).integrations;
        assert!(insecure.allow_insecure_http);
        assert!(!insecure.allow_private_networks);
        let private = IntegrationsCli::parse_from(["bin", "--integrations-allow-private-networks"])
            .integrations;
        assert!(!private.allow_insecure_http);
        assert!(private.allow_private_networks);
    }

    #[derive(Parser)]
    struct ExecutorCli {
        #[command(flatten)]
        executor: ExecutorArgs,
    }

    #[test]
    fn executions_have_their_own_concurrency_defaulting_to_two() {
        assert_eq!(ExecutorCli::parse_from(["bin"]).executor.concurrency, 2);
        let cli = ExecutorCli::parse_from(["bin", "--executor-concurrency", "5"]);
        assert_eq!(cli.executor.concurrency, 5);
    }

    #[test]
    fn allowed_origins_split_on_commas() {
        let cli = Cli::parse_from(["bin", "--allowed-origins", "http://a,http://b"]);
        assert_eq!(cli.server.allowed_origins, vec!["http://a", "http://b"]);
    }
}
