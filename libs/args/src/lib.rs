use clap::Args;

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

#[derive(Debug, Clone, Args)]
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

#[derive(Debug, Clone, Args)]
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

#[derive(Debug, Clone, Args)]
pub struct SecretsArgs {
    /// Base64 of 32 random bytes, distinct from `DELEGATION_KEY`; seals the
    /// providers' API keys at rest. `openssl rand -base64 32`.
    #[arg(id = "secrets_key", long = "secrets-key", env = "SECRETS_KEY")]
    pub key: Option<String>,
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
pub struct GithubAppArgs {
    #[arg(id = "github_app_id", long = "github-app-id", env = "GITHUB_APP_ID")]
    pub app_id: Option<u64>,

    /// PEM of the App's private key (RS256).
    #[arg(
        id = "github_app_private_key",
        long = "github-app-private-key",
        env = "GITHUB_APP_PRIVATE_KEY"
    )]
    pub private_key: Option<String>,

    /// The App's URL name, for `https://github.com/apps/<slug>/installations/new`.
    #[arg(
        id = "github_app_slug",
        long = "github-app-slug",
        env = "GITHUB_APP_SLUG"
    )]
    pub slug: Option<String>,

    #[arg(
        id = "github_api_base",
        long = "github-api-base",
        env = "GITHUB_API_BASE",
        default_value = "https://api.github.com"
    )]
    pub api_base: String,

    /// OAuth client id of the App: proves the connecting admin can access the installation.
    #[arg(
        id = "github_app_client_id",
        long = "github-app-client-id",
        env = "GITHUB_APP_CLIENT_ID"
    )]
    pub client_id: Option<String>,

    #[arg(
        id = "github_app_client_secret",
        long = "github-app-client-secret",
        env = "GITHUB_APP_CLIENT_SECRET"
    )]
    pub client_secret: Option<String>,

    #[arg(
        id = "github_oauth_base",
        long = "github-oauth-base",
        env = "GITHUB_OAUTH_BASE",
        default_value = "https://github.com"
    )]
    pub oauth_base: String,
}

impl GithubAppArgs {
    pub fn app_configured(&self) -> bool {
        self.app_id.is_some()
            && self.private_key.is_some()
            && self.client_id.is_some()
            && self.client_secret.is_some()
    }
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
    fn allowed_origins_split_on_commas() {
        let cli = Cli::parse_from(["bin", "--allowed-origins", "http://a,http://b"]);
        assert_eq!(cli.server.allowed_origins, vec!["http://a", "http://b"]);
    }
}
