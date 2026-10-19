use capitainerie_handlers::{errors::ApiErrorResponse, health};
use capitainerie_handlers_analytics::{
    self as analytics,
    agents::AgentResponse,
    costs::{CostDimensionParam, CostRowResponse, CostTotals, CostsResponse},
};
use capitainerie_handlers_cards::{
    self as cards,
    comment_card::CommentCardRequest,
    execute_card::{ExecuteCardRequest, ExecuteCardResponse},
    list_cards::CardListResponse,
    move_card::MoveCardRequest,
    response::{CardDetailResponse, CardResponse, ColumnDto, CommentResponse, ProvenanceResponse},
};
use capitainerie_handlers_identity::{
    self as identity,
    me::{MeOrganization, MeResponse},
};
use capitainerie_handlers_integrations::{
    self as integrations, ConnectGithubRequest, CreateGithubAppRequest, CreateManifestRequest,
    GithubAppResponse, GithubInstallationResponse, GithubIntegrationResponse, InstallUrlResponse,
    ManifestOwner, ManifestResponse, RepositoryListResponse, RepositoryResponse,
};
use capitainerie_handlers_providers::{
    self as providers,
    dto::{
        CapabilitiesResponse, ModelAssignmentsDto, ModelAssignmentsResponse, ModelCatalogResponse,
        ModelRefDto, ModelResponse, PriceResponse, ProviderErrorResponse, ProviderKindDto,
        ProviderListResponse, ProviderResponse,
    },
    put_provider::PutProviderRequest,
    test_provider::TestProviderResponse,
};
use capitainerie_handlers_runs::{
    self as runs,
    list_runs::RunListResponse,
    response::{RunResponse, RunStatusResponse},
    submit_transcript::{SubmitTranscriptRequest, SubmitTranscriptResponse},
    timeline::{
        DecisionResponse, RunTimelineResponse, StepResponse, StepStatusResponse, ToolCallResponse,
        UsageTotals,
    },
};
use utoipa::{
    Modify, OpenApi,
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
};

#[derive(OpenApi)]
#[openapi(
    info(title = "Capitainerie API"),
    paths(
        health::live,
        health::ready,
        identity::me::handler,
        runs::submit_transcript::handler,
        runs::list_runs::handler,
        runs::get_run::handler,
        runs::timeline::handler,
        cards::list_cards::handler,
        cards::get_card::handler,
        cards::move_card::handler,
        cards::comment_card::handler,
        cards::execute_card::handler,
        analytics::costs::handler,
        analytics::agents::handler,
        providers::list_providers::handler,
        providers::put_provider::handler,
        providers::delete_provider::handler,
        providers::test_provider::handler,
        providers::list_models::handler,
        providers::get_assignments::handler,
        providers::put_assignments::handler,
        integrations::get_integration::handler,
        integrations::disconnect::handler,
        integrations::install_url::handler,
        integrations::callback::handler,
        integrations::create_manifest::handler,
        integrations::create_app::handler,
        integrations::delete_app::handler,
        integrations::list_repositories::handler,
    ),
    components(schemas(
        ApiErrorResponse,
        MeResponse,
        MeOrganization,
        SubmitTranscriptRequest,
        SubmitTranscriptResponse,
        RunListResponse,
        RunResponse,
        RunStatusResponse,
        RunTimelineResponse,
        StepResponse,
        StepStatusResponse,
        ToolCallResponse,
        DecisionResponse,
        UsageTotals,
        CardListResponse,
        CardResponse,
        ColumnDto,
        ProvenanceResponse,
        MoveCardRequest,
        CardDetailResponse,
        CommentResponse,
        CommentCardRequest,
        ExecuteCardRequest,
        ExecuteCardResponse,
        CostsResponse,
        CostRowResponse,
        CostTotals,
        CostDimensionParam,
        AgentResponse,
        ProviderKindDto,
        ProviderResponse,
        ProviderListResponse,
        PutProviderRequest,
        TestProviderResponse,
        ModelResponse,
        CapabilitiesResponse,
        PriceResponse,
        ProviderErrorResponse,
        ModelCatalogResponse,
        ModelRefDto,
        ModelAssignmentsDto,
        ModelAssignmentsResponse,
        GithubIntegrationResponse,
        GithubAppResponse,
        GithubInstallationResponse,
        CreateManifestRequest,
        ManifestOwner,
        ManifestResponse,
        CreateGithubAppRequest,
        InstallUrlResponse,
        ConnectGithubRequest,
        RepositoryListResponse,
        RepositoryResponse,
    )),
    modifiers(&BearerAuth),
    tags(
        (name = health::TAG, description = "Liveness and readiness probes"),
        (name = identity::TAG, description = "The authenticated caller"),
        (name = runs::TAG, description = "Transcripts and the runs processing them"),
        (name = cards::TAG, description = "The Kanban board and its human validation"),
        (name = analytics::TAG, description = "Costs and activity of the agents"),
        (name = providers::TAG, description = "Model providers, their catalogs and the model of each agent role"),
        (name = integrations::TAG, description = "The GitHub App each organization creates, and its installation, that let Claude Code work on a repository"),
    )
)]
pub struct ApiDoc;

struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_operation_has_an_operation_id() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = spec["paths"].as_object().unwrap();
        for (path, operations) in paths {
            for (method, operation) in operations.as_object().unwrap() {
                assert!(
                    operation["operationId"].is_string(),
                    "{method} {path} lacks an operationId"
                );
            }
        }
    }

    #[test]
    fn path_parameters_appear_in_their_path() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        for (path, operations) in spec["paths"].as_object().unwrap() {
            for (method, operation) in operations.as_object().unwrap() {
                let Some(parameters) = operation["parameters"].as_array() else {
                    continue;
                };
                for parameter in parameters.iter().filter(|p| p["in"] == "path") {
                    let name = parameter["name"].as_str().unwrap();
                    assert!(
                        path.contains(&format!("{{{name}}}")),
                        "{method} {path} declares path parameter {name} absent from the path"
                    );
                }
            }
        }
    }

    #[test]
    fn analytics_endpoints_document_their_query_parameters() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let costs = &spec["paths"]["/api/v1/analytics/costs"]["get"];
        let names: Vec<_> = costs["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["in"] == "query")
            .map(|p| p["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["by", "from", "to"]);
        assert!(spec["paths"]["/api/v1/agents"]["get"].is_object());
    }

    #[test]
    fn the_card_panel_endpoints_are_documented() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        assert!(paths["/api/v1/cards/{card_id}"]["get"].is_object());
        assert!(paths["/api/v1/cards/{card_id}/comments"]["post"].is_object());
        let detail = &spec["components"]["schemas"]["CardDetailResponse"]["properties"];
        for field in ["card", "comments", "runs"] {
            assert!(detail[field].is_object(), "{field}");
        }
        let comment = &spec["components"]["schemas"]["CommentResponse"]["properties"];
        for field in ["id", "body", "author_subject", "author_actor", "created_at"] {
            assert!(comment[field].is_object(), "{field}");
        }
    }

    #[test]
    fn the_provider_endpoints_are_documented_without_a_key_in_any_response() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        assert!(paths["/api/v1/providers"]["get"].is_object());
        assert!(paths["/api/v1/providers/{kind}"]["put"].is_object());
        assert!(paths["/api/v1/providers/{kind}"]["delete"].is_object());
        assert!(paths["/api/v1/providers/{kind}/test"]["post"].is_object());
        assert!(paths["/api/v1/models"]["get"].is_object());
        assert!(paths["/api/v1/model-assignments"]["get"].is_object());
        assert!(paths["/api/v1/model-assignments"]["put"].is_object());
        let provider = &spec["components"]["schemas"]["ProviderResponse"]["properties"];
        assert!(provider.get("api_key").is_none());
        assert!(provider["preview"].is_object());
    }

    #[test]
    fn the_execution_and_github_endpoints_are_documented() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        assert!(paths["/api/v1/cards/{card_id}/executions"]["post"].is_object());
        assert!(paths["/api/v1/integrations/github"]["get"].is_object());
        assert!(paths["/api/v1/integrations/github"]["delete"].is_object());
        assert!(paths["/api/v1/integrations/github/install"]["get"].is_object());
        assert!(paths["/api/v1/integrations/github/callback"]["post"].is_object());
        assert!(paths["/api/v1/integrations/github/repositories"]["get"].is_object());
        let schemas = &spec["components"]["schemas"];
        assert!(schemas["ExecuteCardResponse"]["properties"]["run_id"].is_object());
        assert!(schemas["GithubIntegrationResponse"]["properties"]["app"].is_object());
        assert!(schemas["GithubIntegrationResponse"]["properties"]["installation"].is_object());
        assert!(paths["/api/v1/integrations/github/app/manifest"]["post"].is_object());
        assert!(paths["/api/v1/integrations/github/app"]["post"].is_object());
        assert!(paths["/api/v1/integrations/github/app"]["delete"].is_object());
        assert!(schemas["ManifestResponse"]["properties"]["action_url"].is_object());
        let app = &schemas["GithubAppResponse"]["properties"];
        for secret in ["client_secret", "pem", "private_key", "webhook_secret"] {
            assert!(app.get(secret).is_none(), "{secret}");
        }
    }
}
