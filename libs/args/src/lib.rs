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
    #[arg(long = "delegation-key", env = "DELEGATION_KEY")]
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
