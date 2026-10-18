pub mod agents;
pub mod costs;
pub mod state;

use axum::Router;
use axum_extra::routing::RouterExt;
use capitainerie_analytics::Analytics;
use capitainerie_handlers::{ApiError, Caller};

use crate::state::AnalyticsState;

pub const TAG: &str = "analytics";

pub fn router<S, A>(analytics: A) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    A: Analytics + 'static,
{
    Router::<AnalyticsState<A>>::new()
        .typed_get(costs::handler::<A>)
        .typed_get(agents::handler::<A>)
        .with_state(AnalyticsState::new(analytics))
}

pub(crate) fn human_only(caller: &Caller) -> Result<(), ApiError> {
    match caller.identity.actor {
        Some(_) => Err(ApiError::Forbidden(Some(
            "analytics are reserved to human callers".to_owned(),
        ))),
        None => Ok(()),
    }
}

pub(crate) fn internal(error: capitainerie_analytics::AnalyticsError) -> ApiError {
    ApiError::Internal(error.to_string())
}

#[cfg(test)]
mod tests {
    use axum::{
        Extension, Json,
        extract::{Query, State},
    };
    use capitainerie_analytics::{
        AgentRow, AgentsQuery, Analytics, AnalyticsError, CostDimension, CostQuery, CostRow,
    };
    use capitainerie_auth::{Actor, Identity};
    use capitainerie_common::OrganizationId;
    use capitainerie_core::{Organization, OrganizationSlug, domain::agent::SecretToken};
    use chrono::{DateTime, Duration, Utc};

    use super::*;
    use crate::{
        agents::{self, AgentsPath},
        costs::{self, CostDimensionParam, CostsPath, CostsQuery, window},
        state::AnalyticsState,
    };

    struct Fake;

    impl Analytics for Fake {
        async fn costs(&self, query: &CostQuery) -> Result<Vec<CostRow>, AnalyticsError> {
            let key = match query.by {
                CostDimension::Agent => "extractor",
                CostDimension::User => "alice",
                CostDimension::Model => "qwen",
                CostDimension::Day => "2026-10-10",
            };
            Ok(vec![
                CostRow {
                    key: key.into(),
                    runs: 2,
                    input_tokens: 100,
                    output_tokens: 10,
                    cost_micros: 500,
                    latency_ms_p50: Some(12.0),
                },
                CostRow {
                    key: "other".into(),
                    runs: 1,
                    input_tokens: 50,
                    output_tokens: 5,
                    cost_micros: 250,
                    latency_ms_p50: None,
                },
            ])
        }

        async fn agents(&self, _: &AgentsQuery) -> Result<Vec<AgentRow>, AnalyticsError> {
            Ok(vec![AgentRow {
                agent: "extractor".into(),
                runs: 3,
                last_seen: Utc::now(),
                input_tokens: 1,
                output_tokens: 2,
                cost_micros: 3,
            }])
        }
    }

    fn caller(actor: Option<Actor>) -> Caller {
        Caller {
            identity: Identity {
                subject: "user-1".into(),
                issuer: "http://issuer".into(),
                client_id: "capitainerie-console".into(),
                username: None,
                email: None,
                display_name: None,
                actor,
                roles: Vec::new(),
            },
            organization: Organization {
                id: OrganizationId::new(),
                slug: OrganizationSlug::try_from("acme".to_owned()).unwrap(),
                name: "Acme".into(),
                issuer_url: "http://issuer".into(),
                created_at: Utc::now(),
            },
            token: SecretToken::new("token"),
        }
    }

    fn agent_actor() -> Actor {
        Actor {
            sub: "agent".into(),
            client_id: None,
            act: None,
        }
    }

    fn state() -> State<AnalyticsState<Fake>> {
        State(AnalyticsState::new(Fake))
    }

    fn query(by: CostDimensionParam) -> Query<CostsQuery> {
        Query(CostsQuery {
            by,
            from: None,
            to: None,
        })
    }

    #[tokio::test]
    async fn costs_return_rows_and_their_total() {
        let Json(response) = costs::handler(
            CostsPath,
            state(),
            Extension(caller(None)),
            query(CostDimensionParam::User),
        )
        .await
        .unwrap();

        assert_eq!(response.rows[0].key, "alice");
        assert_eq!(response.total.cost_micros, 750);
        assert_eq!(response.total.input_tokens, 150);
        assert_eq!(response.total.output_tokens, 15);
    }

    #[tokio::test]
    async fn an_agent_caller_is_forbidden() {
        let outcome = costs::handler(
            CostsPath,
            state(),
            Extension(caller(Some(agent_actor()))),
            query(CostDimensionParam::Agent),
        )
        .await;
        assert!(matches!(outcome, Err(ApiError::Forbidden(_))));

        let outcome =
            agents::handler(AgentsPath, state(), Extension(caller(Some(agent_actor())))).await;
        assert!(matches!(outcome, Err(ApiError::Forbidden(_))));
    }

    #[tokio::test]
    async fn agents_list_the_activity() {
        let Json(agents) = agents::handler(AgentsPath, state(), Extension(caller(None)))
            .await
            .unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].runs, 3);
    }

    #[test]
    fn the_window_defaults_to_thirty_days() {
        let now = Utc::now();
        let (from, to) = window(None, None, now).unwrap();
        assert_eq!(to, now);
        assert_eq!(from, now - Duration::days(30));
    }

    #[test]
    fn an_inverted_window_is_refused() {
        let now = Utc::now();
        let earlier: DateTime<Utc> = now - Duration::days(1);
        assert!(matches!(
            window(Some(now), Some(earlier), now),
            Err(ApiError::BadRequest(_))
        ));
    }

    #[test]
    fn the_dimension_is_parsed_from_lowercase_words() {
        for (word, expected) in [
            ("agent", CostDimensionParam::Agent),
            ("user", CostDimensionParam::User),
            ("model", CostDimensionParam::Model),
            ("day", CostDimensionParam::Day),
        ] {
            let parsed: CostsQuery =
                serde_json::from_value(serde_json::json!({ "by": word })).unwrap();
            assert_eq!(parsed.by, expected);
        }
    }
}
