use capitainerie_args::{DatabaseArgs, LogArgs, ObservabilityArgs};
use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(name = "capitainerie-mcp-kanban", version)]
pub struct Args {
    #[arg(long = "mcp-host", env = "MCP_HOST", default_value = "0.0.0.0")]
    pub host: String,
    #[arg(long = "mcp-port", env = "MCP_PORT", default_value_t = 4101)]
    pub port: u16,
    #[arg(
        long = "mcp-public-url",
        env = "MCP_PUBLIC_URL",
        default_value = "http://localhost:4101"
    )]
    pub public_url: String,
    #[arg(
        long = "mcp-audience",
        env = "MCP_AUDIENCE",
        default_value = "capitainerie-mcp"
    )]
    pub audience: String,
    #[arg(
        long = "mcp-accepted-agents",
        env = "MCP_ACCEPTED_AGENTS",
        value_delimiter = ',',
        default_value = "capitainerie-agent-writer"
    )]
    pub accepted_agents: Vec<String>,
    #[command(flatten)]
    pub database: DatabaseArgs,
    #[command(flatten)]
    pub log: LogArgs,
    #[command(flatten)]
    pub observability: ObservabilityArgs,
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_command_line_definition_is_valid() {
        <super::Args as clap::CommandFactory>::command().debug_assert();
    }

    use super::*;

    #[test]
    fn defaults_apply() {
        let args = Args::parse_from(["bin", "--database-url", "postgres://x"]);
        assert_eq!(args.port, 4101);
        assert_eq!(args.public_url, "http://localhost:4101");
        assert_eq!(args.audience, "capitainerie-mcp");
        assert_eq!(args.accepted_agents, vec!["capitainerie-agent-writer"]);
    }

    #[test]
    fn accepted_agents_split_on_commas() {
        let args = Args::parse_from([
            "bin",
            "--database-url",
            "postgres://x",
            "--mcp-accepted-agents",
            "a,b",
        ]);
        assert_eq!(args.accepted_agents, vec!["a", "b"]);
    }
}
