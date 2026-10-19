pub mod ports;

use chrono::{DateTime, Utc};

use crate::domain::agent::SecretToken;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegatedToken {
    pub token: SecretToken,
    pub expires_at: DateTime<Utc>,
}

/// The two hops that let an agent call the Kanban MCP server for a member,
/// inside one request: the member's token is exchanged for one addressed to
/// the agent, which the agent turns into a delegated token (`act` = agent).
/// Neither token is stored.
pub async fn delegate_to_mcp<X: ports::TokenExchanger>(
    exchanger: &X,
    issuer: &str,
    user_token: &SecretToken,
    agent_client_id: &str,
) -> Result<DelegatedToken, capitainerie_common::CoreError> {
    let for_agent = exchanger
        .for_agent(issuer, user_token, agent_client_id)
        .await?;
    exchanger
        .for_mcp(issuer, &for_agent.token, agent_client_id)
        .await
}

#[cfg(test)]
mod tests {
    use super::{ports::MockTokenExchanger, *};

    fn token(value: &str) -> DelegatedToken {
        DelegatedToken {
            token: SecretToken::new(value),
            expires_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn the_member_token_goes_to_the_agent_and_only_the_agent_token_goes_on() {
        let mut exchanger = MockTokenExchanger::new();
        exchanger
            .expect_for_agent()
            .withf(|issuer, user, agent| {
                issuer == "http://idp/realms/a" && user.expose() == "member" && agent == "captain"
            })
            .returning(|_, _, _| Box::pin(async { Ok(token("for-agent")) }));
        exchanger
            .expect_for_mcp()
            .withf(|_, subject, agent| subject.expose() == "for-agent" && agent == "captain")
            .returning(|_, _, _| Box::pin(async { Ok(token("for-mcp")) }));

        let delegated = delegate_to_mcp(
            &exchanger,
            "http://idp/realms/a",
            &SecretToken::new("member"),
            "captain",
        )
        .await
        .unwrap();

        assert_eq!(delegated.token.expose(), "for-mcp");
    }
}
