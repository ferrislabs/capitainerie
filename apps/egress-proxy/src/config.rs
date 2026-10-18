use capitainerie_args::{
    DatabaseArgs, EgressProxyArgs, GithubArgs, LogArgs, ObservabilityArgs, SecretsArgs,
};
use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(name = "capitainerie-egress-proxy", version)]
pub struct Args {
    #[command(flatten)]
    pub proxy: EgressProxyArgs,
    #[command(flatten)]
    pub github: GithubArgs,
    #[command(flatten)]
    pub secrets: SecretsArgs,
    #[command(flatten)]
    pub database: DatabaseArgs,
    #[command(flatten)]
    pub log: LogArgs,
    #[command(flatten)]
    pub observability: ObservabilityArgs,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_line_definition_is_valid() {
        <Args as clap::CommandFactory>::command().debug_assert();
    }

    #[test]
    fn defaults_apply() {
        let args = Args::parse_from(["bin", "--database-url", "postgres://x"]);
        assert_eq!(args.proxy.port, 4130);
        assert_eq!(args.proxy.anthropic_base, "https://api.anthropic.com");
        assert_eq!(args.proxy.github_base, "https://github.com");
    }
}
